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
mod file;
mod filter;
mod globals;
mod hash;
mod html;
mod info;
mod json;
mod list;
mod strverscmp;
mod tree;
mod unix;
mod util;
mod xml;

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
    DIRS, FLAG, IPATTERN, IPATTERNS, OUTFILE, PATTERN, PATTERNS,
};
use crate::info::{infocheck, new_infofile, push_infostack};
use crate::tree::{
    stat_fields, Info, StatFields, MINIT, SIXMONTHS, S_IFDIR, S_IFIFO, S_IFLNK, S_IFMT, S_IFREG,
    S_IFSOCK, S_IRUSR, S_ISGID, S_ISUID, S_ISVTX, S_IXGRP, S_IXOTH, S_IXUSR,
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

// === 原 C 函数：char *prot(mode_t m) ===
/// 生成权限字符串：文件类型字符 + rwx 权限位（含 setuid/setgid/sticky 覆盖）。
pub fn prot(m: u32) -> String {
    // C: for(i=0; ifmt[i] && (m&S_IFMT) != ifmt[i]; i++); buf[0] = fmt[i];
    let mut i = 0usize;
    while crate::globals::IFMT[i] != 0 && (m & S_IFMT) != crate::globals::IFMT[i] {
        i += 1;
    }
    let mut buf: Vec<u8> = vec![crate::globals::FMT[i]];
    const PERMS: &[u8] = b"rwxrwxrwx";
    // C: for(b=S_IRUSR, i=0; i<9; b>>=1, i++) buf[i+1] = (m & b) ? perms[i] : '-';
    let mut b: u32 = S_IRUSR;
    for &p in PERMS.iter() {
        buf.push(if m & b != 0 { p } else { b'-' });
        b >>= 1;
    }
    // C: if (m & S_ISUID) buf[3] = (buf[3]=='-')? 'S' : 's';
    if m & S_ISUID != 0 {
        buf[3] = if buf[3] == b'-' { b'S' } else { b's' };
    }
    // C: if (m & S_ISGID) buf[6] = (buf[6]=='-')? 'S' : 's';
    if m & S_ISGID != 0 {
        buf[6] = if buf[6] == b'-' { b'S' } else { b's' };
    }
    // C: if (m & S_ISVTX) buf[9] = (buf[9]=='-')? 'T' : 't';
    if m & S_ISVTX != 0 {
        buf[9] = if buf[9] == b'-' { b'T' } else { b't' };
    }
    String::from_utf8(buf).expect("prot 输出均为 ASCII")
}

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

// 天数 → (年, 月, 日)（Howard Hinnant 的 civil_from_days 算法）
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

// === 原 C 函数：void printit(const char *s) ===
/// 输出文件名，处理不可打印字符（-q 时 '?'，否则八进制转义）。
pub fn printit(s: &str) {
    // unsafe：读取全局 FLAG/MB_CUR_MAX 并输出到全局输出流
    unsafe {
        // C: if (flag.N) { 原样输出（-Q 时加引号）}
        if FLAG.N {
            if FLAG.Q {
                out!("\"{}\"", s);
            } else {
                out!("{}", s);
            }
            return;
        }
        if crate::globals::MB_CUR_MAX > 1 {
            // 等价于 C 的 mbstowcs 成功路径：Rust 的 String 恒为合法 UTF-8，
            // 按字符处理（C 中 mbstowcs 失败路径在 Rust 中不可能出现）。
            if FLAG.Q {
                outc!(b'"');
            }
            for ch in s.chars() {
                // C: if (iswprint(*tp)) fprintf("%lc")；else '?' 或 "\%03o"
                if !ch.is_control() {
                    out!("{}", ch);
                } else if FLAG.q {
                    outc!(b'?');
                } else {
                    out!("\\{:03o}", ch as u32);
                }
            }
            if FLAG.Q {
                outc!(b'"');
            }
            return;
        }
        // C: 字节路径（mb_cur_max <= 1）
        if FLAG.Q {
            outc!(b'"');
        }
        for &c in s.as_bytes() {
            // C: if ((c >= 7 && c <= 13) || c == '\\' || (c == '"' && flag.Q) || (c == ' ' && !flag.Q))
            if (7..=13).contains(&c) || c == b'\\' || (c == b'"' && FLAG.Q) || (c == b' ' && !FLAG.Q)
            {
                outc!(b'\\');
                if c > 13 {
                    outc!(c);
                } else {
                    // C: putc("abtnvfr"[c-7])（\a \b \t \n \v \f \r）
                    outc!(b"abtnvfr"[c as usize - 7]);
                }
            } else if c.is_ascii_graphic() || c == b' ' {
                // C: else if (isprint(c))
                outc!(c);
            } else {
                // C: else { if (flag.q) { if (mb_cur_max > 1 && c > 127) putc(c); else putc('?'); } else fprintf("\\%03o", c); }
                if FLAG.q {
                    if crate::globals::MB_CUR_MAX > 1 && c > 127 {
                        outc!(c);
                    } else {
                        outc!(b'?');
                    }
                } else {
                    out!("\\{:03o}", c);
                }
            }
        }
        if FLAG.Q {
            outc!(b'"');
        }
    }
}

// === 原 C 函数：int psize(char *buf, off_t size) ===
/// 将文件大小格式化到 buf（追加到末尾），返回写入的字符数。
/// -h/-si 时按单位换算，否则固定 11 位宽。
pub fn psize(buf: &mut String, size: i64) -> i32 {
    // unsafe：读取全局 FLAG
    unsafe {
        // C: static char *iec_unit="BKMGTPEZY", *si_unit = "dkMGTPEZY";
        let iec_unit = b"BKMGTPEZY";
        let si_unit = b"dkMGTPEZY";
        let unit = if FLAG.si { si_unit } else { iec_unit };
        // C: int usize = flag.si ? 1000 : 1024;
        let usize_: i64 = if FLAG.si { 1000 } else { 1024 };
        let mut size = size;
        if FLAG.h || FLAG.si {
            // C: for (idx=size<usize?0:1; size >= (usize*usize); idx++, size/=usize);
            let mut idx = if size < usize_ { 0 } else { 1 };
            while size >= usize_ * usize_ {
                idx += 1;
                size /= usize_;
            }
            let s = if idx == 0 {
                // C: sprintf(buf, " %4d", (int)size);
                format!(" {:4}", size)
            } else {
                // C: sprintf(buf, (((size+52)/usize) >= 10)? " %3.0f%c" : " %3.1f%c", (float)size/usize, unit[idx]);
                let val = size as f64 / usize_ as f64;
                if (size + 52) / usize_ >= 10 {
                    format!(" {:3.0}{}", val, unit[idx as usize] as char)
                } else {
                    format!(" {:3.1}{}", val, unit[idx as usize] as char)
                }
            };
            let n = s.len();
            buf.push_str(&s);
            n as i32
        } else {
            // C: sizeof(off_t) == sizeof(long long) ? " %11lld" : " %9lld"
            let s = format!(" {:11}", size);
            let n = s.len();
            buf.push_str(&s);
            n as i32
        }
    }
}

// === 原 C 函数：char Ftype(mode_t mode) ===
/// 返回 -F 指示符：目录 '/'、套接字 '='、FIFO '|'、链接 '@'、可执行文件 '*'。
/// 返回 0 表示无指示符。
// 函数名与 C 源码一致（首字母大写），关闭 snake_case 检查
#[allow(non_snake_case)]
pub fn Ftype(mode: u32) -> u8 {
    // unsafe：读取全局 FLAG
    unsafe {
        let m = mode & S_IFMT;
        if !FLAG.d && m == S_IFDIR {
            return b'/';
        } else if m == S_IFSOCK {
            return b'=';
        } else if m == S_IFIFO {
            return b'|';
        } else if m == S_IFLNK {
            return b'@'; /* 在此出现，但实际上从未被使用 */
        } else if m == S_IFREG && (mode & (S_IXUSR | S_IXGRP | S_IXOTH)) != 0 {
            return b'*';
        }
    }
    0
}

// === 原 C 函数：void indent(int maxlevel) ===
/// 输出缩进线（依据 dirs[] 数组决定连接线形状）。
pub fn indent(maxlevel: i32) {
    let spaces: [&[u8]; 3] = [b"   ", b"  ", b" "];
    let htmlspaces: [&[u8]; 3] = [b"&nbsp;&nbsp;&nbsp;", b"&nbsp;&nbsp;", b"&nbsp;"];
    // unsafe：读取全局 FLAG/DIRS/LINEDRAW 并输出
    unsafe {
        // C: char *space = (flag.H? "&nbsp;" : " ");
        let space: &[u8] = if FLAG.H { b"&nbsp;" } else { b" " };
        // C: int clvl = flag.compress_indent;（main 中已约束为 0..=2）
        let clvl = FLAG.compress_indent as usize;
        // C: if (flag.H) fprintf(outfile, "\t");
        if FLAG.H {
            outbytes!(b"\t");
        }
        // C: for(i=1; (i <= maxlevel) && dirs[i]; i++)
        let mut i = 1;
        while i <= maxlevel && DIRS[i as usize] != 0 {
            // C: dirs[i+1] ? (dirs[i]==1 ? vert[clvl] : (H ? htmlspaces : spaces))
            //        : (dirs[i]==1 ? vert_left[clvl] : corner[clvl])
            let piece: &[u8] = if DIRS[(i + 1) as usize] != 0 {
                if DIRS[i as usize] == 1 {
                    crate::color::LINEDRAW.vert[clvl]
                } else if FLAG.H {
                    htmlspaces[clvl]
                } else {
                    spaces[clvl]
                }
            } else if DIRS[i as usize] == 1 {
                crate::color::LINEDRAW.vert_left[clvl]
            } else {
                crate::color::LINEDRAW.corner[clvl]
            };
            outbytes!(piece);
            // C: if (flag.remove_space != true) fprintf(outfile, "%s", space);
            if !FLAG.remove_space {
                outbytes!(space);
            }
            i += 1;
        }
    }
}

// %-8.32s 格式辅助：截断到 max 字符、左对齐补齐到 min 字符
fn trunc_pad(s: &str, min: usize, max: usize) -> String {
    let mut out: String = s.chars().take(max).collect();
    while out.chars().count() < min {
        out.push(' ');
    }
    out
}

// === 原 C 函数：char *fillinfo(char *buf, const struct _info *ent) ===
/// 将文件的元数据（inode/权限/属主/大小/日期等）填充到 buf（追加）。
pub fn fillinfo(buf: &mut String, ent: Option<&Info>) {
    buf.clear();
    // C: if (!ent) return buf;
    let ent = match ent {
        Some(e) => e,
        None => return,
    };
    // unsafe：读取全局 FLAG
    unsafe {
        // C: if (flag.inode) sprintf(buf, " %7lld", ent->linode);
        if FLAG.inode {
            buf.push_str(&format!(" {:7}", ent.linode));
        }
        // C: if (flag.dev) sprintf(buf+n, " %3d", (int)ent->ldev);
        if FLAG.dev {
            buf.push_str(&format!(" {:3}", ent.ldev));
        }
        // C: if (flag.p) sprintf(buf+n, " %s", prot(ent->mode));
        if FLAG.p {
            buf.push_str(&format!(" {}", prot(ent.mode)));
        }
        #[cfg(target_os = "linux")]
        // C: if (flag.acl) sprintf(buf+n, "%c", ent->hasacl? '+' : ' ');
        if FLAG.acl {
            buf.push(if ent.hasacl { '+' } else { ' ' });
        }
        // C: if (flag.u) sprintf(buf+n, " %-8.32s", uidtoname(ent->uid));
        if FLAG.u {
            buf.push_str(&format!(" {}", trunc_pad(&crate::hash::uidtoname(ent.uid), 8, 32)));
        }
        // C: if (flag.g) sprintf(buf+n, " %-8.32s", gidtoname(ent->gid));
        if FLAG.g {
            buf.push_str(&format!(" {}", trunc_pad(&crate::hash::gidtoname(ent.gid), 8, 32)));
        }
        // C: if (flag.s) n += psize(buf+n, ent->size);
        if FLAG.s {
            psize(buf, ent.size);
        }
        // C: if (flag.D) sprintf(buf+n, " %s", do_date(flag.c? ent->ctime : ent->mtime));
        if FLAG.D {
            let t = if FLAG.c { ent.ctime } else { ent.mtime };
            buf.push_str(&format!(" {}", do_date(t)));
        }
        #[cfg(target_os = "linux")]
        // C: if (flag.selinux) sprintf(buf+n, " %s", ent->secontext);
        if FLAG.selinux {
            if let Some(sc) = &ent.secontext {
                buf.push_str(&format!(" {}", sc));
            }
        }
        // C: if (buf[0] == ' ') { buf[0] = '['; sprintf(buf+n, "]"); }
        if buf.starts_with(' ') {
            buf.replace_range(0..1, "[");
            buf.push(']');
        }
    }
}

// === 原 C 函数：void print_version(int nl) ===
/// 打印版本信息（%s 被 linedraw 的版权符号填充）。
pub fn print_version(nl: bool) {
    // C: v = version+12; sprintf(buf, "%.*s%s", strlen(v)-2, v, nl?"\n":"");
    let v = &crate::globals::VERSION[12..crate::globals::VERSION.len() - 2];
    // C: fprintf(outfile, buf, linedraw->copy);（buf 含一个 %s 占位）
    // Rust 的 format_args! 要求字面量格式串，运行时格式串用 replace 处理
    // unsafe：读取全局 LINEDRAW
    let copy = String::from_utf8_lossy(unsafe { crate::color::LINEDRAW.copy }).into_owned();
    let s = v.replace("%s", &copy);
    out!("{}", s);
    if nl {
        out!("\n");
    }
}

fn main() {}
