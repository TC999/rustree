// 文件路径：src/html.rs
// 对应 C 源文件：html.c
// HTML 输出回调：文档头/尾、URL 编码、条目打印、统计报告。

use std::io::Read;

use crate::globals::{FLAG, HINTRO, HOST, HOUTRO, HTMLDIRLEN, HVERSION, CHARSET, SP, TITLE};
use crate::out;
use crate::outbytes;
use crate::outc;
use crate::tree::{Info, Totals, PATH_MAX};
use crate::{fillinfo, indent, print_version, psize};

// C: char *class(struct _info *info) —— 根据文件类型返回 CSS 类名
fn class(info: &Info) -> &'static str {
    if info.isdir {
        "DIR"
    } else if info.isexe {
        "EXEC"
    } else if info.isfifo {
        "FIFO"
    } else if info.issok {
        "SOCK"
    } else {
        "NORM"
    }
}

// === 原 C 函数：void html_encode(FILE *fd, char *s) ===
/// HTML 转义：< > & " 分别替换为实体。
/// C 的 fd 参数在调用处恒为 outfile，故直接输出到全局输出流。
pub fn html_encode(s: &str) {
    for c in s.chars() {
        match c {
            '<' => out!("&lt;"),
            '>' => out!("&gt;"),
            '&' => out!("&amp;"),
            '"' => out!("&quot;"),
            _ => out!("{}", c),
        }
    }
}

// === 原 C 函数：bool url_encode(FILE *fd, char *s) ===
/// URL 编码：白名单（字母数字与 / - . _ ~）原样输出，其余 %XX。
/// 返回最后一个字符是否为 '/'。
pub fn url_encode(s: &str) -> bool {
    // 白名单比黑名单更安全：
    let unreserved = b"/-._~";
    let mut slash = false;
    // C 中 *s 为有符号 char，非 ASCII 时 %02X 会符号扩展；Rust 按 u8 处理更正确
    for &c in s.as_bytes() {
        if c.is_ascii_alphanumeric() || unreserved.contains(&c) {
            out!("{}", c as char);
        } else {
            out!("%{:02X}", c);
        }
        slash = c == b'/';
    }
    slash
}

// === 原 C 函数：void fcat(const char *filename) ===
/// 将文件内容原样复制到输出流（HTML 自定义 intro/outro 文件）。
fn fcat(filename: &str) {
    // C: if ((fp = fopen(filename, "r")) == NULL) return;
    let file = match std::fs::File::open(filename) {
        Ok(f) => f,
        Err(_) => return,
    };
    let mut reader = std::io::BufReader::new(file);
    let mut buf = [0u8; PATH_MAX];
    // C: while ((n = fread(buf, 1, PATH_MAX, fp)) > 0) fwrite(buf, 1, n, outfile);
    loop {
        let n = reader.read(&mut buf).unwrap_or(0);
        if n == 0 {
            break;
        }
        outbytes!(&buf[..n]);
    }
}

// === 原 C 函数：void html_intro(void) ===
pub fn html_intro() {
    // unsafe：读取全局 HINTRO/CHARSET/TITLE
    unsafe {
        if let Some(hi) = HINTRO {
            // C: if (Hintro) fcat(Hintro);
            fcat(hi);
        } else {
            out!("<!DOCTYPE html>\n\
<html>\n\
<head>\n\
 <meta http-equiv=\"Content-Type\" content=\"text/html; charset={}\">\n\
 <meta name=\"Author\" content=\"Made by 'tree'\">\n\
 <meta name=\"GENERATOR\" content=\"", CHARSET.unwrap_or("iso-8859-1"));
            print_version(false);
            out!("\">\n\
 <title>{}</title>\n\
 <style type=\"text/css\">\n\
  BODY {{ font-family : monospace, sans-serif;  color: black;}}\n\
  P {{ font-family : monospace, sans-serif; color: black; margin:0px; padding: 0px;}}\n\
  A:visited {{ text-decoration : none; margin : 0px; padding : 0px;}}\n\
  A:link    {{ text-decoration : none; margin : 0px; padding : 0px;}}\n\
  A:hover   {{ text-decoration: underline; background-color : yellow; margin : 0px; padding : 0px;}}\n\
  A:active  {{ margin : 0px; padding : 0px;}}\n\
  .VERSION {{ font-size: small; font-family : arial, sans-serif; }}\n\
  .NORM  {{ color: black;  }}\n\
  .FIFO  {{ color: purple; }}\n\
  .CHAR  {{ color: yellow; }}\n\
  .DIR   {{ color: blue;   }}\n\
  .BLOCK {{ color: yellow; }}\n\
  .LINK  {{ color: aqua;   }}\n\
  .SOCK  {{ color: fuchsia;}}\n\
  .EXEC  {{ color: green;  }}\n\
 </style>\n\
</head>\n\
<body>\n\
\t<h1>{}</h1><p>\n", TITLE, TITLE);
        }
    }
}

// === 原 C 函数：void html_outtro(void) ===
pub fn html_outtro() {
    // unsafe：读取全局 HOUTRO/LINEDRAW
    unsafe {
        if let Some(ho) = HOUTRO {
            // C: if (Houtro) fcat(Houtro);
            fcat(ho);
        } else {
            out!("\t<hr>\n\t<p class=\"VERSION\">\n");
            // C: fprintf(outfile, hversion, linedraw->copy ×4)（hversion 含 4 个 %s）
            // Rust 的 format_args! 要求字面量格式串，运行时格式串用 replace 处理
            let copy = String::from_utf8_lossy(crate::color::LINEDRAW.copy).into_owned();
            let s = HVERSION.replace("%s", &copy);
            out!("{}", s);
            out!("\t</p>\n</body>\n</html>\n");
        }
    }
}

// === 原 C 函数：void html_print(char *s) ===
/// 将空格替换为 &nbsp;（sp）后输出，末尾追加两个 sp。
fn html_print(s: &str) {
    for c in s.chars() {
        // C: if (s[i] == ' ') fprintf(outfile, "%s", sp);
        if c == ' ' {
            // unsafe：读取全局 SP（宏本身不含 unsafe 块）
            out!("{}", unsafe { SP });
        } else {
            out!("{}", c);
        }
    }
    // C: fprintf(outfile, "%s%s", sp, sp);
    out!("{}{}", unsafe { SP }, unsafe { SP });
}

// === 原 C 函数：int html_printinfo(char *dirname, struct _info *file, int level) ===
pub fn html_printinfo(_dirname: &str, file: Option<&mut Info>, level: i32) -> i32 {
    // C: char info[512]; fillinfo(info, file);
    let mut info = String::new();
    fillinfo(&mut info, file.as_deref());
    // unsafe：读取全局 FLAG/SP
    unsafe {
        if FLAG.metafirst {
            if info.starts_with('[') {
                html_print(&info);
                out!("{}{}", SP, SP);
            }
            if !FLAG.noindent {
                indent(level);
            }
        } else {
            if !FLAG.noindent {
                indent(level);
            }
            if info.starts_with('[') {
                html_print(&info);
                out!("{}{}", SP, SP);
            }
        }
    }
    0
}

// === 原 C 函数：int html_printfile(char *dirname, char *filename, struct _info *file, int descend) ===
/// 打印一个条目（<a> 元素）。descend > 1 时在链接后追加 /00Tree.html。
pub fn html_printfile(dirname: &str, filename: &str, file: Option<&Info>, descend: i32) -> i32 {
    out!("<a");
    if let Some(file) = file {
        // unsafe：读取全局 FLAG
        unsafe {
            // C: if (flag.force_color) fprintf(outfile, " class=\"%s\"", class(file));
            if FLAG.force_color {
                out!(" class=\"{}\"", class(file));
            }
        }
        // C: if (file->comment) { title 属性 }
        if !file.comment.is_empty() {
            out!(" title=\"");
            for (i, line) in file.comment.iter().enumerate() {
                html_encode(line);
                // C: if (file->comment[i+1]) fprintf(outfile, "\n");
                if i + 1 < file.comment.len() {
                    out!("\n");
                }
            }
            out!("\"");
        }
        // unsafe：读取全局 FLAG/HOST/HTMLDIRLEN
        unsafe {
            if !FLAG.nolinks {
                let host = HOST.unwrap_or("");
                // C: fprintf(outfile, " href=\"%s", host);（引号在后续输出中闭合）
                out!(" href=\"{}{}", host, "");
                // C 中 dirname 恒非 NULL（listdir/emit_tree 均传非空路径），
                // 因此 C 的 dirname==NULL 分支（用 host 拼接）在 Rust 中不可达，省略。
                let len = dirname.len();
                // C: size_t off = (len >= htmldirlen ? htmldirlen : 0);
                let off = if len >= HTMLDIRLEN { HTMLDIRLEN } else { 0 };
                // C: url_encode(outfile, dirname + (flag.htmloffset ? off : 0));
                let start = if FLAG.htmloffset { off } else { 0 };
                url_encode(&dirname[start..]);
                // C: if (strcmp(dirname, filename) != 0)
                if dirname != filename {
                    // C: if (dirname[strlen(dirname)-1] != '/') putc('/', outfile);
                    if !dirname.ends_with('/') {
                        outc!(b'/');
                    }
                    url_encode(filename);
                }
                // C: fprintf(outfile, "%s%s\"", (descend > 1 ? "/00Tree.html" : ""), (file->isdir && descend < 2 ? "/" : ""));
                let s1 = if descend > 1 { "/00Tree.html" } else { "" };
                let s2 = if file.isdir && descend < 2 { "/" } else { "" };
                out!("{}{}\"", s1, s2);
            }
        }
    }
    out!(">");
    // C: if (dirname) html_encode(outfile, filename); else html_encode(outfile, host);
    html_encode(filename);
    out!("</a>");
    0
}

// === 原 C 函数：int html_error(char *error) ===
pub fn html_error(error: &str) -> i32 {
    out!("  [{}]", error);
    0
}

// === 原 C 函数：void html_newline(struct _info *file, int level, int postdir, int needcomma) ===
pub fn html_newline(_file: Option<&Info>, _level: i32, _postdir: i32, _needcomma: bool) {
    out!("<br>\n");
}

// === 原 C 函数：void html_close(struct _info *file, int level, int needcomma) ===
pub fn html_close(file: Option<&Info>, _level: i32, _needcomma: bool) {
    // C: fprintf(outfile, "</%s><br>\n", file->tag);
    // 注意：HTML 模式不设置 tag（只有 xml_printinfo 设置），C 中此处为 NULL，
    // glibc 的 %s 对 NULL 输出 "(null)"；以 unwrap_or 保持该行为。
    let tag = file.and_then(|f| f.tag).unwrap_or("(null)");
    out!("</{}><br>\n", tag);
}

// === 原 C 函数：void html_report(struct totals tot) ===
pub fn html_report(tot: Totals) {
    out!("<br><br><p>\n\n");
    // unsafe：读取全局 FLAG
    unsafe {
        // C: if (flag.du) { psize(buf, tot.size); fprintf("%s%s used in ", ...); }
        if FLAG.du {
            let mut buf = String::new();
            psize(&mut buf, tot.size);
            out!("{}{} used in ", buf, if FLAG.h || FLAG.si { "" } else { " bytes" });
        }
        // C: if (flag.d) "%ld director%s" else "%ld director%s, %ld file%s"
        if FLAG.d {
            out!(
                "{} director{}\n",
                tot.dirs,
                if tot.dirs == 1 { "y" } else { "ies" }
            );
        } else {
            out!(
                "{} director{}, {} file{}\n",
                tot.dirs,
                if tot.dirs == 1 { "y" } else { "ies" },
                tot.files,
                if tot.files == 1 { "" } else { "s" }
            );
        }
    }
    out!("\n</p>\n");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::sync::{Arc, Mutex};

    #[derive(Clone)]
    struct Capture {
        buf: Arc<Mutex<Vec<u8>>>,
    }
    impl Write for Capture {
        fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
            self.buf.lock().unwrap_or_else(|e| e.into_inner()).extend_from_slice(data);
            Ok(data.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    fn with_output<F: FnOnce(&Arc<Mutex<Vec<u8>>>)>(f: F) {
        // 共享全局 OUTFILE/FLAG，串行化
        let _lock = crate::globals::TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let cap = Capture {
            buf: Arc::new(Mutex::new(Vec::new())),
        };
        // unsafe：测试中设置全局输出流与 FLAG
        unsafe {
            crate::globals::OUTFILE = Some(Box::new(cap.clone()));
            FLAG = crate::tree::Flags::new();
        }
        f(&cap.buf);
        // unsafe：清理全局输出流
        unsafe {
            crate::globals::OUTFILE = None;
        }
    }

    #[test]
    fn test_html_encode() {
        with_output(|buf| {
            html_encode("<a href=\"x&y\">");
            let out = String::from_utf8(buf.lock().unwrap_or_else(|e| e.into_inner()).clone()).unwrap();
            assert_eq!(out, "&lt;a href=&quot;x&amp;y&quot;&gt;");
        });
    }

    #[test]
    fn test_url_encode() {
        with_output(|buf| {
            // C 语义：返回"最后一个字符是否为 /"
            let slash = url_encode("/a b/c~/");
            assert!(slash);
            let out = String::from_utf8(buf.lock().unwrap_or_else(|e| e.into_inner()).clone()).unwrap();
            assert_eq!(out, "/a%20b/c~/");
            // 不以 / 结尾 → false
            let slash2 = url_encode("x");
            assert!(!slash2);
        });
    }

    #[test]
    fn test_html_report_text() {
        with_output(|buf| {
            html_report(Totals {
                files: 3,
                dirs: 1,
                size: 0,
            });
            let out = String::from_utf8(buf.lock().unwrap_or_else(|e| e.into_inner()).clone()).unwrap();
            assert!(out.contains("1 director"));
            assert!(out.contains("3 files"));
        });
    }
}


