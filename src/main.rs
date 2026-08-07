// 文件路径：src/main.rs
// 对应 C 源文件：tree.c
// 主程序入口。模块声明随翻译进度逐步添加。
//
// 翻译过程中临时允许 dead_code：尚未翻译完所有模块时，tree.rs 中已定义
// 但尚未被引用的类型/常量会触发该警告；全部模块翻译完成后移除本属性。
//
// 允许 static_mut_refs：本程序为单线程，所有对 static mut 全局变量的
// 访问均在 unsafe 块内并附中文注释，语义与 C 的全局变量一致；
// 直接对 static mut 取引用的方法调用（如 STATIC.take()）因此被允许。

#![allow(dead_code)]
#![allow(static_mut_refs)]

mod color;
mod filter;
mod globals;
mod hash;
mod info;
mod list;
mod strverscmp;
mod tree;
mod util;

// =====================================================================
// 以下两函数提前翻译自 tree.c（patmatch/cond_lower），
// 因为 filter.c 的 filtercheck() 依赖 patmatch，必须先于 filter.rs 可用。
// 它们属于 tree.c，翻译 main.rs 时整体归档于此。
// =====================================================================

// === 原 C 函数：static char cond_lower(char c) ===
/// 若开启 --ignore-case 则转为小写，否则原样返回
fn cond_lower(c: u8) -> u8 {
    // unsafe：读取全局选项 FLAG
    if unsafe { globals::FLAG }.ignorecase {
        c.to_ascii_lowercase()
    } else {
        c
    }
}

// === 原 C 函数：int patmatch(const char *buf, const char *pat, bool isdir) ===
/// 通配符匹配。返回 1 匹配、0 不匹配、-1 模式语法错误。
/// 支持 '?'、'*'、'**'、'[...]'、'\\' 转义、'|' 或（递归子模式）、
/// 以及 '/' 与 isdir 的特殊交互。
pub fn patmatch(buf: &[u8], pat: &[u8], isdir: bool) -> i32 {
    let mut match_: i32 = 1;
    // C: char m, pprev = 0;（记录上一个模式字符，供 ** 跨 '/' 匹配判断）
    let mut pprev: u8 = 0;

    // C: bar = strchr(pat, '|') —— 若存在 '|'，递归比较两个子模式
    if let Some(bar) = pat.iter().position(|&b| b == b'|') {
        // C: if (bar == pat || !bar[1]) return -1;
        if bar == 0 || bar + 1 >= pat.len() {
            return -1;
        }
        // C: *bar = '\0'; match = patmatch(buf, pat, isdir);
        //     if (!match) match = patmatch(buf, bar+1, isdir);
        match_ = patmatch(buf, &pat[..bar], isdir);
        if match_ == 0 {
            match_ = patmatch(buf, &pat[bar + 1..], isdir);
        }
        return match_;
    }

    // buf/pat 的当前读取位置（C 中用指针推进）
    let mut bi: usize = 0;
    let mut pi: usize = 0;

    // C: while(*pat && match)
    while pi < pat.len() && match_ != 0 {
        match pat[pi] {
            b'[' => {
                pi += 1;
                // C: if(*pat != '^') { n = 1; match = 0; } else { pat++; n = 0; }
                let n: i32;
                if pi >= pat.len() || pat[pi] != b'^' {
                    n = 1;
                    match_ = 0;
                } else {
                    pi += 1;
                    n = 0;
                }
                // C: while(*pat != ']') { ... }
                loop {
                    // C: 循环内 if(!*pat) return -1
                    if pi >= pat.len() {
                        return -1;
                    }
                    if pat[pi] == b']' {
                        break;
                    }
                    // C: if(*pat == '\\') pat++;
                    if pat[pi] == b'\\' {
                        pi += 1;
                    }
                    // C: if(!*pat) return -1;
                    if pi >= pat.len() {
                        return -1;
                    }
                    // C: if(pat[1] == '-') —— 范围匹配
                    if pi + 1 < pat.len() && pat[pi + 1] == b'-' {
                        // C: m = *pat; pat += 2;
                        let m = pat[pi];
                        pi += 2;
                        // C: if(*pat == '\\' && *pat) pat++;
                        if pi < pat.len() && pat[pi] == b'\\' {
                            pi += 1;
                        }
                        // C: if(cond_lower(*buf) >= cond_lower(m) && cond_lower(*buf) <= cond_lower(*pat))
                        let bc = if bi < buf.len() { cond_lower(buf[bi]) } else { 0 };
                        let ec = if pi < pat.len() { cond_lower(pat[pi]) } else { 0 };
                        if bc >= cond_lower(m) && bc <= ec {
                            match_ = n;
                        }
                        // C: if(!*pat) pat--;（范围终点越界时回退，使外层循环最终 return -1）
                        if pi >= pat.len() {
                            pi -= 1;
                        }
                    } else if bi < buf.len() && cond_lower(buf[bi]) == cond_lower(pat[pi]) {
                        // C: else if(cond_lower(*buf) == cond_lower(*pat)) match = n;
                        match_ = n;
                    }
                    // C: pat++;
                    pi += 1;
                }
                // C: buf++;
                bi += 1;
            }
            b'*' => {
                pi += 1;
                // C: if(!*pat) { int f = (strchr(buf, '/') == NULL); return f; }
                if pi >= pat.len() {
                    let f = !buf[bi..].contains(&b'/');
                    return f as i32;
                }
                match_ = 0;
                // C: if (*pat == '*') —— "支持 **（与 * 基本等价，但可跨 / 匹配）"
                if pat[pi] == b'*' {
                    pi += 1;
                    // C: if(!*pat) return 1;
                    if pi >= pat.len() {
                        return 1;
                    }
                    // C: while(*buf && !(match = patmatch(buf, pat, isdir)))
                    loop {
                        if bi >= buf.len() {
                            break;
                        }
                        match_ = patmatch(&buf[bi..], &pat[pi..], isdir);
                        if match_ != 0 {
                            break;
                        }
                        // C: if (pprev == '/' && *pat == '/' && *(pat+1) &&
                        //        (match = patmatch(buf, pat+1, isdir))) return match;
                        if pprev == b'/' && pat[pi] == b'/' && pi + 1 < pat.len() {
                            let m = patmatch(&buf[bi..], &pat[pi + 1..], isdir);
                            if m != 0 {
                                return m;
                            }
                        }
                        // C: buf++; while(*buf && *buf != '/') buf++;
                        bi += 1;
                        while bi < buf.len() && buf[bi] != b'/' {
                            bi += 1;
                        }
                    }
                } else {
                    // C: while(*buf && !(match = patmatch(buf++, pat, isdir)))
                    //       if (*buf == '/') break;
                    loop {
                        if bi >= buf.len() {
                            break;
                        }
                        match_ = patmatch(&buf[bi..], &pat[pi..], isdir);
                        bi += 1;
                        if match_ != 0 {
                            break;
                        }
                        if bi < buf.len() && buf[bi] == b'/' {
                            break;
                        }
                    }
                }
                // C: if (!match && (!*buf || *buf == '/')) match = patmatch(buf, pat, isdir);
                if match_ == 0 && (bi >= buf.len() || buf[bi] == b'/') {
                    match_ = patmatch(&buf[bi..], &pat[pi..], isdir);
                }
                return match_;
            }
            b'?' => {
                // C: if(!*buf) return 0; buf++;
                if bi >= buf.len() {
                    return 0;
                }
                bi += 1;
            }
            b'/' => {
                // C: if (!*(pat+1) && !*buf) return isdir;
                if pi + 1 >= pat.len() && bi >= buf.len() {
                    return isdir as i32;
                }
                // C: match = (*buf++ == *pat);
                match_ = if bi < buf.len() && buf[bi] == b'/' { 1 } else { 0 };
                bi += 1;
            }
            b'\\' => {
                // C: if(*pat) pat++;（跳过反斜杠，落到 default 比较转义字符）
                if pi < pat.len() {
                    pi += 1;
                }
                // C: default: match = (cond_lower(*buf++) == cond_lower(*pat));
                let bc = if bi < buf.len() { cond_lower(buf[bi]) } else { 0 };
                let pc = if pi < pat.len() { cond_lower(pat[pi]) } else { 0 };
                match_ = if bc == pc { 1 } else { 0 };
                bi += 1;
            }
            _ => {
                // C: default: match = (cond_lower(*buf++) == cond_lower(*pat));
                let bc = if bi < buf.len() { cond_lower(buf[bi]) } else { 0 };
                let pc = cond_lower(pat[pi]);
                match_ = if bc == pc { 1 } else { 0 };
                bi += 1;
            }
        }
        // C: pprev = *pat++;（default 分支 pat 在循环末尾推进）
        if pi < pat.len() {
            pprev = pat[pi];
        }
        pi += 1;
        // C: if(match<1) return match;
        if match_ < 1 {
            return match_;
        }
    }

    // C: if(!*buf) return match; return 0;
    if bi >= buf.len() {
        match_
    } else {
        0
    }
}

// =====================================================================
// 以下函数提前翻译自 tree.c（stat2info/free_dir/patinclude/patignore/
// getinfo/read_dir/push_files/setoutput 等），因为 list.c 的 emit_tree/
// listdir 依赖它们。它们属于 tree.c，翻译 main.rs 时整体归档于此。
// =====================================================================


use crate::filter::{
    filtercheck, gitignore_search, new_ignorefile, push_filterstack,
};
use crate::globals::{
    FLAG, IPATTERN, IPATTERNS, OUTFILE, PATTERN, PATTERNS,
};
use crate::info::{infocheck, new_infofile, push_infostack};
use crate::tree::{
    stat_fields, Info, StatFields, MINIT, S_IFDIR, S_IFIFO, S_IFLNK, S_IFMT, S_IFSOCK,
    S_IXGRP, S_IXOTH, S_IXUSR,
};
use crate::util::pathconcat;

// === 原 C 函数：struct _info *stat2info(const struct stat *st) ===
/// 由 stat 结果构造一个 Info。
/// C 中返回指向 static 缓冲的指针（每次调用覆盖同一块内存）；
/// Rust 返回新值，调用方随后立即使用，语义等价。
pub fn stat2info(st: &StatFields) -> Info {
    Info {
        linode: st.inode,
        ldev: st.dev,
        mode: st.mode,
        uid: st.uid,
        gid: st.gid,
        size: st.size,
        atime: st.atime,
        ctime: st.ctime,
        mtime: st.mtime,
        // C: info.isdir  = ((st->st_mode & S_IFMT) == S_IFDIR);
        isdir: (st.mode & S_IFMT) == S_IFDIR,
        // C: info.issok  = ((st->st_mode & S_IFMT) == S_IFSOCK);
        issok: (st.mode & S_IFMT) == S_IFSOCK,
        // C: info.isfifo = ((st->st_mode & S_IFMT) == S_IFIFO);
        isfifo: (st.mode & S_IFMT) == S_IFIFO,
        // C: info.isexe  = (st->st_mode & (S_IXUSR|S_IXGRP|S_IXOTH)) ? 1 : 0;
        isexe: (st.mode & (S_IXUSR | S_IXGRP | S_IXOTH)) != 0,
        ..Info::default()
    }
}

// === 原 C 函数：void free_dir(struct _info **d) ===
/// 释放目录项数组。C 中逐一 free 各字段与节点本身；
/// Rust 中 Vec<Info> 的 Drop 自动递归释放（含 child 子树）。
pub fn free_dir(d: Vec<Info>) {
    drop(d);
}

// === 原 C 函数：int patinclude(const char *name, bool isdir, bool checkpaths) ===
/// 若 name 匹配任一 -P 模式则返回 1（应包含）。
/// checkpaths 时对 name 中每个 '/' 之后的部分逐一匹配。
pub fn patinclude(name: &str, isdir: bool, checkpaths: bool) -> i32 {
    // unsafe：读取全局模式列表 PATTERNS/PATTERN 与路径分隔符
    unsafe {
        let sep = crate::globals::FILE_PATHSEP.as_bytes()[0];
        let bytes = name.as_bytes();
        // C: for(i=0; i < pattern; i++)（PATTERNS 中已收集的前 PATTERN 项）
        for pat in PATTERNS.iter().take(PATTERN as usize) {
            // C: if (patmatch(name, patterns[i], isdir)) return 1;
            if patmatch(bytes, pat.as_bytes(), isdir) == 1 {
                return 1;
            } else if checkpaths {
                // C: pc = strchr(name, file_pathsep[0]);
                //     while (pc != NULL && *pc != '\0') { patmatch(pc+1); pc = strchr(pc+1, '/'); }
                let mut start = 0;
                while let Some(rel) = bytes[start..].iter().position(|&b| b == sep) {
                    let pos = start + rel;
                    if patmatch(&bytes[pos + 1..], pat.as_bytes(), isdir) == 1 {
                        return 1;
                    }
                    start = pos + 1;
                    if start >= bytes.len() {
                        break;
                    }
                }
            }
        }
    }
    0
}

// === 原 C 函数：int patignore(const char *name, bool isdir, bool checkpaths) ===
/// 若 name 匹配任一 -I 模式则返回 1（应忽略）。逻辑同 patinclude（用 ipatterns）。
pub fn patignore(name: &str, isdir: bool, checkpaths: bool) -> i32 {
    // unsafe：读取全局模式列表 IPATTERNS/IPATTERN
    unsafe {
        let sep = crate::globals::FILE_PATHSEP.as_bytes()[0];
        let bytes = name.as_bytes();
        // C: for(i=0; i < ipattern; i++)
        for pat in IPATTERNS.iter().take(IPATTERN as usize) {
            if patmatch(bytes, pat.as_bytes(), isdir) == 1 {
                return 1;
            } else if checkpaths {
                let mut start = 0;
                while let Some(rel) = bytes[start..].iter().position(|&b| b == sep) {
                    let pos = start + rel;
                    if patmatch(&bytes[pos + 1..], pat.as_bytes(), isdir) == 1 {
                        return 1;
                    }
                    start = pos + 1;
                    if start >= bytes.len() {
                        break;
                    }
                }
            }
        }
    }
    0
}

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
        let key = std::ffi::CStr::from_ptr(buf.as_ptr().add(i) as *const libc::c_char);
        let len = key.to_bytes().len();
        if key.to_bytes() == b"system.posix_acl_access" {
            return true;
        }
        i += len + 1;
    }
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

// === 原 C 函数：struct _info *getinfo(const char *name, char *path) ===
/// 获取单个目录项的信息（lstat + 按需 stat 跟随），并应用过滤规则。
pub fn getinfo(name: &str, path: &str) -> Option<Info> {
    // C: if (lstat(path, &lst) < 0) return NULL;
    let lst = match crate::tree::lstat_fields(path) {
        Ok(s) => s,
        Err(_) => return None,
    };

    // C: 若 lstat 结果为符号链接，则 stat 跟随；失败时清零 st（rs = -1）
    let (st, rs) = if (lst.mode & S_IFMT) == S_IFLNK {
        match stat_fields(path) {
            Ok(s) => (s, 0),
            // C: memset(&st, 0, sizeof(st))
            Err(_) => (StatFields::default(), -1),
        }
    } else {
        // C: st.st_mode = lst.st_mode; st.st_dev = lst.st_dev; st.st_ino = lst.st_ino;
        (lst, 0)
    };

    // C: isdir = (st.st_mode & S_IFMT) == S_IFDIR;
    let isdir = (st.mode & S_IFMT) == S_IFDIR;

    // unsafe：读取全局选项 FLAG 与模式计数 PATTERN/IPATTERN
    unsafe {
        // C: if (flag.gitignore && filtercheck(path, name, isdir)) return NULL;
        if FLAG.gitignore && filtercheck(path, name, isdir as i32) {
            return None;
        }
        // C: if ((lst.st_mode & S_IFMT) != S_IFDIR && !(flag.l && ((st.st_mode & S_IFMT) == S_IFDIR)))
        if (lst.mode & S_IFMT) != S_IFDIR && !(FLAG.l && (st.mode & S_IFMT) == S_IFDIR) {
            // C: if (pattern && !patinclude(name, isdir, false) && !patinclude(path, isdir, true)) return NULL;
            if PATTERN != 0 && patinclude(name, isdir, false) == 0 && patinclude(path, isdir, true) == 0
            {
                return None;
            }
        }
        // C: if (ipattern && (patignore(name, isdir, false) || patignore(path, isdir, true))) return NULL;
        if IPATTERN != 0
            && (patignore(name, isdir, false) == 1 || patignore(path, isdir, true) == 1)
        {
            return None;
        }
        // C: if (flag.d && ((st.st_mode & S_IFMT) != S_IFDIR)) return NULL;
        if FLAG.d && (st.mode & S_IFMT) != S_IFDIR {
            return None;
        }
    }

    // C: ent = xmalloc + memset(0, ...) + 逐字段赋值
    let mut ent = Info {
        name: name.to_string(),
        // C: ent->mode   = lst.st_mode; 等
        mode: lst.mode,
        uid: lst.uid,
        gid: lst.gid,
        size: lst.size,
        dev: st.dev,
        inode: st.inode,
        ldev: lst.dev,
        linode: lst.inode,
        atime: lst.atime,
        ctime: lst.ctime,
        mtime: lst.mtime,
        isdir,
        issok: (st.mode & S_IFMT) == S_IFSOCK,
        isfifo: (st.mode & S_IFMT) == S_IFIFO,
        isexe: (st.mode & (S_IXUSR | S_IXGRP | S_IXOTH)) != 0,
        ..Info::default()
    };

    // Linux 专属：ACL 与 SELinux 上下文
    #[cfg(target_os = "linux")]
    // unsafe：读取全局选项 FLAG
    unsafe {
        // C: if (flag.acl) ent->hasacl = has_acl(path);
        if FLAG.acl {
            ent.hasacl = has_acl(path);
        }
        // C: if (flag.selinux) ent->secontext = selinux_context(path); else ent->secontext = NULL;
        ent.secontext = if FLAG.selinux {
            Some(selinux_context(path))
        } else {
            None
        };
    }

    // C: if ((lst.st_mode & S_IFMT) == S_IFLNK) { readlink 处理 }
    if (lst.mode & S_IFMT) == S_IFLNK {
        match crate::tree::read_link(path) {
            // C: if ((len = readlink(path, lbuf, lbufsize-1)) < 0)
            Err(_) => {
                ent.lnk = Some("[Error reading symbolic link information]".to_string());
                ent.isdir = false;
                ent.lnkmode = st.mode;
            }
            Ok(target) => {
                ent.lnk = Some(target);
                // C: if (rs < 0) ent->orphan = true;
                if rs < 0 {
                    ent.orphan = true;
                }
                ent.lnkmode = st.mode;
            }
        }
    }

    Some(ent)
}

// === 原 C 函数：struct _info **read_dir(char *dir, ssize_t *n, int infotop) ===
/// 读取目录 dir 的所有可见条目（跳过 . 与 ..、隐藏文件、00Tree.html）。
/// n 输出条目数（-1 表示打开失败）。
pub fn read_dir(dir: &str, n: &mut i64, infotop: i32) -> Option<Vec<Info>> {
    // C: *n = -1;
    *n = -1;
    // C: if ((d = opendir(dir)) == NULL) return NULL;
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return None,
    };
    // C: bool es = (dir[strlen(dir)-1] == '/');
    let es = dir.ends_with('/');
    // C: dl = xmalloc(sizeof(struct _info *) * (ne = MINIT));
    let mut dl: Vec<Info> = Vec::with_capacity(MINIT);

    // C: while ((ent = (struct dirent *)readdir(d)))
    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let d_name = entry.file_name().to_string_lossy().into_owned();
        // C: if (!strcmp("..", d_name) || !strcmp(".", d_name)) continue;
        if d_name == ".." || d_name == "." {
            continue;
        }
        // unsafe：读取全局选项 FLAG
        unsafe {
            // C: if (flag.H && !strcmp(d_name, "00Tree.html")) continue;
            if FLAG.H && d_name == "00Tree.html" {
                continue;
            }
            // C: if (!flag.a && d_name[0] == '.') continue;
            if !FLAG.a && d_name.starts_with('.') {
                continue;
            }
        }
        // C: if (es) sprintf(path, "%s%s", dir, d_name); else sprintf(path, "%s/%s", dir, d_name);
        let path = if es {
            format!("{}{}", dir, d_name)
        } else {
            format!("{}/{}", dir, d_name)
        };

        // C: info = getinfo(d_name, path);
        if let Some(mut info) = getinfo(&d_name, &path) {
            // C: if (flag.showinfo && (com = infocheck(path, d_name, infotop, info->isdir))) {
            //      拷贝 com->desc 到 info->comment }
            if unsafe { FLAG.showinfo } {
                if let Some(com) = infocheck(&path, &d_name, infotop, info.isdir) {
                    info.comment = com.desc.clone();
                }
            }
            dl.push(info);
        }
    }

    // C: if ((*n = (ssize_t)p) == 0) { free(dl); return NULL; }
    *n = dl.len() as i64;
    if dl.is_empty() {
        return None;
    }
    Some(dl)
}

// === 原 C 函数：void push_files(const char *dir, struct ignorefile **ig, struct infofile **inf, bool top) ===
/// 为目录压入 gitignore 过滤栈与 .info 信息栈。
/// C 中压入的对象与 *ig/*inf 指向同一指针；Rust 中克隆入栈、原件交调用者跟踪
/// （调用者仅用其判断"栈上是否有需清理的项"）。
pub fn push_files(
    dir: &str,
    ig: &mut Option<Box<crate::tree::Ignorefile>>,
    inf: &mut Option<Box<crate::tree::Infofile>>,
    top: bool,
) {
    // unsafe：读取全局选项 FLAG
    unsafe {
        if FLAG.gitignore {
            let mut tig: Option<Box<crate::tree::Ignorefile>> = None;
            // C: if (top && (stmp = getenv("GIT_DIR")))
            if top {
                if let Ok(stmp) = std::env::var("GIT_DIR") {
                    // C: pathconcat(path, stmp, "info/exclude", NULL)
                    let path = pathconcat(&stmp, &["info/exclude"]);
                    let new_ig = new_ignorefile(&stmp, &path, false);
                    if let Some(b) = &new_ig {
                        push_filterstack(Some(b.clone()));
                    }
                    tig = new_ig;
                }
            }
            // C: if (top) *ig = gitignore_search(dir, 0);
            //     else push_filterstack(*ig = new_ignorefile(dir, dir, top));
            if top {
                *ig = gitignore_search(dir, 0);
            } else {
                let new_ig = new_ignorefile(dir, dir, top);
                if let Some(b) = &new_ig {
                    push_filterstack(Some(b.clone()));
                }
                *ig = new_ig;
            }
            // C: if (*ig == NULL) *ig = tig;
            if ig.is_none() {
                *ig = tig;
            }
        }
        // C: if (flag.showinfo) push_infostack(*inf = new_infofile(dir, top));
        if FLAG.showinfo {
            let new_inf = new_infofile(dir, top);
            if let Some(b) = &new_inf {
                push_infostack(Some(b.clone()));
            }
            *inf = new_inf;
        }
    }
}

// === 原 C 函数：void setoutput(const char *filename) ===
/// 设置全局输出流：filename 为 NULL 时使用 stdout，否则打开文件。
pub fn setoutput(filename: Option<&str>) {
    // unsafe：访问全局输出流 OUTFILE
    unsafe {
        match filename {
            None => {
                // C: if (outfile == NULL) outfile = stdout;
                if OUTFILE.is_none() {
                    OUTFILE = Some(Box::new(std::io::stdout()));
                }
            }
            Some(f) => {
                // C: outfile = fopen(filename, "w");
                match std::fs::File::create(f) {
                    Ok(file) => {
                        OUTFILE = Some(Box::new(std::io::BufWriter::new(file)));
                    }
                    Err(_) => {
                        // C: fprintf(stderr, "tree: invalid filename '%s'\n", filename); exit(EXIT_FAILURE);
                        eprintln!("tree: invalid filename '{}'", f);
                        std::process::exit(1);
                    }
                }
            }
        }
    }
}

fn main() {}
