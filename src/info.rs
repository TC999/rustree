// 文件路径：src/info.rs
// 对应 C 源文件：info.c
// .info 注释文件支持：解析（new_infofile）、信息栈（push/pop_infostack）、
// 路径匹配（infocheck）与注释行打印（printcomment）。
//
// .info 文件格式（原 C 注释）：
//   # 注释行
//   pattern
//   pattern
//   	info 消息
//   	更多 info

use std::fs::File;
use std::io::{BufRead, BufReader};

use crate::filter::{gittrim, new_pattern};
use crate::out;
use crate::outbytes;
use crate::patmatch;
use crate::tree::{stat_fields, Comment, Infofile, Pattern, S_IFMT, S_IFREG};

// C: struct infofile *infostack = NULL;
// .info 文件栈（链表，最新压入的在栈顶）
static mut INFOSTACK: Option<Box<Infofile>> = None;

// C: is_file 的变体：stat 后判断 path 是否为普通文件（对应 new_infofile 中的
//    i = stat(path, &st); if (i < 0 || !S_ISREG(st.st_mode))）
fn is_regular_file(path: &str) -> bool {
    match stat_fields(path) {
        Err(_) => false,
        Ok(st) => (st.mode & S_IFMT) == S_IFREG,
    }
}

// 将 Vec<Pattern> 反向串联为链表（保持原始顺序）。
// C 中通过尾指针 pend 原地构建（O(1)），此处用 Vec 收集后串联（O(n)），
// 链表内容与顺序等价。
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

// === 原 C 函数：struct comment *new_comment(struct pattern *phead, char **line, int lines) ===
/// 用模式链表与描述行构建一个注释块节点。
/// C 的 line 为 char**（行数组），Rust 中直接持有 Vec<String>（C 的 lines 即其长度）。
fn new_comment(phead: Option<Box<Pattern>>, desc: Vec<String>) -> Box<Comment> {
    Box::new(Comment {
        pattern: phead,
        desc,
        next: None,
    })
}

// === 原 C 函数：struct infofile *new_infofile(const char *path, bool checkparents) ===
/// 解析 path 所指的 .info 文件（若 path 不是普通文件则尝试 path/.info；
/// checkparents 时沿父目录逐级向上查找 .info）。
pub fn new_infofile(path: &str, checkparents: bool) -> Option<Box<Infofile>> {
    // C: i = stat(path, &st); if (i < 0 || !S_ISREG(st.st_mode)) { ... } else fp = fopen(path, "r");
    let fp: Option<File> = if !is_regular_file(path) {
        // C: snprintf(buf, "%s/.info", path); fp = fopen(buf, "r");
        let mut fp = File::open(format!("{}/.info", path)).ok();

        // C: if (fp == NULL && checkparents) { 沿父目录向上搜索 }
        if fp.is_none() && checkparents {
            // C: strcpy(rpath, path);
            let mut rpath = path.to_string();
            // C: while ((fp == NULL) && (strcmp(rpath, "/") != 0))
            while fp.is_none() && rpath != "/" {
                // C: snprintf(buf, "%.*s/..", PATH_MAX-4, rpath);
                let buf = format!("{}/..", rpath);
                // C: if (realpath(buf, rpath) == NULL) break;
                match std::fs::canonicalize(&buf) {
                    Err(_) => break,
                    Ok(rp) => rpath = rp.to_string_lossy().into_owned(),
                }
                // C: snprintf(buf, "%.*s/.info", PATH_MAX-7, rpath);
                fp = File::open(format!("{}/.info", rpath)).ok();
            }
        }
        fp
    } else {
        File::open(path).ok()
    };
    // C: if (fp == NULL) return NULL;
    let fp = fp?;

    // 解析结果收集（C 的 chead/cend 链表与 phead/pend 模式链，
    // 均用 Vec 收集后串联，顺序等价）
    let mut comments: Vec<Box<Comment>> = Vec::new();
    let mut phead: Vec<Pattern> = Vec::new(); // 当前块的模式链表（值收集，构建时装箱）
    let mut line: Vec<String> = Vec::new(); // 当前块的描述行（C: char *line[PATH_MAX]）

    let mut reader = BufReader::new(fp);
    let mut buf = String::new();
    loop {
        buf.clear();
        // C: while (fgets(buf, PATH_MAX, fp) != NULL)（read_line 保留行尾 \n）
        let n = reader.read_line(&mut buf).unwrap_or(0);
        if n == 0 {
            break;
        }
        // C: if (buf[0] == '#') continue;
        if buf.starts_with('#') {
            continue;
        }
        gittrim(&mut buf);
        // C: if (strlen(buf) < 1) continue;
        if buf.is_empty() {
            continue;
        }

        if buf.starts_with('\t') {
            // C: line[lines++] = scopy(buf+1);
            line.push(buf.strip_prefix('\t').unwrap().to_string());
        } else {
            // C: if (lines) { 保存前一个 pattern/message 块 }
            if !line.is_empty() {
                if !phead.is_empty() {
                    // C: com = new_comment(phead, line, lines); 追加到 chead 链表尾
                    let phead_chain = link_patterns(std::mem::take(&mut phead));
                    comments.push(new_comment(phead_chain, std::mem::take(&mut line)));
                } else {
                    // C: 累积了无关联 pattern 的 message 行 → 释放丢弃
                    line.clear();
                }
                // C: phead = pend = NULL; lines = 0;（Vec 已被 take/clear）
            }
            // C: p = new_pattern(buf); 追加到 phead 模式链
            // C 中压入的是 Box 指针；这里解包成值收集，构建链表时再装箱
            phead.push(*new_pattern(&buf));
        }
    }
    // C: if (phead) { 提交最后一块 } else { 丢弃残留描述行 }
    if !phead.is_empty() {
        let phead_chain = link_patterns(std::mem::take(&mut phead));
        comments.push(new_comment(phead_chain, std::mem::take(&mut line)));
    } else {
        line.clear();
    }

    // 将 comments 反向串联为 chead 链表
    let mut chead: Option<Box<Comment>> = None;
    for com in comments.into_iter().rev() {
        let mut com = com;
        com.next = chead.take();
        chead = Some(com);
    }

    // C: inf->comments = chead; inf->path = scopy(path); inf->next = NULL;
    Some(Box::new(Infofile {
        path: path.to_string(),
        comments: chead,
        next: None,
    }))
}

// === 原 C 函数：void push_infostack(struct infofile *inf) ===
/// 将 infofile 压入信息栈顶。
/// C 中压入原指针；Rust 中接收 Option<Box<Infofile>>（None 时直接返回），
/// 调用方通过 clone 保留一份供自己跟踪（同 filter.rs 的模式）。
pub fn push_infostack(inf: Option<Box<Infofile>>) {
    // C: if (inf == NULL) return;
    let mut inf = match inf {
        Some(b) => b,
        None => return,
    };
    // C: inf->next = infostack; infostack = inf;
    // unsafe：访问全局信息栈 INFOSTACK
    unsafe {
        inf.next = INFOSTACK.take();
        INFOSTACK = Some(inf);
    }
}

// === 原 C 函数：struct infofile *pop_infostack(void) ===
/// 弹出并释放信息栈顶（C 中释放各 comment/pattern 与节点本身；
/// Rust 中 drop 自动释放）。C 恒返回 NULL，故 Rust 返回 ()。
pub fn pop_infostack() {
    // unsafe：访问全局信息栈 INFOSTACK
    unsafe {
        // C: inf = infostack; if (inf == NULL) return NULL;
        let top = match INFOSTACK.take() {
            Some(t) => t,
            None => return,
        };
        // C: infostack = infostack->next;（旧栈顶随 drop 释放）
        INFOSTACK = top.next;
    }
}

// === 原 C 函数：struct comment *infocheck(const char *path, const char *name, int top, bool isdir) ===
/// 若路径/名称匹配某个 .info 注释块的模式则返回该注释块。
/// top != 0 表示调用时目录内存在 .info 文件（允许按 name 匹配，仅对第一个 infofile 生效）。
/// C 返回指向栈中对象的指针；Rust 返回 &'static Comment（对象存活于
/// static mut 栈中，程序结束前有效，语义一致）。
pub fn infocheck(path: &str, name: &str, top: i32, isdir: bool) -> Option<&'static Comment> {
    let mut top = top != 0;

    // unsafe：遍历全局信息栈 INFOSTACK 并返回其内部对象的 'static 引用
    unsafe {
        // C: if (inf == NULL) return NULL;（while let 处理：栈空则不进入循环）
        let mut inf = INFOSTACK.as_deref();
        while let Some(inf_node) = inf {
            // C: int fpos = sprintf(xpattern, "%s/", inf->path);
            let mut xpattern = format!("{}/", inf_node.path);
            let fpos = xpattern.len();

            // C: for(com = inf->comments; com != NULL; com = com->next)
            let mut com = inf_node.comments.as_deref();
            while let Some(com_node) = com {
                // C: for(p = com->pattern; p != NULL; p = p->next)
                let mut p = com_node.pattern.as_deref();
                while let Some(pat) = p {
                    // C: if (patmatch(path, p->pattern, isdir) == 1) return com;
                    if patmatch(path.as_bytes(), pat.pattern.as_bytes(), isdir) == 1 {
                        return Some(&*(com_node as *const Comment));
                    }
                    // C: if (top && patmatch(name, p->pattern, isdir) == 1) return com;
                    if top && patmatch(name.as_bytes(), pat.pattern.as_bytes(), isdir) == 1 {
                        return Some(&*(com_node as *const Comment));
                    }

                    // C: sprintf(xpattern + fpos, "%s", p->pattern);
                    //     if (patmatch(path, xpattern, isdir) == 1) return com;
                    xpattern.push_str(&pat.pattern);
                    if patmatch(path.as_bytes(), xpattern.as_bytes(), isdir) == 1 {
                        return Some(&*(com_node as *const Comment));
                    }
                    xpattern.truncate(fpos);

                    p = pat.next.as_deref();
                }
                com = com_node.next.as_deref();
            }

            // C: top = 0;（name 匹配仅对第一个 infofile 有效）
            top = false;
            inf = inf_node.next.as_deref();
        }
    }
    None
}

// === 原 C 函数：void printcomment(size_t line, size_t lines, char *s) ===
/// 打印注释块的一行：根据行号选择 linedraw 的注释标记（单行/顶部/中间/底部/扩展）。
pub fn printcomment(line: usize, lines: usize, s: &str) {
    // unsafe：读取全局 LINEDRAW（指向 CSTABLE 中的表项）并向全局输出流输出
    unsafe {
        let ld = crate::color::LINEDRAW;
        if lines == 1 {
            // C: fprintf(outfile, "%s ", linedraw->csingle);
            outbytes!(ld.csingle);
            out!(" ");
        } else {
            if line == 0 {
                // C: fprintf(outfile, "%s ", linedraw->ctop);
                outbytes!(ld.ctop);
                out!(" ");
            } else if line < 2 {
                // C: fprintf(outfile, "%s ", (lines==2)? linedraw->cbot : linedraw->cmid);
                let which = if lines == 2 { ld.cbot } else { ld.cmid };
                outbytes!(which);
                out!(" ");
            } else {
                // C: fprintf(outfile, "%s ", (line == lines-1)? linedraw->cbot : linedraw->cext);
                let which = if line == lines - 1 { ld.cbot } else { ld.cext };
                outbytes!(which);
                out!(" ");
            }
        }
        // C: fprintf(outfile, "%s\n", s);
        out!("{}\n", s);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 测试辅助：创建带 .info 文件的临时目录
    fn make_temp_info(contents: &str) -> String {
        let dir = std::env::temp_dir().join(format!(
            "rustree_info_test_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(".info"), contents).unwrap();
        dir.to_string_lossy().into_owned()
    }

    #[test]
    fn test_new_infofile_parse() {
        let dir = make_temp_info(
            "# comment line\n\
             *.rs\n\
             \tRust source file\n\
             \tSecond line\n\
             # another block\n\
             *.c\n\
             \tC source\n",
        );
        let inf = new_infofile(&dir, false).expect("解析 .info 失败");

        // 第一个注释块：pattern *.rs + 两行描述
        let mut com = inf.comments.as_deref();
        let c1 = com.expect("第一个注释块");
        assert_eq!(c1.desc, vec!["Rust source file", "Second line"]);
        let p1 = c1.pattern.as_deref().expect("第一个块有 pattern");
        assert_eq!(p1.pattern, "*.rs");

        // 第二个注释块：pattern *.c + 一行描述
        com = c1.next.as_deref();
        let c2 = com.expect("第二个注释块");
        assert_eq!(c2.desc, vec!["C source"]);
        let p2 = c2.pattern.as_deref().expect("第二个块有 pattern");
        assert_eq!(p2.pattern, "*.c");

        assert!(c2.next.is_none());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_new_infofile_missing() {
        // 目录下无 .info 文件且 checkparents=false → None
        let dir = std::env::temp_dir().join(format!("rustree_info_none_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        assert!(new_infofile(dir.to_str().unwrap(), false).is_none());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_infocheck_and_stack() {
        let dir = make_temp_info("*.rs\n\tRust file\n");
        let inf = new_infofile(&dir, false).unwrap();
        push_infostack(Some(inf));

        // top=1 时允许按 name 匹配
        let com = infocheck("/some/path", "main.rs", 1, false);
        assert!(com.is_some());
        assert_eq!(com.unwrap().desc, vec!["Rust file"]);

        // 不匹配
        let com = infocheck("/some/path", "main.c", 1, false);
        assert!(com.is_none());

        // top=0 时 name 匹配无效（path 不含 .info 路径前缀）
        let com = infocheck("/some/path", "main.rs", 0, false);
        assert!(com.is_none());

        // 弹栈后不再匹配
        pop_infostack();
        let com = infocheck("/some/path", "main.rs", 1, false);
        assert!(com.is_none());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_printcomment_no_panic() {
        // 共享全局 OUTFILE/FLAG，串行化
        let _lock = crate::globals::TEST_LOCK.lock().unwrap();
        // 输出到 sink（不检查内容，只验证不 panic）
        // unsafe：测试中设置全局输出流为丢弃型 sink
        unsafe {
            crate::globals::OUTFILE = Some(Box::new(std::io::sink()));
        }
        printcomment(0, 1, "single");
        printcomment(0, 2, "first");
        printcomment(1, 2, "second");
        printcomment(0, 3, "top");
        printcomment(1, 3, "mid");
        printcomment(2, 3, "bottom");
        // unsafe：清理全局输出流
        unsafe {
            crate::globals::OUTFILE = None;
        }
    }
}
