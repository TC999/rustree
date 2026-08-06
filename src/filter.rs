// 文件路径：src/filter.rs
// 对应 C 源文件：filter.c
// gitignore 过滤：过滤栈（filterstack）、gitignore 文件解析与向上搜索、
// 以及 remove/reverse 双层模式匹配（filtercheck）。

use std::fs::File;
use std::io::{BufRead, BufReader};

use crate::patmatch;
use crate::tree::{
    stat_fields, Ignorefile, Pattern, S_IFDIR, S_IFMT, S_IFREG,
};

// C: struct ignorefile *filterstack = NULL;
// gitignore 过滤栈（链表，最新压入的在栈顶，filtercheck 从栈顶开始匹配）
static mut FILTERSTACK: Option<Box<Ignorefile>> = None;

// === 原 C 函数：void gittrim(char *s) ===
/// 修剪 gitignore 行：去掉末尾换行与未转义空格，并删除转义用的反斜杠。
/// C 按字节原地修改；Rust 中重建 String（反斜杠删除涉及字节级搬移）。
/// 注意：忠实保留 C 的边界行为（如整行为单个 '\n' 时因 e>0 条件而不被剔除）。
fn gittrim(s: &mut String) {
    let mut bytes = s.as_bytes().to_vec();
    // C: ssize_t i, e = (ssize_t)strlen(s)-1; if (e < 0) return;
    if bytes.is_empty() {
        return;
    }
    let mut e = bytes.len() - 1;

    // C: while (e > 0 && (s[e] == '\n' || s[e] == '\r')) e--;
    while e > 0 && (bytes[e] == b'\n' || bytes[e] == b'\r') {
        e -= 1;
    }

    // C: for(i = e; i >= 0; i--) {
    //      if (s[i] != ' ') break;
    //      if (i && s[i-1] != '\\') e--;
    //    }
    let mut i = e as i64;
    loop {
        if i < 0 {
            break;
        }
        if bytes[i as usize] != b' ' {
            break;
        }
        if i > 0 && bytes[(i - 1) as usize] != b'\\' {
            e -= 1;
        }
        i -= 1;
    }

    // C: s[e+1] = '\0';（截断）
    bytes.truncate(e + 1);

    // C: for(i = e = 0; s[i] != '\0';) { if (s[i] == '\\') i++; s[e++] = s[i++]; }
    //     s[e] = '\0';
    // 删除未转义的反斜杠（反斜杠后的字符保留）。C 中若反斜杠为末字符，
    // 会复制 '\0' 并越界读取；Rust 以安全方式处理为"丢弃反斜杠"。
    let mut out = Vec::with_capacity(bytes.len());
    let mut j = 0;
    while j < bytes.len() {
        if bytes[j] == b'\\' {
            j += 1;
            if j >= bytes.len() {
                break;
            }
        }
        out.push(bytes[j]);
        j += 1;
    }
    *s = String::from_utf8(out)
        .expect("gittrim 仅删除 ASCII 反斜杠，输入为合法 UTF-8");
}

// === 原 C 函数：struct pattern *new_pattern(char *pattern) ===
/// 创建过滤模式节点。
/// 去掉开头的 '/'；relative 为真表示模式不含 '/' 或 '/' 为末字符。
pub fn new_pattern(pattern: &str) -> Box<Pattern> {
    // C: p->pattern = scopy(pattern + ((pattern[0] == '/')? 1 : 0));
    let pat = pattern.strip_prefix('/').unwrap_or(pattern);
    // C: sl = strchr(pattern, '/');
    //    p->relative = (sl == NULL || (sl && !*(sl+1)));
    let relative = match pattern.find('/') {
        None => 1,
        Some(pos) => {
            if pos + 1 >= pattern.len() {
                1
            } else {
                0
            }
        }
    };
    Box::new(Pattern {
        pattern: pat.to_string(),
        relative,
        next: None,
    })
}

// C: is_file(path) —— stat 后判断是否为普通文件（S_ISREG）
fn is_file(path: &str) -> bool {
    match stat_fields(path) {
        Err(_) => false,
        Ok(st) => (st.mode & S_IFMT) == S_IFREG,
    }
}

// C: is_dir(path) —— stat 后判断是否为目录（S_ISDIR）
fn is_dir(path: &str) -> bool {
    match stat_fields(path) {
        Err(_) => false,
        Ok(st) => (st.mode & S_IFMT) == S_IFDIR,
    }
}

// === 原 C 函数：struct ignorefile *gitignore_search(const char *startpath, int depth) ===
/// 沿目录树向上搜索 .gitignore 文件，遇到含 .git 目录处（视为 git 根）停止。
/// depth 仅为防止无限循环的保险（上限 2048）。
/// C 中返回最后压栈的 ignorefile 指针；Rust 中返回对应 Box 的副本
/// （压栈的为克隆，见 push_filterstack 的说明）。
pub fn gitignore_search(startpath: &str, depth: i32) -> Option<Box<Ignorefile>> {
    let mut pign: Option<Box<Ignorefile>> = None;
    let mut ign: Option<Box<Ignorefile>> = None;

    // C: snprintf(path, PATH_MAX, "%.*s/.git", PATH_MAX-6, startpath);
    // 检测 startpath/.git 是否为目录（Rust 的 String 无长度上限，省略精度截断）
    let mut path = format!("{}/.git", startpath);
    if is_dir(&path) {
        // 到达 git 根：加载 .git/config/exclude
        // C: snprintf(path, PATH_MAX, "%.*s/.git/info/exclude", PATH_MAX-21, startpath);
        path = format!("{}/.git/info/exclude", startpath);
        if is_file(&path) {
            let new_ig = new_ignorefile(startpath, &path, false);
            push_filterstack(new_ig.clone());
            pign = new_ig;
        }
    } else {
        // C: if (realpath(startpath, path) == NULL) return NULL;
        let real = std::fs::canonicalize(startpath);
        match real {
            Err(_) => return None,
            Ok(rp) => {
                let rp = rp.to_string_lossy();
                // C: if (strcmp(path, "/") != 0 && depth < 2048) —— 未到根则继续向上
                if rp != "/" && depth < 2048 {
                    // C: snprintf(path, "%.*s/..", PATH_MAX-4, startpath);
                    path = format!("{}/..", startpath);
                    pign = gitignore_search(&path, depth + 1);
                }
            }
        }
    }

    // C: snprintf(path, PATH_MAX, "%.*s/.gitignore", PATH_MAX-12, startpath);
    path = format!("{}/.gitignore", startpath);
    if is_file(&path) {
        let new_ig = new_ignorefile(startpath, &path, false);
        push_filterstack(new_ig.clone());
        ign = new_ig;
    }

    // C: return ign == NULL? pign : ign;
    if ign.is_some() {
        ign
    } else {
        pign
    }
}

// === 原 C 函数：struct ignorefile *new_ignorefile(const char *basepath, const char *path, bool checkparents) ===
/// 解析一个 gitignore 文件（或 path/.gitignore），返回 Ignorefile。
/// remove 为普通模式链表、reverse 为以 '!' 开头的反向模式链表。
pub fn new_ignorefile(
    basepath: &str,
    path: &str,
    checkparents: bool,
) -> Option<Box<Ignorefile>> {
    let mut buf = String::new();

    // C: if (!is_file(path)) { snprintf(buf, "%s/.gitignore", path); fp = fopen(buf, "r"); ... }
    //     else fp = fopen(path, "r");
    let fp: Option<File> = if !is_file(path) {
        buf = format!("{}/.gitignore", path);
        match File::open(&buf) {
            Ok(f) => Some(f),
            Err(_) => {
                // C: if (fp == NULL && checkparents) return gitignore_search(path, 0);
                if checkparents {
                    return gitignore_search(path, 0);
                }
                None
            }
        }
    } else {
        File::open(path).ok()
    };
    // C: if (fp == NULL) return NULL;
    let fp = fp?;

    // C: remove/reverse 模式链表。C 用尾指针 remend/revend 原地构建（O(1)）；
    //    这里先用 Vec 收集（顺序不变），最后反向串联成链表（O(n)），
    //    链表内容与顺序与 C 等价。
    let mut remove_vec: Vec<Pattern> = Vec::new();
    let mut reverse_vec: Vec<Pattern> = Vec::new();

    // C: while (fgets(buf, PATH_MAX, fp) != NULL)
    // read_line 保留行尾 '\n'，与 fgets 行为一致（gittrim 依赖它）
    let mut reader = BufReader::new(fp);
    loop {
        buf.clear();
        let n = reader.read_line(&mut buf).unwrap_or(0);
        if n == 0 {
            break;
        }
        // C: if (buf[0] == '#') continue;
        if buf.starts_with('#') {
            continue;
        }
        // C: rev = (buf[0] == '!');
        let rev = buf.starts_with('!');
        gittrim(&mut buf);
        // C: if (strlen(buf) == 0) continue;
        if buf.is_empty() {
            continue;
        }
        // C: p = new_pattern(buf + (rev? 1 : 0));
        let pat_str = if rev { &buf[1..] } else { &buf[..] };
        let p = new_pattern(pat_str);
        if rev {
            // C 中压入的是 Box 指针；这里解包成值收集，构建链表时再装箱
            reverse_vec.push(*p);
        } else {
            remove_vec.push(*p);
        }
    }

    // 将 Vec 反向串联为链表（保持原始顺序）
    fn link_patterns(mut vec: Vec<Pattern>) -> Option<Box<Pattern>> {
        let mut head: Option<Box<Pattern>> = None;
        while let Some(p) = vec.pop() {
            head = Some(Box::new(Pattern {
                pattern: p.pattern,
                relative: p.relative,
                next: head,
            }));
        }
        head
    }
    let remove = link_patterns(remove_vec);
    let reverse = link_patterns(reverse_vec);

    // C: ig = xmalloc(...); ig->remove = remove; ig->reverse = reverse;
    //     ig->path = scopy(basepath); ig->next = NULL;
    Some(Box::new(Ignorefile {
        path: basepath.to_string(),
        remove,
        reverse,
        next: None,
    }))
}

// === 原 C 函数：void push_filterstack(struct ignorefile *ig) ===
/// 将 ignorefile 压入过滤栈顶。
/// C 中压入原指针；Rust 中接收 Option<Box<Ignorefile>>（None 时直接返回）。
pub fn push_filterstack(ig: Option<Box<Ignorefile>>) {
    // C: if (ig == NULL) return;
    let mut ig = match ig {
        Some(b) => b,
        None => return,
    };
    // C: ig->next = filterstack; filterstack = ig;
    // unsafe：访问全局过滤栈 FILTERSTACK
    unsafe {
        ig.next = FILTERSTACK.take();
        FILTERSTACK = Some(ig);
    }
}

// === 原 C 函数：struct ignorefile *pop_filterstack(void) ===
/// 弹出并释放过滤栈顶（C 中 free 各 pattern 与节点本身；
/// Rust 中 drop 自动释放）。C 恒返回 NULL，故 Rust 返回 ()。
pub fn pop_filterstack() {
    // unsafe：访问全局过滤栈 FILTERSTACK
    unsafe {
        // C: ig = filterstack; if (ig == NULL) return NULL;
        let top = match FILTERSTACK.take() {
            Some(t) => t,
            None => return,
        };
        // C: filterstack = filterstack->next;（旧栈顶随 drop 释放）
        FILTERSTACK = top.next;
    }
}

// === 原 C 函数：struct ignorefile *flush_filterstack(void) ===
/// 清空整个过滤栈。C 恒返回 NULL，故 Rust 返回 ()。
pub fn flush_filterstack() {
    // C: while (filterstack != NULL) pop_filterstack();
    // unsafe：访问全局过滤栈 FILTERSTACK
    unsafe {
        while FILTERSTACK.is_some() {
            FILTERSTACK = FILTERSTACK.take().unwrap().next;
        }
    }
}

// === 原 C 函数：bool filtercheck(const char *path, const char *name, int isdir) ===
/// 若 remove 模式命中且无 reverse 模式命中则返回 true（应过滤掉）。
/// 第一遍检查 remove 链：任一命中即 filter=true；
/// 第二遍检查 reverse 链：任一命中则立即返回 false（放行）。
pub fn filtercheck(path: &str, name: &str, isdir: i32) -> bool {
    let isdir_b = isdir != 0;
    let mut filter = false;

    // C: for(ig = filterstack; !filter && ig; ig = ig->next) { ... remove 链 ... }
    // unsafe：遍历全局过滤栈 FILTERSTACK
    unsafe {
        let mut ig = FILTERSTACK.as_deref();
        while !filter {
            let ig_node = match ig {
                Some(n) => n,
                None => break,
            };
            // C: int fpos = sprintf(xpattern, "%s/", ig->path);
            let mut xpattern = format!("{}/", ig_node.path);
            let fpos = xpattern.len();

            let mut p = ig_node.remove.as_deref();
            while let Some(pat) = p {
                if pat.relative != 0 {
                    // C: if (patmatch(name, p->pattern, isdir) == 1) { filter = true; break; }
                    if patmatch(name.as_bytes(), pat.pattern.as_bytes(), isdir_b) == 1 {
                        filter = true;
                        break;
                    }
                } else {
                    // C: sprintf(xpattern + fpos, "%s", p->pattern);
                    xpattern.push_str(&pat.pattern);
                    // C: if (patmatch(path, xpattern, isdir) == 1) { filter = true; break; }
                    if patmatch(path.as_bytes(), xpattern.as_bytes(), isdir_b) == 1 {
                        filter = true;
                        break;
                    }
                    xpattern.truncate(fpos);
                }
                p = pat.next.as_deref();
            }
            ig = ig_node.next.as_deref();
        }
    }
    // C: if (!filter) return false;
    if !filter {
        return false;
    }

    // C: for(ig = filterstack; ig; ig = ig->next) { ... reverse 链 ... }
    // unsafe：遍历全局过滤栈 FILTERSTACK
    unsafe {
        let mut ig = FILTERSTACK.as_deref();
        while let Some(ig_node) = ig {
            // C: int fpos = sprintf(xpattern, "%s/", ig->path);
            let mut xpattern = format!("{}/", ig_node.path);
            let fpos = xpattern.len();

            let mut p = ig_node.reverse.as_deref();
            while let Some(pat) = p {
                if pat.relative != 0 {
                    // C: if (patmatch(name, p->pattern, isdir) == 1) return false;
                    if patmatch(name.as_bytes(), pat.pattern.as_bytes(), isdir_b) == 1 {
                        return false;
                    }
                } else {
                    // C: sprintf(xpattern + fpos, "%s", p->pattern);
                    xpattern.push_str(&pat.pattern);
                    // C: if (patmatch(path, xpattern, isdir) == 1) return false;
                    if patmatch(path.as_bytes(), xpattern.as_bytes(), isdir_b) == 1 {
                        return false;
                    }
                    xpattern.truncate(fpos);
                }
                p = pat.next.as_deref();
            }
            ig = ig_node.next.as_deref();
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::{Flags, S_IFDIR, S_IFREG};

    // 测试辅助：用模式构建过滤栈（绕过文件系统，直接构造 Pattern/Ignorefile）
    fn push_patterns(remove: &[(&str, i32)], reverse: &[(&str, i32)]) {
        fn link(patterns: &[(&str, i32)]) -> Option<Box<Pattern>> {
            let mut head: Option<Box<Pattern>> = None;
            for (pat, rel) in patterns.iter().rev() {
                head = Some(Box::new(Pattern {
                    pattern: pat.to_string(),
                    relative: *rel,
                    next: head,
                }));
            }
            head
        }
        let ig = Ignorefile {
            path: "/tmp".to_string(),
            remove: link(remove),
            reverse: link(reverse),
            next: None,
        };
        push_filterstack(Some(Box::new(ig)));
    }

    fn clear_stack() {
        flush_filterstack();
    }

    #[test]
    fn test_gittrim() {
        let mut s = "abc\n".to_string();
        gittrim(&mut s);
        assert_eq!(s, "abc");

        let mut s = "abc\r\n".to_string();
        gittrim(&mut s);
        assert_eq!(s, "abc");

        // 尾部空格剔除
        let mut s = "abc   \n".to_string();
        gittrim(&mut s);
        assert_eq!(s, "abc");

        // 转义反斜杠删除（gitignore 中 "\ " 表示字面空格）
        let mut s = "a\\ b\n".to_string();
        gittrim(&mut s);
        assert_eq!(s, "a b");

        let mut s = "".to_string();
        gittrim(&mut s);
        assert_eq!(s, "");
    }

    #[test]
    fn test_new_pattern() {
        let p = new_pattern("*.o");
        assert_eq!(p.pattern, "*.o");
        assert_eq!(p.relative, 1);

        // 开头的 '/' 被剥离；relative 在原始模式上判定（strchr 找到位置 0 的 '/'，
        // 其后是 'b' 而非 '\0'）→ 非相对模式
        let p = new_pattern("/build");
        assert_eq!(p.pattern, "build");
        assert_eq!(p.relative, 0);

        // 含内部 '/' → 非相对模式
        let p = new_pattern("src/*.c");
        assert_eq!(p.pattern, "src/*.c");
        assert_eq!(p.relative, 0);
    }

    #[test]
    fn test_filtercheck_basic() {
        clear_stack();
        // 相对模式：按名字匹配
        push_patterns(&[("*.log", 1)], &[]);
        assert!(filtercheck("/tmp", "a.log", 0));
        assert!(!filtercheck("/tmp", "a.txt", 0));
        clear_stack();

        // 绝对模式（含内部 '/'，relative=0）：按 ig.path + '/' + 模式 匹配完整路径
        push_patterns(&[("sub/*.o", 0)], &[]);
        assert!(filtercheck("/tmp/sub/a.o", "a.o", 0));
        assert!(!filtercheck("/tmp/other/a.o", "a.o", 0));
        clear_stack();
    }

    #[test]
    fn test_filtercheck_reverse() {
        clear_stack();
        // 先过滤所有 .o，再用 ! 保留 keep.o
        push_patterns(&[("*.o", 1)], &[("keep.o", 1)]);
        assert!(filtercheck("/tmp", "drop.o", 0));
        assert!(!filtercheck("/tmp", "keep.o", 0));
        assert!(!filtercheck("/tmp", "a.txt", 0));
        clear_stack();
    }

    #[test]
    fn test_filtercheck_dir_only() {
        clear_stack();
        // 目录专用的模式（结尾 '/' 与 isdir 交互）
        push_patterns(&[("node_modules/", 1)], &[]);
        assert!(filtercheck("/tmp/node_modules", "node_modules", 1));
        clear_stack();
    }

    // 让 FLAG 常量在测试中被引用，避免未使用警告
    #[test]
    fn test_flags_accessible() {
        let _f: Flags = Flags::new();
        let _m = S_IFDIR;
        let _r = S_IFREG;
        assert!(true);
    }
}
