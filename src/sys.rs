// 文件路径：src/sys.rs
// 平台抽象层（跨平台重构）：
// 把 lstat/stat 元数据、uid/gid 名称查询、日期格式化、主机名、
// locale 设置、ACL/SELinux 等平台差异全部收敛到此模块——
// 各平台用 cfg 分支实现，对外提供统一签名，
// 其余模块一律通过本模块调用，不再散落 #[cfg]。

use crate::tree::{SIXMONTHS, StatFields};
#[cfg(unix)]
use crate::tree::PATH_MAX;
use std::io;

/* =====================================================================
 * 文件元数据（C: lstat/stat/readlink）
 * ===================================================================== */

// Unix：直接使用 std::os::unix::fs::MetadataExt（mode/uid/gid/dev/ino 齐全）
#[cfg(unix)]
fn metadata_to_fields(md: &std::fs::Metadata) -> StatFields {
    use std::os::unix::fs::MetadataExt;
    StatFields {
        mode: md.mode(),
        uid: md.uid(),
        gid: md.gid(),
        size: md.size() as i64,
        atime: md.atime(),
        ctime: md.ctime(),
        mtime: md.mtime(),
        dev: md.dev(),
        inode: md.ino(),
    }
}

#[cfg(unix)]
pub fn lstat_fields(path: &str) -> io::Result<StatFields> {
    let md = std::fs::symlink_metadata(path)?;
    Ok(metadata_to_fields(&md))
}

#[cfg(unix)]
pub fn stat_fields(path: &str) -> io::Result<StatFields> {
    let md = std::fs::metadata(path)?;
    Ok(metadata_to_fields(&md))
}

// Windows：std 的 MetadataExt 无 mode/uid/gid/dev/ino；
// mode 用文件属性粗略近似（含 reparse point 的 lstat 语义），
// dev/inode 通过 FFI 调用 GetFileInformationByHandle 获取
//（避免 nightly feature windows_by_handle，stable 工具链亦可编译）。
#[cfg(windows)]
fn metadata_to_fields(md: &std::fs::Metadata) -> StatFields {
    use std::os::windows::fs::MetadataExt;
    let size = md.file_size() as i64;
    let mtime = md
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let readonly = md.permissions().readonly();
    // Windows 的 lstat 语义：目录符号链接的 is_dir() 为 true，需用
    // FILE_ATTRIBUTE_REPARSE_POINT (0x400) 检测符号链接（近似 S_IFLNK）。
    let attrs = md.file_attributes();
    // 粗略近似：符号链接 0777、目录 0755、只读文件 0444、普通文件 0644
    let mode = if attrs & 0x400 != 0 {
        0o120777
    } else if md.is_dir() {
        0o40755
    } else if readonly {
        0o100444
    } else {
        0o100644
    };
    StatFields {
        mode,
        uid: 0,
        gid: 0,
        size,
        atime: mtime,
        ctime: mtime,
        mtime,
        dev: 0,   // 由 file_identity 覆盖
        inode: 0, // 由 file_identity 覆盖
    }
}

// Windows：dev/inode 通过 GetFileInformationByHandle 获取。
// File::open 跟随符号链接（等价于 stat 语义）；lstat 的基础字段
// 用 symlink_metadata（reparse 检测），inode 仍为目标的索引——
// 与 C 中 stat 跟随返回目标 st_ino 的循环检测语义一致。
// 字段名保持 Win32 命名（BY_HANDLE_FILE_INFORMATION 结构体）
#[cfg(windows)]
#[allow(non_snake_case)]
mod win_identity {
    use std::os::windows::io::AsRawHandle;

    #[repr(C)]
    struct FileTime {
        dwLowDateTime: u32,
        dwHighDateTime: u32,
    }

    #[repr(C)]
    struct ByHandleFileInformation {
        dwFileAttributes: u32,
        ftCreationTime: FileTime,
        ftLastAccessTime: FileTime,
        ftLastWriteTime: FileTime,
        dwVolumeSerialNumber: u32,
        nFileSizeHigh: u32,
        nFileSizeLow: u32,
        nNumberOfLinks: u32,
        nFileIndexHigh: u32,
        nFileIndexLow: u32,
    }

    extern "system" {
        fn GetFileInformationByHandle(
            hFile: *mut core::ffi::c_void,
            lpFileInformation: *mut ByHandleFileInformation,
        ) -> i32;
    }

    /// 返回 (dev, inode)：卷序列号与 64 位文件索引；失败时 None（调用方回退 0）。
    pub fn identity(path: &str) -> Option<(u64, u64)> {
        use std::os::windows::fs::OpenOptionsExt;
        // FILE_FLAG_BACKUP_SEMANTICS (0x02000000)：允许 CreateFile 打开目录
        //（Rust 的 File::open 对目录会失败）；打开句柄跟随符号链接（stat 语义）
        let file = std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(0x0200_0000)
            .open(path)
            .ok()?;
        let handle = file.as_raw_handle();
        // unsafe：调用 Win32 API GetFileInformationByHandle（std 无等价封装，
        // libc 亦未提供，故直接 FFI 声明）
        let mut info: ByHandleFileInformation = unsafe { std::mem::zeroed() };
        let ok = unsafe { GetFileInformationByHandle(handle, &mut info) };
        if ok == 0 {
            return None;
        }
        let dev = info.dwVolumeSerialNumber as u64;
        let inode = ((info.nFileIndexHigh as u64) << 32) | info.nFileIndexLow as u64;
        Some((dev, inode))
    }
}

#[cfg(windows)]
fn fields_for(path: &str, follow: bool) -> io::Result<StatFields> {
    let md = if follow {
        std::fs::metadata(path)?
    } else {
        std::fs::symlink_metadata(path)?
    };
    let mut f = metadata_to_fields(&md);
    // dev/inode 由句柄查询覆盖（跟随语义，见 win_identity 注释）
    if let Some((dev, inode)) = win_identity::identity(path) {
        f.dev = dev;
        f.inode = inode;
    }
    Ok(f)
}

#[cfg(windows)]
pub fn lstat_fields(path: &str) -> io::Result<StatFields> {
    fields_for(path, false)
}

#[cfg(windows)]
pub fn stat_fields(path: &str) -> io::Result<StatFields> {
    fields_for(path, true)
}

// C: readlink(path, buf, size) —— 读取符号链接目标（std 通用实现，无平台差异）
pub fn read_link(path: &str) -> io::Result<String> {
    std::fs::read_link(path).map(|p| p.to_string_lossy().into_owned())
}

/* =====================================================================
 * 用户/组名称（C: getpwuid/getgrgid）
 * ===================================================================== */

// C: getpwuid(uid) —— 从系统用户数据库查询 uid 对应的用户名
// 理由：std 无法查询系统用户数据库，必须使用 libc。
#[cfg(unix)]
pub fn uid_name(uid: u32) -> Option<String> {
    // unsafe：调用 C 库函数 getpwuid（libc 无安全封装），返回静态缓冲区中的 passwd 结构
    unsafe {
        let pw = libc::getpwuid(uid);
        if pw.is_null() {
            return None;
        }
        // C: ent->pw_name
        let name = std::ffi::CStr::from_ptr((*pw).pw_name);
        Some(name.to_string_lossy().into_owned())
    }
}

#[cfg(not(unix))]
pub fn uid_name(_uid: u32) -> Option<String> {
    // 非 Unix 平台（如 Windows）无 getpwuid 系统调用，返回 None，
    // 调用方回退到数字字符串（对应 C 的 snprintf(ubuf,"%d",uid) 分支）
    None
}

// C: getgrgid(gid) —— 从系统组数据库查询 gid 对应的组名
#[cfg(unix)]
pub fn gid_name(gid: u32) -> Option<String> {
    // unsafe：调用 C 库函数 getgrgid（libc 无安全封装）
    unsafe {
        let gr = libc::getgrgid(gid);
        if gr.is_null() {
            return None;
        }
        // C: ent->gr_name
        let name = std::ffi::CStr::from_ptr((*gr).gr_name);
        Some(name.to_string_lossy().into_owned())
    }
}

#[cfg(not(unix))]
pub fn gid_name(_gid: u32) -> Option<String> {
    None
}

/* =====================================================================
 * 时间格式（C: do_date，strftime）
 * ===================================================================== */

// === 原 C 函数：char *do_date(time_t t) ===
/// 格式化时间。默认格式按 6 个月窗口选择 "%b %e  %Y"（较远）或 "%b %e %R"（较近）；
/// 设置 --timefmt 时按自定义格式。
pub fn do_date(t: i64) -> String {
    #[cfg(unix)]
    // unsafe：调用 C 库函数 localtime_r/strftime（libc 无安全封装）
    unsafe {
        let mut tm: libc::tm = std::mem::zeroed();
        let tt: libc::time_t = t as libc::time_t;
        if libc::localtime_r(&tt, &mut tm).is_null() {
            return String::new();
        }
        // C: if (timefmt) 用 timefmt，否则按时间窗口选默认格式
        let fmt: &[u8] = if crate::globals::TIMEFMT.is_some() {
            crate::globals::TIMEFMT.unwrap().as_bytes()
        } else {
            // C: time_t c = time(0);
            let c = libc::time(std::ptr::null_mut());
            if t > c as i64 || (t + SIXMONTHS) < c as i64 {
                b"%b %e  %Y"
            } else {
                b"%b %e %R"
            }
        };
        let mut buf = vec![0u8; 256];
        let n = libc::strftime(
            buf.as_mut_ptr() as *mut libc::c_char,
            255,
            fmt.as_ptr() as *const libc::c_char,
            &tm,
        );
        buf.truncate(n);
        String::from_utf8_lossy(&buf).into_owned()
    }
    #[cfg(not(unix))]
    {
        // Windows 无 strftime/localtime_r：手写两种默认格式（UTC 近似，注释说明时区差异）。
        // 自定义 --timefmt 仅支持常见占位符替换。
        const MONTHS: &[&str] = &[
            "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
        ];
        // C: time_t c = time(0)
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        // 简化：按 UTC 计算（C 的 localtime 依赖本地时区，Windows 上注释说明）
        let days = t.div_euclid(86400);
        let secs = t.rem_euclid(86400);
        let (y, m, d) = civil_from_days(days);
        let hh = secs / 3600;
        let mm = (secs % 3600) / 60;
        // unsafe：读取全局 TIMEFMT
        if let Some(fmt) = unsafe { crate::globals::TIMEFMT } {
            // 简单 strftime 子集替换：%Y %y %m %d %e %H %M %S %b %B %%
            let mut out = String::new();
            let mut chars = fmt.chars().peekable();
            while let Some(c) = chars.next() {
                if c != '%' {
                    out.push(c);
                    continue;
                }
                match chars.next() {
                    Some('Y') => out.push_str(&format!("{}", y)),
                    Some('y') => out.push_str(&format!("{:02}", y % 100)),
                    Some('m') => out.push_str(&format!("{:02}", m)),
                    Some('d') => out.push_str(&format!("{:02}", d)),
                    Some('e') => out.push_str(&format!("{:2}", d)),
                    Some('H') => out.push_str(&format!("{:02}", hh)),
                    Some('M') => out.push_str(&format!("{:02}", mm)),
                    Some('S') => out.push_str(&format!("{:02}", secs % 60)),
                    Some('b') => out.push_str(MONTHS[(m - 1) as usize]),
                    Some('%') => out.push('%'),
                    Some(other) => {
                        out.push('%');
                        out.push(other);
                    }
                    None => out.push('%'),
                }
            }
            return out;
        }
        if t > now || (t + SIXMONTHS) < now {
            format!("{} {:2}  {}", MONTHS[(m - 1) as usize], d, y)
        } else {
            format!("{} {:2} {:02}:{:02}", MONTHS[(m - 1) as usize], d, hh, mm)
        }
    }
}

// 天数 → (年, 月, 日)（Howard Hinnant 的 civil_from_days 算法；Windows 手写日期用）
#[cfg(not(unix))]
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719468;
    let era = z.div_euclid(146097);
    let doe = z.rem_euclid(146097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

/* =====================================================================
 * locale（C: setlocale）
 * ===================================================================== */

// C: setlocale(LC_CTYPE, ""); setlocale(LC_COLLATE, "");
#[cfg(unix)]
pub fn set_locale_ctype_collate() {
    // unsafe：调用 C 库函数 setlocale（libc 无安全封装）
    unsafe {
        let empty = std::ffi::CString::new("").unwrap();
        libc::setlocale(libc::LC_CTYPE, empty.as_ptr());
        libc::setlocale(libc::LC_COLLATE, empty.as_ptr());
    }
}

#[cfg(not(unix))]
pub fn set_locale_ctype_collate() {
    // 非 Unix 平台无 locale 概念，空实现
}

// C: if (timefmt) setlocale(LC_TIME, "");
#[cfg(unix)]
pub fn set_locale_time() {
    // unsafe：调用 C 库函数 setlocale（libc 无安全封装）
    unsafe {
        let empty = std::ffi::CString::new("").unwrap();
        libc::setlocale(libc::LC_TIME, empty.as_ptr());
    }
}

#[cfg(not(unix))]
pub fn set_locale_time() {
    // 非 Unix 平台空实现
}

/* =====================================================================
 * 主机名（C: gethostname）
 * ===================================================================== */

// C: gethostname(xpattern, PATH_MAX)；失败时返回 None（调用方回退 "localhost"）
#[cfg(unix)]
pub fn get_hostname() -> Option<String> {
    let mut buf = vec![0u8; PATH_MAX];
    // unsafe：调用 C 库函数 gethostname（libc 无安全封装）
    let r = unsafe { libc::gethostname(buf.as_mut_ptr() as *mut libc::c_char, PATH_MAX) };
    if r < 0 {
        return None;
    }
    // unsafe：CStr::from_ptr 从 gethostname 写入的 C 缓冲区构造字符串
    let name = unsafe { std::ffi::CStr::from_ptr(buf.as_ptr() as *const libc::c_char) }
        .to_string_lossy()
        .into_owned();
    Some(name)
}

#[cfg(not(unix))]
pub fn get_hostname() -> Option<String> {
    // 非 Unix 平台无 gethostname：用 COMPUTERNAME 或 localhost
    Some(std::env::var("COMPUTERNAME").unwrap_or_else(|_| "localhost".to_string()))
}

/* =====================================================================
 * Linux 专属：ACL 与 SELinux（非 Linux 平台返回安全默认值）
 * ===================================================================== */

// === 原 C 函数：bool has_acl(const char *path) ===（仅 Linux）
/// 检测文件是否带有 POSIX ACL（通过 listxattr 检查 "system.posix_acl_access"）。
#[cfg(target_os = "linux")]
pub fn has_acl(path: &str) -> bool {
    // unsafe：调用 C 库函数 listxattr（libc 无安全封装）
    let c_path = std::ffi::CString::new(path).unwrap_or_default();
    let mut buf = vec![0u8; PATH_MAX];
    let n = unsafe {
        libc::listxattr(
            c_path.as_ptr(),
            buf.as_mut_ptr() as *mut libc::c_char,
            buf.len(),
        )
    };
    // C: ssize_t n = listxattr(path, buf, PATH_MAX); if (n <= 0) return false;
    if n <= 0 {
        return false;
    }
    // C: for(key=buf, i=0; i < n; i+=len+1) { len = strlen(key); if (!strcmp(key, "system.posix_acl_access")) return true; }
    let mut i = 0usize;
    while i < n as usize {
        // unsafe：从 listxattr 填充的缓冲区构造 C 字符串（buf 内容为 NUL 分隔的 xattr 名）
        let key = unsafe { std::ffi::CStr::from_ptr(buf.as_ptr().add(i) as *const libc::c_char) };
        let len = key.to_bytes().len();
        if key.to_bytes() == b"system.posix_acl_access" {
            return true;
        }
        i += len + 1;
    }
    false
}

#[cfg(not(target_os = "linux"))]
// 非 Linux 平台上 has_acl 无调用点（fillinfo 的 ACL 块为 cfg(linux)），保留统一签名
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub fn has_acl(_path: &str) -> bool {
    false
}

// === 原 C 函数：char *selinux_context(const char *path) ===（仅 Linux）
/// 读取文件的 SELinux 安全上下文（getxattr），并驻留到 strhash 表中。
/// C 中返回驻留字符串指针；Rust 返回克隆的 String（值等价）。
#[cfg(target_os = "linux")]
pub fn selinux_context(path: &str) -> String {
    // unsafe：调用 C 库函数 getxattr（libc 无安全封装）
    let c_path = std::ffi::CString::new(path).unwrap_or_default();
    let mut buf = vec![0u8; PATH_MAX];
    let len = unsafe {
        libc::getxattr(
            c_path.as_ptr(),
            b"security.selinux\0".as_ptr() as *const libc::c_char,
            buf.as_mut_ptr() as *mut libc::c_void,
            PATH_MAX - 1,
        )
    };
    // C: xpattern[len < 0 ? 0 : len] = '\0';
    let valid = if len < 0 { 0 } else { len as usize };
    buf.truncate(valid);
    // C: return strhash(xpattern);
    crate::hash::strhash(&String::from_utf8_lossy(&buf))
}

#[cfg(not(target_os = "linux"))]
// 非 Linux 平台上 selinux_context 无调用点（fillinfo 的 SELinux 块为 cfg(linux)），保留统一签名
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub fn selinux_context(_path: &str) -> String {
    String::new()
}
