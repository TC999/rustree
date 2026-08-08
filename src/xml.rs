// 文件路径：src/xml.rs
// 对应 C 源文件：xml.c
// XML 输出回调（-X 模式）。
// 注意：xml_printinfo 会设置 file->tag（对应 C 中 file->tag = ftype[t]），
// 供 xml_close 输出闭合标签使用；因此 printinfo 回调的 file 参数为 &mut。

use crate::globals::{FLAG, FTYPE, IFMT, NL};
use crate::hash::{gidtoname, uidtoname};
use crate::html::html_encode;
use crate::out;
use crate::tree::{Info, S_IFMT, S_IRWXG, S_IRWXO, S_IRWXU, S_ISGID, S_ISUID, S_ISVTX, Totals};
use crate::{do_date, prot};

// === 原 C 函数：void xml_indent(int maxlevel) ===
pub fn xml_indent(maxlevel: i32) {
    // C: char *spaces[] = {"    ", "   ", "  ", " ", ""};
    let spaces: [&str; 5] = ["    ", "   ", "  ", " ", ""];
    // unsafe：读取全局 FLAG
    unsafe {
        // C: int clvl = flag.compress_indent + (flag.remove_space? 1 : 0);
        let clvl = (FLAG.compress_indent + if FLAG.remove_space { 1 } else { 0 }) as usize;
        // C: if (flag.noindent) return;
        if FLAG.noindent {
            return;
        }
        out!("{}", spaces[clvl]);
        // C: for(i=0; i<maxlevel; i++)
        for _ in 0..maxlevel {
            out!("{}", spaces[clvl]);
        }
    }
}

// === 原 C 函数：void xml_fillinfo(struct _info *ent) ===
/// 输出条目的元数据属性。
fn xml_fillinfo(ent: &Info) {
    // unsafe：读取全局 FLAG
    unsafe {
        // C: if (flag.inode) fprintf(" inode=\"%lld\"", ent->inode);
        if FLAG.inode {
            out!(" inode=\"{}\"", ent.inode);
        }
        // C: if (flag.dev) fprintf(" dev=\"%d\"", ent->dev);
        if FLAG.dev {
            out!(" dev=\"{}\"", ent.dev);
        }
        // C: if (flag.p) fprintf(" mode=\"%04o\" prot=\"%s\"",
        //                         ent->mode & (S_IRWXU|S_IRWXG|S_IRWXO|S_ISUID|S_ISGID|S_ISVTX), prot(ent->mode));
        if FLAG.p {
            let mode = ent.mode & (S_IRWXU | S_IRWXG | S_IRWXO | S_ISUID | S_ISGID | S_ISVTX);
            out!(" mode=\"{:04o}\" prot=\"{}\"", mode, prot(ent.mode));
        }
        // C: if (flag.u) fprintf(" user=\"%s\"", uidtoname(ent->uid));
        if FLAG.u {
            out!(" user=\"{}\"", uidtoname(ent.uid));
        }
        // C: if (flag.g) fprintf(" group=\"%s\"", gidtoname(ent->gid));
        if FLAG.g {
            out!(" group=\"{}\"", gidtoname(ent.gid));
        }
        // C: if (flag.s) fprintf(" size=\"%lld\"", ent->size);
        if FLAG.s {
            out!(" size=\"{}\"", ent.size);
        }
        // C: if (flag.D) fprintf(" time=\"%s\"", do_date(flag.c? ctime : mtime));
        if FLAG.D {
            let t = if FLAG.c { ent.ctime } else { ent.mtime };
            out!(" time=\"{}\"", do_date(t));
        }
    }
}

// === 原 C 函数：void xml_intro(void) ===
pub fn xml_intro() {
    // unsafe：读取全局 NL
    unsafe {
        // C: fprintf(outfile, "<?xml version=\"1.0\"");
        out!("<?xml version=\"1.0\"");
        // C: if (charset) fprintf(" encoding=\"%s\"", charset);
        // 字符集机制已移除：固定声明 UTF-8
        out!(" encoding=\"UTF-8\"");
        // C: fprintf(outfile, "?>%s<tree>%s", _nl, _nl);
        out!("?>{}<tree>{}", NL, NL);
    }
}

// === 原 C 函数：void xml_outtro(void) ===
pub fn xml_outtro() {
    // unsafe：读取全局 NL
    unsafe {
        // C: fprintf(outfile, "</tree>%s", _nl);
        out!("</tree>{}", NL);
    }
}

// === 原 C 函数：int xml_printinfo(char *dirname, struct _info *file, int level) ===
pub fn xml_printinfo(_dirname: &str, file: Option<&mut Info>, level: i32) -> i32 {
    // unsafe：读取全局 FLAG
    unsafe {
        // C: if (!flag.noindent) xml_indent(level);
        if !FLAG.noindent {
            xml_indent(level);
        }

        // C: if (file != NULL) { if (file->lnk) mt = mode & S_IFMT; else mt = mode & S_IFMT; }
        //     else mt = 0;
        // 注意：C 的两个分支完全相同（均为 mode & S_IFMT），此处合并并保留注释
        let mt = if let Some(file) = &file {
            file.mode & S_IFMT
        } else {
            0
        };

        // C: for(t=0; ifmt[t]; t++) if (ifmt[t] == mt) break;
        let mut t = 0usize;
        while IFMT[t] != 0 {
            if IFMT[t] == mt {
                break;
            }
            t += 1;
        }
        // C: if (file) file->tag = ftype[t];（供 xml_close 输出闭合标签）
        if let Some(file) = file {
            file.tag = Some(FTYPE[t]);
        }
        // C: fprintf(outfile, "<%s", ftype[t]);
        out!("<{}", FTYPE[t]);
    }
    0
}

// === 原 C 函数：int xml_printfile(char *dirname, char *filename, struct _info *file, int descend) ===
pub fn xml_printfile(_dirname: &str, filename: &str, file: Option<&Info>, _descend: i32) -> i32 {
    // unsafe：读取全局 NL
    unsafe {
        // C: fprintf(outfile, " name=\""); html_encode(outfile, filename); fputc('"', outfile);
        out!(" name=\"");
        html_encode(filename);
        out!("\"");

        if let Some(file) = file {
            // C: if (file->comment) { " info=\"" ...（用 _nl 连接多行）}
            if !file.comment.is_empty() {
                out!(" info=\"");
                for (i, line) in file.comment.iter().enumerate() {
                    html_encode(line);
                    // C: if (file->comment[i+1]) fprintf(outfile, "%s", _nl);
                    if i + 1 < file.comment.len() {
                        out!("{}", NL);
                    }
                }
                out!("\"");
            }
            // C: if (file->lnk) { " target=\"" ... }
            if let Some(lnk) = &file.lnk {
                out!(" target=\"");
                html_encode(lnk);
                out!("\"");
            }
            // C: xml_fillinfo(file);
            xml_fillinfo(file);
        }
        // C: fputc('>', outfile);
        out!(">");
    }
    // C: return 1;（恒为 1，使调用方执行 lc.close 输出闭合标签）
    1
}

// === 原 C 函数：int xml_error(char *error) ===
pub fn xml_error(error: &str) -> i32 {
    // C: fprintf(outfile, "<error>%s</error>", error);
    out!("<error>{}</error>", error);
    0
}

// === 原 C 函数：void xml_newline(struct _info *file, int level, int postdir, int needcomma) ===
pub fn xml_newline(_file: Option<&Info>, _level: i32, postdir: i32, _needcomma: bool) {
    // unsafe：读取全局 NL
    unsafe {
        // C: if (postdir >= 0) fprintf(outfile, "%s", _nl);
        if postdir >= 0 {
            out!("{}", NL);
        }
    }
}

// === 原 C 函数：void xml_close(struct _info *file, int level, int needcomma) ===
pub fn xml_close(file: Option<&Info>, level: i32, _needcomma: bool) {
    // unsafe：读取全局 FLAG/NL
    unsafe {
        // C: if (!flag.noindent && level >= 0) xml_indent(level);
        if !FLAG.noindent && level >= 0 {
            xml_indent(level);
        }
        // C: fprintf(outfile, "</%s>%s", file? file->tag : "unknown", flag.noindent? "" : _nl);
        let tag = file.and_then(|f| f.tag).unwrap_or("unknown");
        out!("</{}>{}", tag, if FLAG.noindent { "" } else { NL });
    }
}

// === 原 C 函数：void xml_report(struct totals tot) ===
pub fn xml_report(tot: Totals) {
    // unsafe：读取全局 FLAG/NL
    unsafe {
        // C: xml_indent(0); fprintf("<report>%s", _nl);
        xml_indent(0);
        out!("<report>{}", NL);
        // C: if (flag.du) { xml_indent(1); fprintf("<size>%lld</size>%s", tot.size, _nl); }
        if FLAG.du {
            xml_indent(1);
            out!("<size>{}</size>{}", tot.size, NL);
        }
        // C: xml_indent(1); fprintf("<directories>%ld</directories>%s", tot.dirs, _nl);
        xml_indent(1);
        out!("<directories>{}</directories>{}", tot.dirs, NL);
        // C: if (!flag.d) { xml_indent(1); fprintf("<files>%ld</files>%s", tot.files, _nl); }
        if !FLAG.d {
            xml_indent(1);
            out!("<files>{}</files>{}", tot.files, NL);
        }
        // C: xml_indent(0); fprintf("</report>%s", _nl);
        xml_indent(0);
        out!("</report>{}", NL);
    }
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
        let _lock = crate::globals::TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let cap = Capture {
            buf: Arc::new(Mutex::new(Vec::new())),
        };
        // unsafe：测试中初始化全局状态
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
    fn test_xml_intro_outtro() {
        with_output(|buf| {
            xml_intro();
            xml_outtro();
            let out = String::from_utf8(buf.lock().unwrap_or_else(|e| e.into_inner()).clone()).unwrap();
            assert!(out.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
            assert!(out.contains("<tree>"));
            assert!(out.contains("</tree>"));
        });
    }

    #[test]
    fn test_xml_printinfo_sets_tag() {
        with_output(|buf| {
            let mut info = Info {
                mode: 0o100644 | 0o100000, // 普通文件
                ..Info::default()
            };
            xml_printinfo("/d", Some(&mut info), 0);
            // tag 被设置为 ftype[0] = "file"
            assert_eq!(info.tag, Some("file"));
            let out = String::from_utf8(buf.lock().unwrap_or_else(|e| e.into_inner()).clone()).unwrap();
            assert!(out.contains("<file"));
        });
    }

    #[test]
    fn test_xml_report() {
        with_output(|buf| {
            xml_report(Totals {
                files: 2,
                dirs: 1,
                size: 0,
            });
            let out = String::from_utf8(buf.lock().unwrap_or_else(|e| e.into_inner()).clone()).unwrap();
            assert!(out.contains("<report>"));
            assert!(out.contains("<directories>1</directories>"));
            assert!(out.contains("<files>2</files>"));
            assert!(out.contains("</report>"));
        });
    }

    #[test]
    fn test_xml_printfile_escapes() {
        with_output(|buf| {
            let r = xml_printfile("/d", "a&b<c>d\"e", None, 0);
            assert_eq!(r, 1);
            let out = String::from_utf8(buf.lock().unwrap_or_else(|e| e.into_inner()).clone()).unwrap();
            assert!(out.contains(" name=\"a&amp;b&lt;c&gt;d&quot;e\""));
        });
    }
}
