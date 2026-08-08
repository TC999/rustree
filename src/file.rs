// 文件路径：src/file.rs
// 对应 C 源文件：file.c
// 从文件读取目录树（--fromfile/--fromtabfile）：
// file_getfulltree 按"每行一条路径"解析，tabedfile_getfulltree 按制表符缩进
// 表示层级解析；两者都构建链表树后经 fprune 剪枝/排序输出。

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::sync::atomic::{AtomicBool, Ordering};

use crate::filter::{filtercheck, flush_filterstack};
use crate::globals::{FILE_COMMENT, FILE_PATHSEP, FLAG, PATTERN, TOPSORT};
use crate::info::{infocheck, pop_infostack};
use crate::tree::{Info, S_IFDIR, S_IFLNK, S_IFREG};
use crate::util::{is_singleton, pathconcat};
use crate::{patignore, patinclude, push_files};

// C: #define MAXPATH 64*1024（64K 路径上限；Rust 的 String 动态分配，
//     此上限不再需要，保留常量以对应源码）
#[allow(dead_code)]
const MAXPATH: usize = 64 * 1024;

// C: enum ftok { T_PATHSEP, T_DIR, T_FILE, T_EOP };
const T_PATHSEP: i32 = 0;
const T_DIR: i32 = 1;
const T_FILE: i32 = 2;
const T_EOP: i32 = 3;

// 判断字节是否为路径分隔符（C: strchr(file_pathsep, c) != NULL）
fn is_pathsep(b: u8) -> bool {
    // unsafe：读取全局 FILE_PATHSEP
    unsafe { FILE_PATHSEP }.as_bytes().contains(&b)
}

// === 原 C 函数：char *nextpc(char **p, int *tok) ===
/// 路径行解析器：从当前位置读取下一个路径分量。
/// C 中直接修改字节缓冲区（分隔符替换为 '\0'）；Rust 中维护 (data, pos)。
/// C 的 static prev 跨调用状态用全局 AtomicBool 表达（单线程语义一致）。
static PREV: AtomicBool = AtomicBool::new(false);

struct PathParser {
    data: Vec<u8>, // 路径行缓冲（C 的 char* 缓冲区）
    pos: usize, // 当前读取位置（C 的 char** p）
}

/// 返回当前 token 的字符串；tok 输出 token 类型。
/// C 中返回指向缓冲区内的指针，Rust 返回拷贝的 String（调用方随后立即使用）。
fn nextpc(p: &mut PathParser, tok: &mut i32) -> Option<String> {
    let s = p.pos;
    // C: if (!**p) { *tok = T_EOP; return NULL; }
    if p.pos >= p.data.len() {
        *tok = T_EOP;
        return None;
    }
    // C: if (prev) { prev = 0; *tok = T_PATHSEP; return NULL; }
    if PREV.load(Ordering::SeqCst) {
        PREV.store(false, Ordering::SeqCst);
        *tok = T_PATHSEP;
        return None;
    }
    // C: if (strchr(file_pathsep, **p) != NULL) { (*p)++; *tok = T_PATHSEP; return NULL; }
    if is_pathsep(p.data[p.pos]) {
        p.pos += 1;
        *tok = T_PATHSEP;
        return None;
    }
    // C: while (**p && strchr(file_pathsep, **p) == NULL) (*p)++;
    while p.pos < p.data.len() && !is_pathsep(p.data[p.pos]) {
        p.pos += 1;
    }
    // C: if (**p) { *tok = T_DIR; prev = **p; *(*p)++ = '\0'; } else *tok = T_FILE;
    let token_end = p.pos;
    if p.pos < p.data.len() {
        *tok = T_DIR;
        PREV.store(true, Ordering::SeqCst);
        // C: *(*p)++ = '\0'（越过分隔符；Rust 中以位置边界表示截断）
        p.pos += 1;
    } else {
        *tok = T_FILE;
    }
    // C: return s;（token 为 [s, 分隔符) 的内容）
    Some(String::from_utf8_lossy(&p.data[s..token_end]).into_owned())
}

// === 原 C 函数：struct _info *newent(const char *name) ===
fn newent(name: &str) -> Box<Info> {
    // C: xmalloc + memset(0) + name/child/tchild/next 赋值
    Box::new(Info {
        name: name.to_string(),
        ..Info::default()
    })
}

// === 原 C 函数：struct _info *search(struct _info **dir, const char *name) ===
/// 在链表中查找 name；不存在则按序插入新节点。返回指向节点的引用。
/// C 中 dir 为链表头指针的地址，Rust 中为 &mut Option<Box<Info>>。
fn search<'a>(dir: &'a mut Option<Box<Info>>, name: &str) -> &'a mut Info {
    // C: if (*dir == NULL) return (*dir = newent(name));
    if dir.is_none() {
        *dir = Some(newent(name));
        return dir.as_mut().unwrap();
    }

    // C: for(prev = ptr = *dir; ptr != NULL; ptr = ptr->next) { cmp; ... }
    // 用裸指针遍历并定位插入点（对应 C 的 prev/ptr 指针）
    // unsafe：裸指针链表遍历（借用检查器无法表达"插入到前驱之后"）
    unsafe {
        let mut cur: *mut Option<Box<Info>> = dir;
        loop {
            let node: *mut Info = match (*cur).as_mut() {
                None => break,
                Some(n) => &mut **n as *mut Info,
            };
            // C: cmp = strcmp(ptr->name, name); if (cmp == 0) return ptr;
            if (*node).name == name {
                return &mut *node;
            }
            cur = &mut (*node).next;
        }
        // 未找到：插入新节点（cur 指向插入槽位）
        let mut n = newent(name);
        // C: n->next = ptr（ptr 为 NULL）
        n.next = (*cur).take();
        *cur = Some(n);
        // C: if (prev == ptr) *dir = n; else prev->next = n;（cur 已指向 prev 的后继槽）
        (*cur).as_mut().unwrap()
    }
}

// === 原 C 函数：void freefiletree(struct _info *ent) ===
/// 释放整棵文件树（tchild 递归 + next 兄弟链）。
/// Rust 中 drop 自动递归释放所有字段。
fn freefiletree(ent: Option<Box<Info>>) {
    drop(ent);
}

// === 原 C 函数：struct _info **fprune(struct _info *head, const char *path, bool matched, bool root) ===
/// 递归剪枝：应用 -d/-a/-P/-I/gitignore/showinfo 过滤与 -prune/--condense 处理，
/// 将链表树转为排序后的数组（Vec<Info>）。
fn fprune(
    head: Option<Box<Info>>,
    path: &str,
    mut matched: bool,
    root: bool,
) -> Option<Vec<Info>> {
    let mut ig: Option<Box<crate::tree::Ignorefile>> = None;
    let mut inf: Option<Box<crate::tree::Infofile>> = None;

    // C: strcpy(fpath, path); cur = fpath + strlen(fpath); *(cur++) = '/';
    let fpath_base_len = path.len() + 1;
    let mut fpath = format!("{}/", path);

    // C: push_files(path, &ig, &inf, root);
    push_files(path, &mut ig, &mut inf, root);

    // 输出的节点收集（C 的 new/end 链表，最终转数组）
    let mut shown: Vec<Box<Info>> = Vec::new();
    let mut count: usize = 0;
    // C: bool defmatched = matched; int tmp_pattern = 0;
    let defmatched = matched;
    let mut tmp_pattern: i32 = 0;

    // C: for(ent = head; ent != NULL;)
    let mut ent = head;
    while let Some(mut node) = ent {
        // C: t = ent; ent = ent->next;（先取后继，处理完 node 后继续）
        let next = node.next.take();
        // C: strcpy(cur, ent->name);
        fpath.truncate(fpath_base_len);
        fpath.push_str(&node.name);
        // C: if (ent->tchild) ent->isdir = 1;
        if node.tchild.is_some() {
            node.isdir = true;
        }

        // C: show = true;
        let mut show = true;
        // unsafe：读取/修改全局 FLAG 与 PATTERN
        unsafe {
            // C: if (flag.d && !ent->isdir) show = false;
            if FLAG.d && !node.isdir {
                show = false;
            }
            // C: if (!flag.a && ent->name[0] == '.') show = false;
            if !FLAG.a && node.name.starts_with('.') {
                show = false;
            }
            // C: if (show && !matched)
            if show && !matched {
                if !node.isdir {
                    // C: if (pattern && !patinclude(name, isdir, false) && !patinclude(fpath, isdir, true)) show = false;
                    if PATTERN != 0
                        && patinclude(&node.name, node.isdir, false) == 0
                        && patinclude(&fpath, node.isdir, true) == 0
                    {
                        show = false;
                    }
                    // C: if (ipattern && (patignore(...) || patignore(...))) show = false;
                    if crate::globals::IPATTERN != 0
                        && (patignore(&node.name, node.isdir, false) == 1
                            || patignore(&fpath, node.isdir, true) == 1)
                    {
                        show = false;
                    }
                } else {
                    // C: if (pattern && (patinclude(...) || patinclude(...))) {
                    //       show = true; matched = true; tmp_pattern = pattern; pattern = 0; }
                    if PATTERN != 0
                        && (patinclude(&node.name, node.isdir, false) == 1
                            || patinclude(&fpath, node.isdir, true) == 1)
                    {
                        show = true;
                        matched = true;
                        tmp_pattern = PATTERN;
                        PATTERN = 0;
                    }
                    // C: if (ipattern && (patignore(...) || patignore(...))) show = false;
                    if crate::globals::IPATTERN != 0
                        && (patignore(&node.name, node.isdir, false) == 1
                            || patignore(&fpath, node.isdir, true) == 1)
                    {
                        show = false;
                    }
                }
            }
            // C: if (flag.gitignore && filtercheck(path, ent->name, ent->isdir)) show = false;
            if FLAG.gitignore && filtercheck(path, &node.name, node.isdir as i32) {
                show = false;
            }
        }

        // C: if (show && flag.showinfo && (com = infocheck(path, ent->name, inf != NULL, ent->isdir)))
        //      拷贝 com->desc 到 ent->comment
        if show && unsafe { FLAG.showinfo } {
            if let Some(com) = infocheck(path, &node.name, if inf.is_some() { 1 } else { 0 }, node.isdir) {
                node.comment = com.desc.clone();
            }
        }

        // C: if (show && ent->tchild != NULL) ent->child = fprune(ent->tchild, fpath, matched, false);
        if show && node.tchild.is_some() {
            node.child = fprune(node.tchild.take(), &fpath, matched, false);
        }

        // C: if (flag.prune && !matched && ent->isdir && ent->child == NULL) {
        //       ent->tchild = NULL; show = false; }
        if unsafe { FLAG.prune } && !matched && node.isdir && node.child.is_none() {
            node.tchild = None;
            show = false;
        }

        // C: if (flag.condense_singletons) { while (is_singleton(ent)) { ... } }
        if unsafe { FLAG.condense_singletons } {
            while is_singleton(&node) {
                // C: child = ent->child（is_singleton 保证恰有一个子项）
                let child: Vec<Info> = node.child.take().expect("is_singleton 要求 child 非空");
                // C: name = pathconcat(ent->name, child[0]->name, NULL);
                let name = pathconcat(&node.name, &[&child[0].name]);
                node.name = name;
                // C: ent->child = child[0]->child;
                let mut child0 = child.into_iter().next().unwrap();
                node.child = child0.child.take();
                // C: ent->condensed = ent->condensed + 1 + child[0]->condensed;
                node.condensed = node.condensed + 1 + child0.condensed;
                // C: free_dir(child);（drop 自动释放）
            }
        }

        // C: if (tmp_pattern) { pattern = tmp_pattern; tmp_pattern = 0; }
        if tmp_pattern != 0 {
            // unsafe：恢复全局 PATTERN
            unsafe {
                PATTERN = tmp_pattern;
            }
            tmp_pattern = 0;
        }
        // C: matched = defmatched;
        matched = defmatched;

        // C: t = ent; ent = ent->next;
        if show {
            shown.push(node);
            count += 1;
        } else {
            // C: t->next = NULL; freefiletree(t);
            drop(node);
        }
        ent = next;
    }

    // C: if (count > 0) { 构建数组并排序 }
    let mut dir: Option<Vec<Info>> = None;
    if count > 0 {
        let mut dir_vec: Vec<Info> = shown.into_iter().map(|b| *b).collect();
        // C: if (topsort && count > 1) qsort(...)（qsort 不稳定 → sort_unstable_by）
        if let Some(f) = unsafe { TOPSORT } {
            if dir_vec.len() > 1 {
                dir_vec.sort_unstable_by(|a, b| f(a, b).cmp(&0));
            }
        }
        dir = Some(dir_vec);
    }

    // C: if (ig != NULL) ig = flush_filterstack();
    if ig.is_some() {
        flush_filterstack();
    }
    // C: if (inf != NULL) inf = pop_infostack();
    if inf.is_some() {
        pop_infostack();
    }
    // C: free(fpath);（drop 自动）

    dir
}

// === 原 C 函数：struct _info **file_getfulltree(char *d, u_long lev, dev_t dev, off_t *size, char **err) ===
/// 从文件读取目录树：每行一个完整路径（可含 " -> " 链接目标）。
pub fn file_getfulltree(
    d: &str,
    _lev: u64,
    _dev: u64,
    size: &mut i64,
    _err: &mut Option<String>,
) -> Option<Vec<Info>> {
    // C: FILE *fp = (strcmp(d, ".") ? fopen(d, "r") : stdin);
    let is_stdin = d == ".";
    let fp: Option<Box<dyn BufRead>> = if is_stdin {
        Some(Box::new(BufReader::new(std::io::stdin())))
    } else {
        match File::open(d) {
            Ok(f) => Some(Box::new(BufReader::new(f))),
            Err(_) => None,
        }
    };
    // C: *size = 0;
    *size = 0;
    // C: if (fp == NULL) { fprintf(stderr, "tree: Error opening %s for reading.\n", d); return NULL; }
    let mut reader = match fp {
        Some(r) => r,
        None => {
            eprintln!("tree: Error opening {} for reading.", d);
            return None;
        }
    };

    let mut root: Option<Box<Info>> = None;
    let mut line = String::new();
    loop {
        line.clear();
        // C: while (fgets(path, MAXPATH, fp) != NULL)（Rust String 无 MAXPATH 上限）
        let n = reader.read_line(&mut line).unwrap_or(0);
        if n == 0 {
            break;
        }
        // C: if (file_comment != NULL && strncmp(path, file_comment, strlen(file_comment)) == 0) continue;
        if line.starts_with(unsafe { FILE_COMMENT }) {
            continue;
        }
        // C: l = strlen(path); while (l && (path[l-1]=='\n' || path[l-1]=='\r')) path[--l] = '\0';
        while line.ends_with('\n') || line.ends_with('\r') {
            line.pop();
        }
        // C: if (l == 0) continue;
        if line.is_empty() {
            continue;
        }

        // C: spath = path; cwd = &root;
        let mut parser = PathParser {
            data: line.as_bytes().to_vec(),
            pos: 0,
        };
        let mut cwd: &mut Option<Box<Info>> = &mut root;

        // C: link = flag.fflinks ? strstr(path, " -> ") : NULL;
        let mut link: Option<String> = None;
        // unsafe：读取全局 FLAG
        if unsafe { FLAG.fflinks } {
            if let Some(pos) = line.find(" -> ") {
                // C: *link = '\0'; link += 4;
                link = Some(line[pos + 4..].to_string());
                parser.data.truncate(pos);
            }
        }

        // C: ent = NULL; do { s = nextpc(&spath, &tok); ... } while (tok != T_FILE && tok != T_EOP);
        // 用裸指针记录"最后一个处理的节点"（C 的 ent，供链接目标赋值）
        // unsafe：ent 裸指针指向 search 返回的节点（由 cwd 链持有）
        let mut ent: *mut Info = std::ptr::null_mut();
        let mut tok: i32 = 0;
        loop {
            let s = nextpc(&mut parser, &mut tok);
            match tok {
                T_PATHSEP => continue,
                T_FILE | T_DIR => {
                    let s = s.expect("T_FILE/T_DIR 时 nextpc 返回 Some");
                    // C: if (strcmp(s, ".") == 0) continue;
                    if s == "." {
                        continue;
                    }
                    // C: ent = search(cwd, s);
                    let e = search(cwd, &s);
                    // C: if (tok == T_DIR) { ent->isdir = 1; ent->mode = S_IFDIR; } else { ent->mode = S_IFREG; }
                    if tok == T_DIR {
                        e.isdir = true;
                        e.mode = S_IFDIR;
                    } else {
                        e.mode = S_IFREG;
                    }
                    ent = e as *mut Info;
                    // C: cwd = &(ent->tchild);
                    cwd = &mut e.tchild;
                }
                _ => break, // T_EOP
            }
            if tok == T_FILE || tok == T_EOP {
                break;
            }
        }

        // C: if (ent && link) { ent->isdir = 0; ent->mode = S_IFLNK; ent->lnk = scopy(link); }
        if !ent.is_null() {
            if let Some(link_str) = link {
                // unsafe：解引用 ent 裸指针
                unsafe {
                    (*ent).isdir = false;
                    (*ent).mode = S_IFLNK;
                    (*ent).lnk = Some(link_str);
                }
            }
        }
    }
    // C: if (fp != stdin) fclose(fp); free(path);（drop 自动）

    // C: return fprune(root, "", false, true);
    fprune(root, "", false, true)
}

// === 原 C 函数：struct _info **tabedfile_getfulltree(...) ===
/// 从文件读取目录树：制表符缩进表示层级（--fromtabfile）。
pub fn tabedfile_getfulltree(
    d: &str,
    _lev: u64,
    _dev: u64,
    size: &mut i64,
    _err: &mut Option<String>,
) -> Option<Vec<Info>> {
    // C: FILE *fp = (strcmp(d, ".") ? fopen(d, "r") : stdin);
    let is_stdin = d == ".";
    let fp: Option<Box<dyn BufRead>> = if is_stdin {
        Some(Box::new(BufReader::new(std::io::stdin())))
    } else {
        match File::open(d) {
            Ok(f) => Some(Box::new(BufReader::new(f))),
            Err(_) => None,
        }
    };
    *size = 0;
    let mut reader = match fp {
        Some(r) => r,
        None => {
            eprintln!("tree: Error opening {} for reading.", d);
            return None;
        }
    };

    let mut root: Option<Box<Info>> = None;
    // C: struct _info *istack[maxstack]（裸指针数组，对应 C 的指针栈）
    const MAXSTACK: usize = 2048;
    let mut istack: Vec<*mut Info> = vec![std::ptr::null_mut(); MAXSTACK];
    let mut line: usize = 0;
    let mut top: usize = 0;

    let mut buf = String::new();
    loop {
        buf.clear();
        let n = reader.read_line(&mut buf).unwrap_or(0);
        if n == 0 {
            break;
        }
        // C: line++;
        line += 1;
        if buf.starts_with(unsafe { FILE_COMMENT }) {
            continue;
        }
        while buf.ends_with('\n') || buf.ends_with('\r') {
            buf.pop();
        }
        if buf.is_empty() {
            continue;
        }

        // C: for(tabs=0; path[tabs] == '\t'; tabs++);
        let tabs = buf.as_bytes().iter().take_while(|&&b| b == b'\t').count();
        // C: if (tabs >= maxstack) { 错误并继续 }
        if tabs >= MAXSTACK {
            eprintln!(
                "tree: Tab depth exceeds maximum path depth ({} >= {}) on line {}",
                tabs, MAXSTACK, line
            );
            continue;
        }

        // C: spath = path + tabs;
        let spath = &buf[tabs..];

        // C: link = flag.fflinks ? strstr(spath, " -> ") : NULL;
        let mut link: Option<String> = None;
        let spath_no_link: String = if unsafe { FLAG.fflinks } {
            if let Some(pos) = spath.find(" -> ") {
                // C: *link = '\0'; link += 4;
                link = Some(spath[pos + 4..].to_string());
                spath[..pos].to_string()
            } else {
                spath.to_string()
            }
        } else {
            spath.to_string()
        };

        // C: if (tabs > 0 && ((tabs-1 > top) || (istack[tabs-1] == NULL))) { 孤儿错误并继续 }
        if tabs > 0 && (tabs - 1 > top || istack[tabs - 1].is_null()) {
            eprintln!(
                "tree: Orphaned file [{}] on line {}, check tab depth in file.",
                spath, line
            );
            continue;
        }

        // C: ent = istack[tabs] = search(tabs? &(istack[tabs-1]->tchild) : &root, spath);
        // unsafe：istack 为裸指针数组，解引用以访问其 tchild
        let ent: *mut Info = unsafe {
            if tabs > 0 {
                let node = &mut *istack[tabs - 1];
                let e = search(&mut node.tchild, &spath_no_link);
                let p = e as *mut Info;
                istack[tabs] = p;
                p
            } else {
                let e = search(&mut root, &spath_no_link);
                let p = e as *mut Info;
                istack[tabs] = p;
                p
            }
        };
        // C: ent->mode = S_IFREG;
        // unsafe：解引用 ent
        unsafe {
            (*ent).mode = S_IFREG;
        }
        // C: if (tabs) { istack[tabs-1]->isdir = 1; istack[tabs-1]->mode = S_IFDIR; }
        if tabs > 0 {
            // unsafe：解引用 istack[tabs-1]
            unsafe {
                (*istack[tabs - 1]).isdir = true;
                (*istack[tabs - 1]).mode = S_IFDIR;
            }
        }
        // C: if (link) { ent->isdir = 0; ent->mode = S_IFLNK; ent->lnk = scopy(link); }
        if let Some(link_str) = link {
            // unsafe：解引用 ent
            unsafe {
                (*ent).isdir = false;
                (*ent).mode = S_IFLNK;
                (*ent).lnk = Some(link_str);
            }
        }
        // C: top = tabs;
        top = tabs;
    }
    // C: if (fp != stdin) fclose(fp); free(path); free(istack);（drop 自动）

    // C: return fprune(root, "", false, true);
    fprune(root, "", false, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nextpc_basic() {
        // 重置跨调用状态（C 的 static prev）
        PREV.store(false, Ordering::SeqCst);
        let mut parser = PathParser {
            data: b"a/b/c".to_vec(),
            pos: 0,
        };
        let mut tok: i32 = 0;
        let s = nextpc(&mut parser, &mut tok).unwrap();
        assert_eq!((s.as_str(), tok), ("a", T_DIR));
        let s = nextpc(&mut parser, &mut tok);
        assert_eq!((s, tok), (None, T_PATHSEP));
        let s = nextpc(&mut parser, &mut tok).unwrap();
        assert_eq!((s.as_str(), tok), ("b", T_DIR));
        let s = nextpc(&mut parser, &mut tok);
        assert_eq!((s, tok), (None, T_PATHSEP));
        let s = nextpc(&mut parser, &mut tok).unwrap();
        assert_eq!((s.as_str(), tok), ("c", T_FILE));
    }

    #[test]
    fn test_nextpc_consecutive_seps() {
        PREV.store(false, Ordering::SeqCst);
        let mut parser = PathParser {
            data: b"a//b".to_vec(),
            pos: 0,
        };
        let mut tok: i32 = 0;
        assert_eq!((nextpc(&mut parser, &mut tok).unwrap().as_str(), tok), ("a", T_DIR));
        // prev 置位 → T_PATHSEP
        assert_eq!((nextpc(&mut parser, &mut tok), tok), (None, T_PATHSEP));
        // 第二个 '/' → T_PATHSEP
        assert_eq!((nextpc(&mut parser, &mut tok), tok), (None, T_PATHSEP));
        assert_eq!((nextpc(&mut parser, &mut tok).unwrap().as_str(), tok), ("b", T_FILE));
    }

    #[test]
    fn test_search_and_tree() {
        let mut root: Option<Box<Info>> = None;
        // 插入 a、b，重复 a
        let ea = search(&mut root, "a");
        ea.isdir = true;
        let eb = search(&mut root, "b");
        eb.mode = S_IFREG;
        let ea2 = search(&mut root, "a");
        assert_eq!(ea2.name, "a");

        let mut list = root.as_deref();
        let mut names = Vec::new();
        while let Some(n) = list {
            names.push(n.name.clone());
            list = n.next.as_deref();
        }
        assert_eq!(names, vec!["a", "b"]);
        // a 是目录（含 tchild？不，这里没设 tchild）
        assert!(root.as_deref().unwrap().isdir);
    }

    #[test]
    fn test_file_getfulltree() {
        // 构造临时输入文件：a/b/c.txt 与 a/b.txt（-F 链接测试留空）
        let tmp = std::env::temp_dir().join(format!(
            "rustree_file_test_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::write(&tmp, "a/b/c.txt\na/d.txt\n").unwrap();
        let path = tmp.to_string_lossy().into_owned();

        // unsafe：测试中初始化全局状态
        unsafe {
            FLAG = crate::tree::Flags::new();
        }
        let mut size: i64 = 0;
        let mut err: Option<String> = None;
        let dir = file_getfulltree(&path, 0, 0, &mut size, &mut err).expect("解析成功");
        // a 是目录（含子项 b 和 d.txt）
        let a = &dir[0];
        assert!(a.isdir);
        assert_eq!(a.name, "a");
        let children = a.child.as_ref().expect("a 有子项");
        // b（目录）和 d.txt（文件）
        assert_eq!(children.len(), 2);
        let b = &children[0];
        assert!(b.isdir);
        assert_eq!(b.name, "b");
        assert_eq!(b.child.as_ref().unwrap()[0].name, "c.txt");
        assert!(!children[1].isdir);
        assert_eq!(children[1].name, "d.txt");

        std::fs::remove_file(&tmp).ok();
    }

    #[test]
    fn test_tabedfile_getfulltree() {
        let tmp = std::env::temp_dir().join(format!(
            "rustree_tab_test_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        // 制表符缩进层级
        std::fs::write(&tmp, "root\n\tchild1\n\tchild2\n\t\tgrand\n").unwrap();
        let path = tmp.to_string_lossy().into_owned();

        // unsafe：测试中初始化全局状态
        unsafe {
            FLAG = crate::tree::Flags::new();
        }
        let mut size: i64 = 0;
        let mut err: Option<String> = None;
        let dir = tabedfile_getfulltree(&path, 0, 0, &mut size, &mut err).expect("解析成功");
        assert_eq!(dir.len(), 1);
        let root = &dir[0];
        assert_eq!(root.name, "root");
        assert!(root.isdir);
        let children = root.child.as_ref().unwrap();
        assert_eq!(children.len(), 2);
        // child1 无子项 → C 中 isdir=false（tab 缩进只标记父为目录）
        assert_eq!(children[0].name, "child1");
        assert!(!children[0].isdir);
        // child2 有子项 grand → isdir=true
        assert_eq!(children[1].name, "child2");
        assert!(children[1].isdir);
        assert_eq!(children[1].child.as_ref().unwrap()[0].name, "grand");

        std::fs::remove_file(&tmp).ok();
    }
}
