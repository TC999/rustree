// 文件路径：src/main.rs
// 对应 C 源文件：tree.c
// 主程序入口。模块声明随翻译进度逐步添加。
//
// 翻译过程中临时允许 dead_code：尚未翻译完所有模块时，tree.rs 中已定义
// 但尚未被引用的类型/常量会触发该警告；全部模块翻译完成后移除本属性。
//（已全部翻译完成，dead_code allow 已移除）

// 允许 static_mut_refs：本程序为单线程，所有对 static mut 全局变量的
// 访问均在 unsafe 块内并附中文注释，语义与 C 的全局变量一致；
// 直接对 static mut 取引用的方法调用（如 STATIC.take()）因此被允许。

#![allow(static_mut_refs)]
// Windows 的卷序列号/文件索引（st_dev/st_ino 近似）由 unstable feature
// 门控；本项目使用 nightly 工具链（见环境检测），故按平台启用。

mod color;
mod file;
mod filter;
mod globals;
mod hash;
mod html;
mod i18n;
mod info;
mod json;
mod list;
mod strverscmp;
mod sys;
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
    DIRS, FLAG, GETFULLTREE, IPATTERN, IPATTERNS, OUTFILE, PATTERN, PATTERNS, leak_str,
};
use crate::hash::saveino;
use crate::info::{infocheck, new_infofile, push_infostack};
use crate::sys::{lstat_fields, read_link, stat_fields};
use crate::tree::{
    Info, StatFields, MINIT, S_IFDIR, S_IFIFO, S_IFLNK, S_IFMT, S_IFREG,
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

// === 原 C 函数：struct _info *getinfo(const char *name, char *path, int infotop) ===
/// 获取单个目录项的信息（lstat + 按需 stat 跟随），并应用过滤规则；
/// infotop 非 0 时从 .info 文件提取注释（ent->comment）。
pub fn getinfo(name: &str, path: &str, infotop: i32) -> Option<Info> {
    // C: if (lstat(path, &lst) < 0) return NULL;
    let lst = match lstat_fields(path) {
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
            ent.hasacl = crate::sys::has_acl(path);
        }
        // C: if (flag.selinux) ent->secontext = selinux_context(path); else ent->secontext = NULL;
        ent.secontext = if FLAG.selinux {
            Some(crate::sys::selinux_context(path))
        } else {
            None
        };
    }

    // C: if ((lst.st_mode & S_IFMT) == S_IFLNK) { readlink 处理 }
    if (lst.mode & S_IFMT) == S_IFLNK {
        match read_link(path) {
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

    // C: if (flag.showinfo && (com = infocheck(path, name, infotop, isdir))) {
    //       for(i = 0; com->desc[i] != NULL; i++);
    //       ent->comment = xmalloc(sizeof(char *) * (i+1));
    //       for(i = 0; com->desc[i] != NULL; i++) ent->comment[i] = scopy(com->desc[i]);
    //       ent->comment[i] = NULL; }
    // unsafe：读取全局 FLAG 并查询信息栈
    unsafe {
        if FLAG.showinfo {
            if let Some(com) = infocheck(path, name, infotop, isdir) {
                ent.comment = com.desc.clone();
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

        // C: info = getinfo(d_name, path, infotop);（注释提取在 getinfo 内完成）
        if let Some(info) = getinfo(&d_name, &path, infotop) {
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
                        eprintln!("{}", crate::tr!("invalid-filename", "f" => f));
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
                    // C: putc("abtnvfr"[c-7])（\a \x08 \t \n \v \x0c \r）
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
            buf.push_str(&format!(" {}", crate::sys::do_date(t)));
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

// =====================================================================
// tree.c 排序函数、sorts 表、long_arg、usage、unix_getfulltree
// =====================================================================

use crate::filter::pop_filterstack;
use crate::globals::{
    AUTHORITY, BASESORT, ERRORS, HINTRO, HOST, HOUTRO, LC, LEVEL, MAXDIRS, MAXIPATTERN,
    MAXPATTERN, MB_CUR_MAX, NL, SCHEME, SP, TIMEFMT, TITLE, TOPSORT,
};
use crate::hash::findino;
use crate::info::pop_infostack;
use crate::list::{null_close, null_intro, null_outtro};
use crate::tree::{Ignorefile, Infofile, ListingCalls, SortFn, PATH_MAX};
use crate::unix::{unix_error, unix_newline, unix_printfile, unix_printinfo, unix_report};
use crate::util::is_singleton;
use crate::xml::{xml_close, xml_error, xml_intro, xml_newline, xml_outtro, xml_printfile, xml_printinfo, xml_report};
use crate::json::{json_close, json_error, json_intro, json_newline, json_outtro, json_printfile, json_printinfo, json_report};

#[cfg(target_os = "linux")]
use std::os::fd::FromRawFd;

// === 原 C 函数：int filesfirst(struct _info **a, struct _info **b) ===
pub fn filesfirst(a: &Info, b: &Info) -> i32 {
    if a.isdir != b.isdir {
        return if a.isdir { 1 } else { -1 };
    }
    // C: return basesort(a, b);（basesort 为 NULL 时 C 会解引用崩溃；
    // -U 与 --filesfirst 组合下 topsort 被 main 置 NULL，实际不会调用此处）
    match unsafe { BASESORT } {
        Some(f) => f(a, b),
        None => 0,
    }
}

// === 原 C 函数：int dirsfirst(struct _info **a, struct _info **b) ===
pub fn dirsfirst(a: &Info, b: &Info) -> i32 {
    if a.isdir != b.isdir {
        return if a.isdir { -1 } else { 1 };
    }
    match unsafe { BASESORT } {
        Some(f) => f(a, b),
        None => 0,
    }
}

// C 的 strcoll（locale 感知排序）；Rust std 无 strcoll，
// 用字节序比较近似（locale 差异见注释，下同）
fn name_cmp(a: &str, b: &str) -> i32 {
    match a.as_bytes().cmp(b.as_bytes()) {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    }
}

// === 原 C 函数：int alnumsort(struct _info **a, struct _info **b) ===
pub fn alnumsort(a: &Info, b: &Info) -> i32 {
    // C: int v = strcoll((*a)->name, (*b)->name);
    let v = name_cmp(&a.name, &b.name);
    // unsafe：读取全局 FLAG
    if unsafe { FLAG.reverse } {
        -v
    } else {
        v
    }
}

// === 原 C 函数：int versort(struct _info **a, struct _info **b) ===
pub fn versort(a: &Info, b: &Info) -> i32 {
    let v = crate::strverscmp::strverscmp(&a.name, &b.name);
    if unsafe { FLAG.reverse } {
        -v
    } else {
        v
    }
}

// === 原 C 函数：int mtimesort(struct _info **a, struct _info **b) ===
pub fn mtimesort(a: &Info, b: &Info) -> i32 {
    if a.mtime == b.mtime {
        let v = name_cmp(&a.name, &b.name);
        return if unsafe { FLAG.reverse } { -v } else { v };
    }
    let v = if a.mtime < b.mtime { -1 } else { 1 };
    if unsafe { FLAG.reverse } {
        -v
    } else {
        v
    }
}

// === 原 C 函数：int ctimesort(struct _info **a, struct _info **b) ===
pub fn ctimesort(a: &Info, b: &Info) -> i32 {
    if a.ctime == b.ctime {
        let v = name_cmp(&a.name, &b.name);
        return if unsafe { FLAG.reverse } { -v } else { v };
    }
    let v = if a.ctime < b.ctime { -1 } else { 1 };
    if unsafe { FLAG.reverse } {
        -v
    } else {
        v
    }
}

// === 原 C 函数：int sizecmp(off_t a, off_t b) ===
fn sizecmp(a: i64, b: i64) -> i32 {
    // C: (a == b)? 0 : ((a < b)? 1 : -1)（注意 a<b 时返回 1，即大者在前）
    if a == b {
        0
    } else if a < b {
        1
    } else {
        -1
    }
}

// === 原 C 函数：int fsizesort(struct _info **a, struct _info **b) ===
pub fn fsizesort(a: &Info, b: &Info) -> i32 {
    let mut v = sizecmp(a.size, b.size);
    if v == 0 {
        v = name_cmp(&a.name, &b.name);
    }
    if unsafe { FLAG.reverse } {
        -v
    } else {
        v
    }
}

// C: struct sorts sorts[] = {...};
// {NULL, NULL} 哨兵以数组长度 + Option 表达
struct Sort {
    name: Option<&'static str>,
    cmpfunc: Option<SortFn>,
}

static SORTS: [Sort; 6] = [
    Sort { name: Some("name"), cmpfunc: Some(alnumsort) },
    Sort { name: Some("version"), cmpfunc: Some(versort) },
    Sort { name: Some("size"), cmpfunc: Some(fsizesort) },
    Sort { name: Some("mtime"), cmpfunc: Some(mtimesort) },
    Sort { name: Some("ctime"), cmpfunc: Some(ctimesort) },
    Sort { name: Some("none"), cmpfunc: None },
];

// === 原 C 函数：char *long_arg(char *argv[], size_t i, size_t *j, size_t *n, char *prefix) ===
/// 处理 `--prefix=value` 或 `--prefix value` 形式的长选项参数。
/// C 返回 argv 内指针；Rust 返回借用 args 的 &str。
fn long_arg<'a>(args: &'a [String], i: usize, j: &mut usize, n: &mut usize, prefix: &str) -> Option<&'a str> {
    let argv_i = &args[i - 1];
    // C: if (!strncmp(prefix, argv[i], len))
    if argv_i.starts_with(prefix) {
        // C: *j = len;
        *j = prefix.len();
        // C: if (*(argv[i] + (*j)) == '=')
        if argv_i[*j..].starts_with('=') {
            // C: if (*(argv[i] + (++(*j)))) —— '=' 之后非空
            *j += 1;
            if *j < argv_i.len() {
                // C: ret = argv[i] + (*j); *j = strlen(argv[i])-1;
                let ret = &argv_i[*j..];
                *j = argv_i.len() - 1;
                Some(ret)
            } else {
                // C: fprintf(stderr, "tree: Missing argument to %s=\n", prefix);
                eprintln!("{}", crate::tr!("missing-long-arg-eq", "prefix" => prefix));
                std::process::exit(1);
            }
        } else if *n < args.len() + 1 {
            // C: else if (argv[*n] != NULL) { ret = argv[*n]; (*n)++; *j = strlen(argv[i])-1; }
            let ret = &args[*n - 1];
            *n += 1;
            *j = argv_i.len() - 1;
            Some(ret)
        } else {
            // C: else { 报错 }
            eprintln!("{}", crate::tr!("missing-long-arg", "prefix" => prefix));
            std::process::exit(1);
        }
    } else {
        None
    }
}

// === 原 C 函数：void usage(int n) ===
/// 打印使用说明。n < 2 时输出到 stderr（错误时），否则 stdout 并 exit(0)。
// 帮助文本逐行消息 ID（每行一条 FTL 消息，见 locales/en.ftl 与 locales/zh-CN.ftl；
// 参照 riptree 的语言文件风格拆分，便于逐行维护与翻译）
const USAGE_HELP_LINES: &[&str] = &[
    "help-listing-options",
    "help-all-files",
    "help-list-dirs-only",
    "help-follow-symlinks",
    "help-print-full-path",
    "help-stay-on-fs",
    "help-descend-level",
    "help-rerun-tree",
    "help-list-match-pattern",
    "help-exclude-match-pattern",
    "help-filter-gitignore",
    "help-explicit-gitfile",
    "help-ignore-case",
    "help-match-dirs",
    "help-meta-first",
    "help-prune-empty-dirs",
    "help-info-files",
    "help-explicit-infofile",
    "help-no-report",
    "help-file-limit",
    "help-condense",
    "help-output-file",
    "help-file-options",
    "help-print-nonprintable",
    "help-print-raw",
    "help-quote-filenames",
    "help-print-protections",
    "help-display-owner",
    "help-display-group",
    "help-print-size",
    "help-human-readable-size",
    "help-si-units",
    "help-compute-dir-size",
    "help-print-date",
    "help-time-format",
    "help-append-ls",
    "help-print-inodes",
    "help-print-device",
    "help-sorting-options",
    "help-sort-version",
    "help-sort-mtime",
    "help-sort-ctime",
    "help-unsorted",
    "help-reverse-sort",
    "help-dirs-first",
    "help-files-first",
    "help-select-sort",
    "help-graphics-options",
    "help-no-indent",
    "help-ansi-lines",
    "help-no-color",
    "help-force-color",
    "help-compress-lines",
    "help-xml-html-options",
    "help-xml-output",
    "help-json-output",
    "help-html-output",
    "help-html-title",
    "help-no-links",
    "help-html-intro",
    "help-html-outro",
    "help-hyperlink",
    "help-scheme",
    "help-authority",
    "help-input-options",
    "help-from-file",
    "help-from-tabfile",
    "help-fflinks",
    "help-misc-options",
    "help-opt-toggle",
    "help-print-version",
    "help-print-help",
    "help-options-terminator",
];

pub fn usage(n: i32) {
    crate::color::parse_dir_colors();
    crate::color::initlinedraw(false);

    // C: fancy(n < 2 ? stderr : stdout, ...)（C 中传入 FILE* 流）
    let mut err_out = std::io::stderr();
    let mut stdout_out = std::io::stdout();
    let out: &mut dyn std::io::Write = if n < 2 {
        &mut err_out
    } else {
        &mut stdout_out
    };
    crate::color::fancy(out, &crate::tr!("usage-summary"));
    // summary 消息以纯文本结束（无尾换行），补一个换行
    out.write_all(b"\n").ok();

    if n < 2 {
        return;
    }
    // 逐行输出帮助文本（每行一条消息，无 fancy 控制字符，纯文本）
    for msg in USAGE_HELP_LINES {
        let line = crate::i18n::tr(msg, &[], Vec::new());
        crate::color::fancy(&mut std::io::stdout(), &line);
        // 每条消息一行：消息值不含换行，输出端补
        std::io::Write::write_all(&mut std::io::stdout(), b"\n").ok();
    }
    std::process::exit(0);
}

// === 原 C 函数：struct _info **unix_getfulltree(char *d, u_long lev, dev_t dev, off_t *size, char **err) ===
/// 读取完整目录树（--du/--prune/--matchdirs/--condense 时使用）。
/// 递归遍历，处理符号链接循环检测与 -f/-x/-L/--filelimit/prune/condense。
pub fn unix_getfulltree(
    d: &str,
    lev: u64,
    mut dev: u64,
    size: &mut i64,
    err: &mut Option<String>,
) -> Option<Vec<Info>> {
    // path 的初始赋值对应 C 的 xmalloc(pathsize=PATH_MAX)，循环内被覆盖，
    // 初始值本身不读（C 语义）
    #[allow(unused_assignments)]
    let mut path = String::new();
    let mut pathsize: usize = PATH_MAX;
    // C: *err = NULL;
    *err = None;
    // C: if (Level >= 0 && lev > (u_long)Level) return NULL;
    if unsafe { LEVEL } >= 0 && lev as i64 > unsafe { LEVEL } {
        return None;
    }
    // C: if (flag.xdev && lev == 0) { stat(d, &sb); dev = sb.st_dev; }
    if unsafe { FLAG.xdev } && lev == 0 {
        if let Ok(sb) = stat_fields(d) {
            dev = sb.dev;
        }
    }

    // C: last_name = strrchr(d, file_pathsep[0]);
    let last_name = d.rfind('/');
    // C: if (pattern && (patinclude(d, true, true) ||
    //                    (last_name && patinclude(last_name+1, true, false)))) { tmp_pattern = pattern; pattern = 0; }
    let mut tmp_pattern: i32 = 0;
    // unsafe：读写全局 PATTERN
    unsafe {
        if PATTERN != 0 {
            let d_match = patinclude(d, true, true) == 1;
            let name_match = last_name
                .map(|pos| patinclude(&d[pos + 1..], true, false) == 1)
                .unwrap_or(false);
            if d_match || name_match {
                tmp_pattern = PATTERN;
                PATTERN = 0;
            }
        }
    }

    // C: push_files(d, &ig, &inf, lev==0);
    let mut ig: Option<Box<Ignorefile>> = None;
    let mut inf: Option<Box<Infofile>> = None;
    push_files(d, &mut ig, &mut inf, lev == 0);

    // C: sav = dir = read_dir(d, &n, inf != NULL);
    let mut n: i64 = 0;
    let mut sav = read_dir(d, &mut n, if inf.is_some() { 1 } else { 0 });

    // C: if (dir == NULL && n) { *err = scopy("error opening dir"); ... return NULL; }
    if sav.is_none() && n != 0 {
        *err = Some(crate::tr!("error-opening-dir"));
        if tmp_pattern != 0 {
            // unsafe：恢复全局 PATTERN
            unsafe { PATTERN = tmp_pattern; }
        }
        return None;
    }
    // C: if (n == 0) { if (sav != NULL) free_dir(sav); ... return NULL; }
    if n == 0 {
        if tmp_pattern != 0 {
            // unsafe：恢复全局 PATTERN
            unsafe { PATTERN = tmp_pattern; }
        }
        return None;
    }

    // C: path = xmalloc(pathsize = PATH_MAX);（path/pathsize 已在函数开头定义）

    // C: if (flag.flimit > 0 && n > flag.flimit) { *err = ...; return NULL; }
    // unsafe：读取全局 FLAG
    unsafe {
        if FLAG.flimit > 0 && n > FLAG.flimit as i64 {
            *err = Some(crate::tr!("filelimit-exceeded", "n" => n));
            if tmp_pattern != 0 {
                PATTERN = tmp_pattern;
            }
            return None;
        }
    }

    // C: if (lev >= (u_long)maxdirs-1) dirs = xrealloc(...)
    // unsafe：读写全局 DIRS/MAXDIRS
    unsafe {
        if lev as usize >= MAXDIRS.saturating_sub(1) {
            MAXDIRS += 1024;
            DIRS.resize(MAXDIRS, 0);
        }
    }

    // C: while (*dir) { ... }
    // n > 0 时 read_dir 返回 Some（上方 n==0 已提前返回）
    let sav_vec = sav.as_mut().expect("n>0 时 sav 非空");
    let mut idx = 0;
    while idx < sav_vec.len() {
        // C: if ((*dir)->isdir && !(flag.xdev && dev != (*dir)->dev))
        if sav_vec[idx].isdir && !(unsafe { FLAG.xdev } && dev != sav_vec[idx].dev) {
            if sav_vec[idx].lnk.is_some() {
                // C: if (flag.l) { ... }
                if unsafe { FLAG.l } {
                    if findino(sav_vec[idx].inode, sav_vec[idx].dev) {
                        sav_vec[idx].err = Some(crate::tr!("recursive-not-followed"));
                    } else {
                        saveino(sav_vec[idx].inode, sav_vec[idx].dev);
                        let lnk = sav_vec[idx].lnk.clone().unwrap();
                        if lnk.starts_with('/') {
                            // C: (*dir)->child = unix_getfulltree((*dir)->lnk, lev+1, ...)
                            let mut child_err: Option<String> = None;
                            let mut child_size: i64 = sav_vec[idx].size;
                            let child = unix_getfulltree(&lnk, lev + 1, dev, &mut child_size, &mut child_err);
                            sav_vec[idx].child = child;
                            sav_vec[idx].size = child_size;
                            sav_vec[idx].err = child_err;
                        } else {
                            // C: if (strlen(d)+strlen(lnk)+2 > pathsize) path = xrealloc(...)
                            if d.len() + lnk.len() + 2 > pathsize {
                                pathsize = d.len() + lnk.len() + 1024;
                            }
                            // C: if (flag.f && !strcmp(d,"/")) sprintf(path,"%s%s",d,lnk);
                            //     else sprintf(path,"%s/%s",d,lnk);
                            path = if unsafe { FLAG.f } && d == "/" {
                                format!("{}{}", d, lnk)
                            } else {
                                format!("{}/{}", d, lnk)
                            };
                            let mut child_err: Option<String> = None;
                            let mut child_size: i64 = sav_vec[idx].size;
                            let child = unix_getfulltree(&path, lev + 1, dev, &mut child_size, &mut child_err);
                            sav_vec[idx].child = child;
                            sav_vec[idx].size = child_size;
                            sav_vec[idx].err = child_err;
                        }
                    }
                }
            } else {
                // C: 非链接目录
                if d.len() + sav_vec[idx].name.len() + 2 > pathsize {
                    pathsize = d.len() + sav_vec[idx].name.len() + 1024;
                }
                path = if unsafe { FLAG.f } && d == "/" {
                    format!("{}{}", d, sav_vec[idx].name)
                } else {
                    format!("{}/{}", d, sav_vec[idx].name)
                };
                saveino(sav_vec[idx].inode, sav_vec[idx].dev);
                let mut child_err: Option<String> = None;
                let mut child_size: i64 = sav_vec[idx].size;
                let child = unix_getfulltree(&path, lev + 1, dev, &mut child_size, &mut child_err);
                sav_vec[idx].child = child;
                sav_vec[idx].size = child_size;
                sav_vec[idx].err = child_err;

                // C: if (flag.condense_singletons) { while (is_singleton(*dir)) { ... } }
                if unsafe { FLAG.condense_singletons } {
                    while is_singleton(&sav_vec[idx]) {
                        let child: Vec<Info> = sav_vec[idx].child.take().expect("is_singleton 要求 child 非空");
                        let name = pathconcat(&sav_vec[idx].name, &[&child[0].name]);
                        sav_vec[idx].name = name;
                        let mut child0 = child.into_iter().next().unwrap();
                        sav_vec[idx].child = child0.child.take();
                        sav_vec[idx].condensed = sav_vec[idx].condensed + 1 + child0.condensed;
                    }
                }
            }
            // C: if (flag.prune && (*dir)->child == NULL &&
            //        !(flag.matchdirs && pattern && patinclude((*dir)->name, isdir, false))) { 删除并 continue; }
            // unsafe：读取全局 FLAG/PATTERN
            unsafe {
                let prune_cond = FLAG.prune
                    && sav_vec[idx].child.is_none()
                    && !(FLAG.matchdirs
                        && PATTERN != 0
                        && patinclude(&sav_vec[idx].name, sav_vec[idx].isdir, false) == 1);
                if prune_cond {
                    // C: for(p=dir;*p;p++) *p = *(p+1); n--; free(xp);
                    sav_vec.remove(idx);
                    n -= 1;
                    continue;
                }
            }
        }
        // C: if (flag.du) *size += (*dir)->size;
        if unsafe { FLAG.du } {
            *size += sav_vec[idx].size;
        }
        idx += 1;
    }

    // C: if (tmp_pattern) { pattern = tmp_pattern; tmp_pattern = 0; }
    if tmp_pattern != 0 {
        // unsafe：恢复全局 PATTERN
        unsafe { PATTERN = tmp_pattern; }
    }

    // C: if (topsort) qsort(sav, n, ...)（qsort 不稳定 → sort_unstable_by）
    if let Some(f) = unsafe { TOPSORT } {
        sav.as_mut().unwrap().sort_unstable_by(|a, b| f(a, b).cmp(&0));
    }

    // C: if (n == 0) { free_dir(sav); return NULL; }
    if n == 0 {
        return None;
    }
    // C: if (ig != NULL) pop_filterstack();
    if ig.is_some() {
        pop_filterstack();
    }
    // C: if (inf != NULL) pop_infostack();
    if inf.is_some() {
        pop_infostack();
    }
    sav
}

// === 原 C 函数：int main(int argc, char **argv) ===
fn main() {
    // i18n：按系统 locale（LC_ALL/LC_MESSAGES/LANG）初始化语言包（默认英文）
    crate::i18n::init();
    // 默认 HTML 标题随语言本地化（-T 选项在参数解析时覆盖此默认值）
    if crate::i18n::lang() != "en" {
        // unsafe：写全局 TITLE（单线程）
        unsafe {
            TITLE = crate::globals::leak_str(crate::tr!("html-title"));
        }
    }

    // C 中 argv[0] 是程序名；Rust 的 args 已跳过
    let args: Vec<String> = std::env::args().skip(1).collect();
    let argc = args.len() + 1; // C 的 argc（含程序名）

    let mut dirname: Vec<String> = Vec::new();
    let mut outfilename: Option<String> = None;
    let needfulltree: bool;
    // 说明：needfulltree 在参数解析后的 unsafe 块中一次性赋值（对应 C 中
    // bool needfulltree = flag.du || ... 的语义），声明为不可变延迟初始化
    let mut showversion = false;
    let mut opt_toggle = false;
    let mut optf = true;

    // C: memset(&flag, 0, sizeof(flag));（Flags::new() 全 false，static 已初始化）
    // C: dirs = xmalloc(...maxdirs=PATH_MAX); memset(dirs, 0, ...);
    // unsafe：初始化全局 DIRS/MAXDIRS/LEVEL
    unsafe {
        MAXDIRS = PATH_MAX;
        DIRS.resize(MAXDIRS, 0);
        LEVEL = -1;
    }

    // C: setlocale(LC_CTYPE, ""); setlocale(LC_COLLATE, "");
    // 平台抽象：见 sys.rs（Unix 调 libc::setlocale，非 Unix 空实现）
    crate::sys::set_locale_ctype_collate();

    // C: charset = getcharset(); ...（字符集检测已随 --charset/TREE_CHARSET 机制移除，
    // 图形线固定为 UTF-8 或默认，见 color.rs 的 initlinedraw）

    // C: lc = (struct listingcalls){ null_intro, ..., unix_report };
    // unsafe：写全局 LC
    unsafe {
        LC = Some(ListingCalls {
            intro: null_intro,
            outtro: null_outtro,
            printinfo: unix_printinfo,
            printfile: unix_printfile,
            error: unix_error,
            newline: unix_newline,
            close: null_close,
            report: unix_report,
        });
    }

    // C: #ifdef MB_CUR_MAX mb_cur_max = MB_CUR_MAX; #else 1
    // Rust 字符串恒为 UTF-8：设 4（UTF-8 最大字节数），使 printit 走字符路径
    //（等价于 C 在 UTF-8 locale 下的行为）
    // unsafe：写全局 MB_CUR_MAX
    unsafe {
        MB_CUR_MAX = 4;
    }

    // C: #ifdef __linux__ 的 STDDATA_FD 处理（JSON 自动输出到 stddata fd）
    #[cfg(target_os = "linux")]
    {
        if let Ok(fd_str) = std::env::var(crate::tree::ENV_STDDATA_FD) {
            let mut std_fd = fd_str.parse::<i32>().unwrap_or(0);
            if std_fd <= 0 {
                std_fd = crate::tree::STDDATA_FILENO;
            }
            // C: if (fcntl(std_fd, F_GETFD) >= 0)
            // unsafe：调用 C 库函数 fcntl（libc 无安全封装）
            let ok = unsafe { libc::fcntl(std_fd, libc::F_GETFD) >= 0 };
            if ok {
                // unsafe：读写全局 FLAG/NL/LC/OUTFILE
                unsafe {
                    FLAG.J = true;
                    FLAG.noindent = true;
                    NL = "";
                    LC = Some(ListingCalls {
                        intro: json_intro,
                        outtro: json_outtro,
                        printinfo: json_printinfo,
                        printfile: json_printfile,
                        error: json_error,
                        newline: json_newline,
                        close: json_close,
                        report: json_report,
                    });
                    // C: outfile = fdopen(std_fd, "w");
                    // 用 std::os::fd::FromRawFd 包装（接管 fd 的所有权）
                    let file = std::fs::File::from_raw_fd(std_fd);
                    OUTFILE = Some(Box::new(std::io::BufWriter::new(file)));
                }
            }
        }
    }

    crate::hash::init_hashes();

    // C: for(n=i=1; i<(size_t)argc; i=n) { n++; ... }
    // 所有对全局状态的读写都在 unsafe 块内（单线程）
    // j/n 的“赋值后不读”对应 C 中 for 循环的 j++/n++ 推进，语义保留
    #[allow(unused_assignments)]
    unsafe {
        let mut i: usize = 1;
        let mut n: usize;
        let mut j: usize;
        while i < argc {
            n = i + 1;
            let argi = &args[i - 1];
            if optf && argi.starts_with('-') && argi.len() > 1 {
                let bytes = argi.as_bytes();
                j = 1;
                while j < bytes.len() {
                    match bytes[j] {
                        b'N' => {
                            FLAG.N = if opt_toggle { !FLAG.N } else { true };
                        }
                        b'q' => {
                            FLAG.q = if opt_toggle { !FLAG.q } else { true };
                        }
                        b'Q' => {
                            FLAG.Q = if opt_toggle { !FLAG.Q } else { true };
                        }
                        b'd' => {
                            FLAG.d = if opt_toggle { !FLAG.d } else { true };
                        }
                        b'l' => {
                            FLAG.l = if opt_toggle { !FLAG.l } else { true };
                        }
                        b's' => {
                            FLAG.s = if opt_toggle { !FLAG.s } else { true };
                        }
                        b'h' => {
                            // C: /* Assume they also want -s */ flag.s = (flag.h = ...);
                            FLAG.h = if opt_toggle { !FLAG.h } else { true };
                            FLAG.s = FLAG.h;
                        }
                        b'u' => {
                            FLAG.u = if opt_toggle { !FLAG.u } else { true };
                        }
                        b'g' => {
                            FLAG.g = if opt_toggle { !FLAG.g } else { true };
                        }
                        b'f' => {
                            FLAG.f = if opt_toggle { !FLAG.f } else { true };
                        }
                        b'F' => {
                            FLAG.F = if opt_toggle { !FLAG.F } else { true };
                        }
                        b'a' => {
                            FLAG.a = if opt_toggle { !FLAG.a } else { true };
                        }
                        b'p' => {
                            FLAG.p = if opt_toggle { !FLAG.p } else { true };
                        }
                        b'i' => {
                            FLAG.noindent = if opt_toggle { !FLAG.noindent } else { true };
                            // C: _nl = "";
                            NL = "";
                        }
                        b'C' => {
                            FLAG.force_color = if opt_toggle { !FLAG.force_color } else { true };
                        }
                        b'n' => {
                            FLAG.nocolor = if opt_toggle { !FLAG.nocolor } else { true };
                        }
                        b'x' => {
                            FLAG.xdev = if opt_toggle { !FLAG.xdev } else { true };
                        }
                        b'P' => {
                            if n >= argc {
                                eprintln!("{}", crate::tr!("missing-option-arg", "opt" => "P"));
                                std::process::exit(1);
                            }
                            // C: if (pattern >= maxpattern-1) patterns = xrealloc(...)
                            //     patterns[pattern++] = argv[n++]; patterns[pattern] = NULL;
                            // Vec 自动扩容，保留 maxpattern 逻辑
                            if PATTERN >= MAXPATTERN - 1 {
                                MAXPATTERN += 10;
                            }
                            PATTERNS.push(args[n - 1].clone());
                            PATTERN += 1;
                            n += 1;
                        }
                        b'I' => {
                            if n >= argc {
                                eprintln!("{}", crate::tr!("missing-option-arg", "opt" => "I"));
                                std::process::exit(1);
                            }
                            if IPATTERN >= MAXIPATTERN - 1 {
                                MAXIPATTERN += 10;
                            }
                            IPATTERNS.push(args[n - 1].clone());
                            IPATTERN += 1;
                            n += 1;
                        }
                        b'A' => {
                            FLAG.ansilines = if opt_toggle { !FLAG.ansilines } else { true };
                        }
                        // -S（CP437 控制台图形）已删除：字符集仅保留 UTF-8 与默认
                        b'D' => {
                            FLAG.D = if opt_toggle { !FLAG.D } else { true };
                        }
                        b't' => {
                            BASESORT = Some(mtimesort);
                        }
                        b'c' => {
                            BASESORT = Some(ctimesort);
                            FLAG.c = true;
                        }
                        b'r' => {
                            FLAG.reverse = if opt_toggle { !FLAG.reverse } else { true };
                        }
                        b'v' => {
                            BASESORT = Some(versort);
                        }
                        b'U' => {
                            BASESORT = None;
                        }
                        b'X' => {
                            FLAG.X = true;
                            FLAG.H = false;
                            FLAG.J = false;
                            LC = Some(ListingCalls {
                                intro: xml_intro,
                                outtro: xml_outtro,
                                printinfo: xml_printinfo,
                                printfile: xml_printfile,
                                error: xml_error,
                                newline: xml_newline,
                                close: xml_close,
                                report: xml_report,
                            });
                        }
                        b'J' => {
                            FLAG.J = true;
                            FLAG.X = false;
                            FLAG.H = false;
                            LC = Some(ListingCalls {
                                intro: json_intro,
                                outtro: json_outtro,
                                printinfo: json_printinfo,
                                printfile: json_printfile,
                                error: json_error,
                                newline: json_newline,
                                close: json_close,
                                report: json_report,
                            });
                        }
                        b'H' => {
                            FLAG.H = true;
                            FLAG.X = false;
                            FLAG.J = false;
                            LC = Some(ListingCalls {
                                intro: crate::html::html_intro,
                                outtro: crate::html::html_outtro,
                                printinfo: crate::html::html_printinfo,
                                printfile: crate::html::html_printfile,
                                error: crate::html::html_error,
                                newline: crate::html::html_newline,
                                close: crate::html::html_close,
                                report: crate::html::html_report,
                            });
                            if n >= argc {
                                eprintln!("{}", crate::tr!("missing-option-arg", "opt" => "H"));
                                std::process::exit(1);
                            }
                            let mut host = args[n - 1].clone();
                            n += 1;
                            // C: k = strlen(host)-1;（仅被注释代码使用，省略）
                            if host.starts_with('-') {
                                FLAG.htmloffset = true;
                                host = host[1..].to_string();
                            }
                            HOST = Some(leak_str(host));
                            // C: sp = "&nbsp;";
                            SP = "&nbsp;";
                        }
                        b'T' => {
                            if n >= argc {
                                eprintln!("{}", crate::tr!("missing-option-arg", "opt" => "T"));
                                std::process::exit(1);
                            }
                            TITLE = leak_str(args[n - 1].clone());
                            n += 1;
                        }
                        b'R' => {
                            FLAG.R = if opt_toggle { !FLAG.R } else { true };
                        }
                        b'L' => {
                            // C: if (isdigit(argv[i][j+1])) { 内联数字 } else { sLevel = argv[n++]; }
                            let next_digit = bytes.get(j + 1).is_some_and(|c| c.is_ascii_digit());
                            let s_level: String = if next_digit {
                                let mut k = 0;
                                let mut s = String::new();
                                while let Some(&c2) = bytes.get(j + 1 + k) {
                                    if !c2.is_ascii_digit() {
                                        break;
                                    }
                                    if k >= PATH_MAX - 1 {
                                        break;
                                    }
                                    s.push(c2 as char);
                                    k += 1;
                                }
                                j += k;
                                s
                            } else {
                                if n >= argc {
                                    eprintln!("{}", crate::tr!("missing-option-arg", "opt" => "L"));
                                    std::process::exit(1);
                                }
                                let s = args[n - 1].clone();
                                n += 1;
                                s
                            };
                            // C: Level = (int)strtoul(sLevel, NULL, 0) - 1;
                            LEVEL = s_level.parse::<u64>().unwrap_or(0) as i64 - 1;
                            if LEVEL < 0 {
                                eprintln!("{}", crate::tr!("invalid-level"));
                                std::process::exit(1);
                            }
                        }
                        b'o' => {
                            if n >= argc {
                                eprintln!("{}", crate::tr!("missing-option-arg", "opt" => "o"));
                                std::process::exit(1);
                            }
                            outfilename = Some(args[n - 1].clone());
                            n += 1;
                        }
                        b'-' => {
                            if j == 1 {
                                // C: 长选项处理
                                if argi == "--" {
                                    optf = false;
                                    break;
                                }
                                if argi == "--help" {
                                    usage(2);
                                    std::process::exit(0);
                                }
                                if argi == "--version" {
                                    j = argi.len() - 1;
                                    showversion = true;
                                    break;
                                }
                                if argi == "--inodes" {
                                    j = argi.len() - 1;
                                    FLAG.inode = if opt_toggle { !FLAG.inode } else { true };
                                    break;
                                }
                                if argi == "--device" {
                                    j = argi.len() - 1;
                                    FLAG.dev = if opt_toggle { !FLAG.dev } else { true };
                                    break;
                                }
                                if argi == "--noreport" {
                                    j = argi.len() - 1;
                                    FLAG.noreport = if opt_toggle { !FLAG.noreport } else { true };
                                    break;
                                }
                                if argi == "--nolinks" {
                                    j = argi.len() - 1;
                                    FLAG.nolinks = if opt_toggle { !FLAG.nolinks } else { true };
                                    break;
                                }
                                if argi == "--dirsfirst" {
                                    j = argi.len() - 1;
                                    TOPSORT = Some(dirsfirst);
                                    break;
                                }
                                if argi == "--filesfirst" {
                                    j = argi.len() - 1;
                                    TOPSORT = Some(filesfirst);
                                    break;
                                }
                                if let Some(a) = long_arg(&args, i, &mut j, &mut n, "--filelimit") {
                                    FLAG.flimit = a.parse::<i32>().unwrap_or(0);
                                    break;
                                }
                                // --charset 已随字符集机制移除（图形线固定 UTF-8 或默认）
                                if argi == "--si" {
                                    j = argi.len() - 1;
                                    FLAG.s = true;
                                    FLAG.h = true;
                                    FLAG.si = if opt_toggle { !FLAG.si } else { true };
                                    break;
                                }
                                if argi == "--du" {
                                    j = argi.len() - 1;
                                    FLAG.s = if opt_toggle { !FLAG.du } else { true };
                                    FLAG.du = FLAG.s;
                                    break;
                                }
                                if argi == "--prune" {
                                    j = argi.len() - 1;
                                    FLAG.prune = if opt_toggle { !FLAG.prune } else { true };
                                    break;
                                }
                                if let Some(a) = long_arg(&args, i, &mut j, &mut n, "--timefmt") {
                                    TIMEFMT = Some(leak_str(a.to_string()));
                                    FLAG.D = true;
                                    break;
                                }
                                if argi == "--ignore-case" {
                                    j = argi.len() - 1;
                                    FLAG.ignorecase = if opt_toggle { !FLAG.ignorecase } else { true };
                                    break;
                                }
                                if argi == "--matchdirs" {
                                    j = argi.len() - 1;
                                    FLAG.matchdirs = if opt_toggle { !FLAG.matchdirs } else { true };
                                    break;
                                }
                                if let Some(a) = long_arg(&args, i, &mut j, &mut n, "--sort") {
                                    BASESORT = None;
                                    let mut found = false;
                                    for s in SORTS.iter() {
                                        if let Some(name) = s.name {
                                            if name.eq_ignore_ascii_case(a) {
                                                BASESORT = s.cmpfunc;
                                                found = true;
                                                break;
                                            }
                                        }
                                    }
                                    if !found {
                                        // C: fprintf(stderr, "tree: Sort type '%s' not valid, should be one of: ", arg);
                                        let names: Vec<&str> = SORTS
                                            .iter()
                                            .filter_map(|s| s.name)
                                            .collect();
                                        eprintln!(
                                            "{}",
                                            crate::tr!("invalid-sort", "arg" => a, "list" => names.join(","))
                                        );
                                        std::process::exit(1);
                                    }
                                    break;
                                }
                                if argi == "--fromtabfile" {
                                    j = argi.len() - 1;
                                    FLAG.fromfile = true;
                                    GETFULLTREE = Some(crate::file::tabedfile_getfulltree);
                                    break;
                                }
                                if argi == "--fromfile" {
                                    j = argi.len() - 1;
                                    FLAG.fromfile = true;
                                    GETFULLTREE = Some(crate::file::file_getfulltree);
                                    break;
                                }
                                if argi == "--metafirst" {
                                    j = argi.len() - 1;
                                    FLAG.metafirst = if opt_toggle { !FLAG.metafirst } else { true };
                                    break;
                                }
                                if let Some(a) = long_arg(&args, i, &mut j, &mut n, "--gitfile") {
                                    FLAG.gitignore = true;
                                    let new_ig = new_ignorefile(a, a, false);
                                    match new_ig {
                                        Some(b) => push_filterstack(Some(b)),
                                        None => {
                                            eprintln!("{}", crate::tr!("load-gitignore-fail"));
                                            std::process::exit(1);
                                        }
                                    }
                                    break;
                                }
                                if argi == "--gitignore" {
                                    j = argi.len() - 1;
                                    FLAG.gitignore = if opt_toggle { !FLAG.gitignore } else { true };
                                    break;
                                }
                                if argi == "--info" {
                                    j = argi.len() - 1;
                                    FLAG.showinfo = if opt_toggle { !FLAG.showinfo } else { true };
                                    break;
                                }
                                if let Some(a) = long_arg(&args, i, &mut j, &mut n, "--infofile") {
                                    FLAG.showinfo = true;
                                    let new_inf = new_infofile(a, false);
                                    match new_inf {
                                        Some(b) => push_infostack(Some(b)),
                                        None => {
                                            eprintln!("{}", crate::tr!("load-infofile-fail"));
                                            std::process::exit(1);
                                        }
                                    }
                                    break;
                                }
                                if let Some(a) = long_arg(&args, i, &mut j, &mut n, "--hintro") {
                                    HINTRO = Some(leak_str(a.to_string()));
                                    break;
                                }
                                if let Some(a) = long_arg(&args, i, &mut j, &mut n, "--houtro") {
                                    HOUTRO = Some(leak_str(a.to_string()));
                                    break;
                                }
                                if argi == "--fflinks" {
                                    j = argi.len() - 1;
                                    FLAG.fflinks = if opt_toggle { !FLAG.fflinks } else { true };
                                    break;
                                }
                                if argi == "--hyperlink" {
                                    j = argi.len() - 1;
                                    FLAG.hyper = if opt_toggle { !FLAG.hyper } else { true };
                                    break;
                                }
                                if let Some(a) = long_arg(&args, i, &mut j, &mut n, "--scheme") {
                                    // C: if (strchr(arg, ':') == NULL) { sprintf("%s://", arg); }
                                    if !a.contains(':') {
                                        SCHEME = leak_str(format!("{}://", a));
                                    } else {
                                        SCHEME = leak_str(a.to_string());
                                    }
                                    break;
                                }
                                if let Some(a) = long_arg(&args, i, &mut j, &mut n, "--authority") {
                                    // C: '.' 视为空 authority
                                    AUTHORITY = if a == "." {
                                        Some("")
                                    } else {
                                        Some(leak_str(a.to_string()))
                                    };
                                    break;
                                }
                                if argi == "--opt-toggle" {
                                    j = argi.len() - 1;
                                    opt_toggle = !opt_toggle;
                                    break;
                                }
                                if argi == "--condense" {
                                    j = argi.len() - 1;
                                    FLAG.condense_singletons =
                                        if opt_toggle { !FLAG.condense_singletons } else { true };
                                    break;
                                }
                                if let Some(a) = long_arg(&args, i, &mut j, &mut n, "--compress") {
                                    FLAG.compress_indent = a.parse::<i32>().unwrap_or(0);
                                    FLAG.remove_space = FLAG.compress_indent < 0;
                                    if FLAG.compress_indent < 0 {
                                        FLAG.compress_indent = -FLAG.compress_indent;
                                    }
                                    if FLAG.compress_indent > 3 {
                                        FLAG.compress_indent = 0;
                                        FLAG.noindent = true;
                                        NL = "";
                                    }
                                    if FLAG.compress_indent > 0 {
                                        FLAG.compress_indent -= 1;
                                    }
                                    break;
                                }
                                #[cfg(target_os = "linux")]
                                {
                                    if argi == "--acl" {
                                        j = argi.len() - 1;
                                        FLAG.acl = if opt_toggle { !FLAG.acl } else { true };
                                        FLAG.p = if FLAG.acl { true } else { FLAG.p };
                                        break;
                                    }
                                    if argi == "--selinux" {
                                        j = argi.len() - 1;
                                        FLAG.selinux = if opt_toggle { !FLAG.selinux } else { true };
                                        break;
                                    }
                                }
                                eprintln!("{}", crate::tr!("invalid-option", "arg" => argi));
                                usage(1);
                                std::process::exit(1);
                            }
                            // C: 落入 default
                            eprintln!("{}", crate::tr!("invalid-option-char", "char" => bytes[j] as char));
                            usage(1);
                            std::process::exit(1);
                        }
                        _ => {
                            eprintln!("{}", crate::tr!("invalid-option-char", "char" => bytes[j] as char));
                            usage(1);
                            std::process::exit(1);
                        }
                    }
                    j += 1;
                }
            } else {
                dirname.push(argi.clone());
            }
            i = n;
        }
    }

    // C: setoutput(outfilename);
    setoutput(outfilename.as_deref());
    crate::color::parse_dir_colors();
    crate::color::initlinedraw(false);

    if showversion {
        print_version(true);
        std::process::exit(0);
    }

    // unsafe：读写全局 TOPSORT/BASESORT/FLAG 等
    unsafe {
        // C: if (dirname == NULL) { dirname[0] = "."; }
        if dirname.is_empty() {
            dirname.push(".".to_string());
        }
        // C: if (topsort == NULL) topsort = basesort;
        if TOPSORT.is_none() {
            TOPSORT = BASESORT;
        }
        // C: if (basesort == NULL) topsort = NULL;
        if BASESORT.is_none() {
            TOPSORT = None;
        }
        // C: if (timefmt) setlocale(LC_TIME, "");
        // 平台抽象：见 sys.rs（Unix 调 libc::setlocale，非 Unix 空实现）
        if TIMEFMT.is_some() {
            crate::sys::set_locale_time();
        }
        // C: if (flag.d) flag.prune = false;（否则什么都得不到）
        if FLAG.d {
            FLAG.prune = false;
        }
        // C: if (flag.R && (Level == -1)) flag.R = false;
        if FLAG.R && LEVEL == -1 {
            FLAG.R = false;
        }

        // C: if (flag.hyper && authority == NULL) { gethostname(...) }
        if FLAG.hyper && AUTHORITY.is_none() {
            // 平台抽象：见 sys.rs（Unix 调 gethostname；非 Unix 用 COMPUTERNAME/localhost）
            match crate::sys::get_hostname() {
                None => {
                    // C: fprintf(stderr, "Unable to get hostname, using 'localhost'.\n");
                    eprintln!("{}", crate::tr!("get-hostname-fail"));
                    AUTHORITY = Some("localhost");
                }
                Some(name) => AUTHORITY = Some(leak_str(name)),
            }
        }

        // C: if (flag.showinfo) push_infostack(new_infofile(INFO_PATH, false));
        if FLAG.showinfo {
            push_infostack(new_infofile(crate::tree::INFO_PATH, false));
        }

        // C: needfulltree = flag.du || flag.prune || flag.matchdirs || flag.fromfile || flag.condense_singletons;
        needfulltree = FLAG.du || FLAG.prune || FLAG.matchdirs || FLAG.fromfile || FLAG.condense_singletons;
    }

    // C: emit_tree(dirname, needfulltree);
    crate::list::emit_tree(&mut dirname, needfulltree);

    // C: if (outfilename != NULL) fclose(outfile);
    // process::exit 不运行析构函数，BufWriter 缓冲需显式 flush（等价于 fclose）
    if outfilename.is_some() {
        // unsafe：访问全局 OUTFILE
        unsafe {
            if let Some(w) = OUTFILE.as_mut() {
                let _ = w.flush();
            }
        }
    }

    // C: return errors ? 2 : 0;
    let code = if unsafe { ERRORS } != 0 { 2 } else { 0 };
    std::process::exit(code);
}




