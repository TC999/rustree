// 文件路径：src/json.rs
// 对应 C 源文件：json.c
// JSON 输出回调（-J 模式）。
// JSON 编码字符串遵循 RFC 8259；注意 FIXME 注释：并非严格 UTF-8 输出
//（非 ASCII 字节按原始字节写出，与 C 的 %c 行为一致）。

use crate::globals::{FLAG, FTYPE, IFMT, NL};
use crate::hash::{gidtoname, uidtoname};
use crate::out;
use crate::outc;
use crate::tree::{Info, S_IFMT, S_IRWXG, S_IRWXO, S_IRWXU, S_ISGID, S_ISUID, S_ISVTX, Totals};
use crate::sys::do_date;
use crate::{prot, psize};

// === 原 C 函数：void json_encode(FILE *fd, char *s) ===
/// JSON 字符串转义：控制字符映射（\b \t \n \f \r）或 \u00xx，
/// 引号与反斜杠转义，其余原样（含非 ASCII 字节）。
fn json_encode(s: &str) {
    // C: char *ctrl = "0-------btn-fr------------------";（索引 0-31）
    let ctrl = b"0-------btn-fr------------------";
    for &c in s.as_bytes() {
        if c < 32 {
            // C: if (ctrl[c] != '-') fprintf("\\%c", ctrl[c]); else fprintf("\\u%04x", c);
            if ctrl[c as usize] != b'-' {
                out!("\\{}", ctrl[c as usize] as char);
            } else {
                out!("\\u{:04x}", c);
            }
        } else if c == b'"' || c == b'\\' {
            // C: fprintf("\\%c", *s);
            out!("\\{}", c as char);
        } else {
            // C: fprintf("%c", *s)（原始字节输出，非 ASCII 保持原字节）
            outc!(c);
        }
    }
}

// === 原 C 函数：void json_indent(int maxlevel) ===
pub fn json_indent(maxlevel: i32) {
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

// === 原 C 函数：void json_fillinfo(struct _info *ent) ===
/// 输出条目的元数据字段（inode/dev/mode/user/group/size/time）。
fn json_fillinfo(ent: &Info) {
    // unsafe：读取全局 FLAG
    unsafe {
        // C: if (flag.inode) fprintf(",\"inode\":%lld", ent->inode);
        // 注意：json.c 使用 ent->inode（stat 跟随链接的结果）
        if FLAG.inode {
            out!(",\"inode\":{}", ent.inode);
        }
        // C: if (flag.dev) fprintf(",\"dev\":%d", ent->dev);
        if FLAG.dev {
            out!(",\"dev\":{}", ent.dev);
        }
        // C: if (flag.p) fprintf(",\"mode\":\"%04o\",\"prot\":\"%s\"",
        //                         ent->mode & (S_IRWXU|S_IRWXG|S_IRWXO|S_ISUID|S_ISGID|S_ISVTX), prot(ent->mode));
        if FLAG.p {
            let mode = ent.mode & (S_IRWXU | S_IRWXG | S_IRWXO | S_ISUID | S_ISGID | S_ISVTX);
            out!(",\"mode\":\"{:04o}\",\"prot\":\"{}\"", mode, prot(ent.mode));
        }
        // C: if (flag.u) fprintf(",\"user\":\"%s\"", uidtoname(ent->uid));
        if FLAG.u {
            out!(",\"user\":\"{}\"", uidtoname(ent.uid));
        }
        // C: if (flag.g) fprintf(",\"group\":\"%s\"", gidtoname(ent->gid));
        if FLAG.g {
            out!(",\"group\":\"{}\"", gidtoname(ent.gid));
        }
        // C: if (flag.s) { h/si 时 psize 并 trim 前导空白；否则输出数字 }
        if FLAG.s {
            if FLAG.h || FLAG.si {
                let mut nbuf = String::new();
                psize(&mut nbuf, ent.size);
                // C: for(i=0; isspace(nbuf[i]); i++); /* trim() hack */
                out!(",\"size\":\"{}\"", nbuf.trim_start());
            } else {
                out!(",\"size\":{}", ent.size);
            }
        }
        // C: if (flag.D) fprintf(",\"time\":\"%s\"", do_date(flag.c? ctime : mtime));
        if FLAG.D {
            let t = if FLAG.c { ent.ctime } else { ent.mtime };
            out!(",\"time\":\"{}\"", do_date(t));
        }
    }
}

// === 原 C 函数：void json_intro(void) ===
pub fn json_intro() {
    // unsafe：读取全局 FLAG/NL
    unsafe {
        // C: fprintf(outfile, "[%s", flag.noindent? "" : _nl);
        out!("[{}", if FLAG.noindent { "" } else { NL });
    }
}

// === 原 C 函数：void json_outtro(void) ===
pub fn json_outtro() {
    // unsafe：读取全局 FLAG/NL
    unsafe {
        // C: fprintf(outfile, "%s]\n", flag.noindent? "" : _nl);
        out!("{}]\n", if FLAG.noindent { "" } else { NL });
    }
}

// === 原 C 函数：int json_printinfo(char *dirname, struct _info *file, int level) ===
pub fn json_printinfo(_dirname: &str, file: Option<&mut Info>, level: i32) -> i32 {
    // unsafe：读取全局 FLAG
    unsafe {
        // C: if (!flag.noindent) json_indent(level);
        if !FLAG.noindent {
            json_indent(level);
        }

        // C: if (file != NULL) { if (file->lnk) mt = mode & S_IFMT; else mt = mode & S_IFMT; }
        //     else mt = 0;
        // 注意：C 的两个分支完全相同（均为 mode & S_IFMT），此处合并并保留注释
        let mt = if let Some(file) = file {
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
        // C: fprintf(outfile, "{\"type\":\"%s\"", ftype[t]);
        out!("{{\"type\":\"{}\"", FTYPE[t]);
    }
    0
}

// === 原 C 函数：int json_printfile(char *dirname, char *filename, struct _info *file, int descend) ===
pub fn json_printfile(_dirname: &str, filename: &str, file: Option<&Info>, descend: i32) -> i32 {
    let mut direrr = false;

    // C: fprintf(outfile, ",\"name\":\""); json_encode(outfile, filename); fputc('"', outfile);
    out!(",\"name\":\"");
    json_encode(filename);
    outc!(b'"');

    if let Some(file) = file {
        // C: if (file->comment) { ",\"info\":\"..."（用 \\n 连接多行）}
        if !file.comment.is_empty() {
            out!(",\"info\":\"");
            for (i, line) in file.comment.iter().enumerate() {
                json_encode(line);
                // C: if (file->comment[i+1]) fprintf(outfile, "\\n");
                if i + 1 < file.comment.len() {
                    out!("\\n");
                }
            }
            out!("\"");
        }
        // C: if (file->lnk) { ",\"target\":\"" ... }
        if let Some(lnk) = &file.lnk {
            out!(",\"target\":\"");
            json_encode(lnk);
            outc!(b'"');
        }
        // C: json_fillinfo(file);
        json_fillinfo(file);
        // C: direrr = file->isdir && file->err;
        direrr = file.isdir && file.err.is_some();
    }

    // C: if (descend || direrr) fprintf(",\"contents\":["); else fputc('}', outfile);
    if descend != 0 || direrr {
        out!(",\"contents\":[");
    } else {
        outc!(b'}');
    }

    // C: return descend || direrr;
    if descend != 0 || direrr {
        1
    } else {
        0
    }
}

// === 原 C 函数：int json_error(char *error) ===
pub fn json_error(error: &str) -> i32 {
    // C: fprintf(outfile, "{\"error\": \"%s\"}%s", error, flag.noindent? "" : "");
    // 注意：C 中两个分支均为空串（原样保留），等价于省略第二个参数
    out!("{{\"error\": \"{}\"}}", error);
    0
}

// === 原 C 函数：void json_newline(struct _info *file, int level, int postdir, int needcomma) ===
pub fn json_newline(_file: Option<&Info>, _level: i32, _postdir: i32, needcomma: bool) {
    // unsafe：读取全局 NL
    unsafe {
        // C: fprintf(outfile, "%s%s", needcomma? "," : "", _nl);
        out!("{}{}", if needcomma { "," } else { "" }, NL);
    }
}

// === 原 C 函数：void json_close(struct _info *file, int level, int needcomma) ===
pub fn json_close(_file: Option<&Info>, level: i32, needcomma: bool) {
    // unsafe：读取全局 FLAG
    unsafe {
        // C: if (!flag.noindent) json_indent(level);
        if !FLAG.noindent {
            json_indent(level);
        }
        // C: fprintf(outfile, "]}%s%s", needcomma? ",":"", flag.noindent? "":"\n");
        out!(
            "]}}{}{}",
            if needcomma { "," } else { "" },
            if FLAG.noindent { "" } else { "\n" }
        );
    }
}

// === 原 C 函数：void json_report(struct totals tot) ===
pub fn json_report(tot: Totals) {
    // unsafe：读取全局 FLAG
    unsafe {
        // C: fputc(',', outfile);
        outc!(b',');
        json_indent(0);
        // C: fprintf(outfile, "{\"type\":\"report\"");
        out!("{{\"type\":\"report\"");
        // C: if (flag.du) fprintf(",\"size\":%lld", tot.size);
        if FLAG.du {
            out!(",\"size\":{}", tot.size);
        }
        // C: fprintf(",\"directories\":%ld", tot.dirs);
        out!(",\"directories\":{}", tot.dirs);
        // C: if (!flag.d) fprintf(",\"files\":%ld", tot.files);
        if !FLAG.d {
            out!(",\"files\":{}", tot.files);
        }
        // C: fprintf(outfile, "}");
        out!("}}");
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
    fn test_json_encode_special() {
        with_output(|buf| {
            json_encode("a\"b\\c\n");
            let out = buf.lock().unwrap_or_else(|e| e.into_inner()).clone();
            assert_eq!(out, b"a\\\"b\\\\c\\n");
        });
    }

    #[test]
    fn test_json_encode_control() {
        with_output(|buf| {
            json_encode("\t\x01");
            let out = buf.lock().unwrap_or_else(|e| e.into_inner()).clone();
            // \t → \\t；\x01 不在映射表 → \u0001
            assert_eq!(out, b"\\t\\u0001");
        });
    }

    #[test]
    fn test_json_report() {
        with_output(|buf| {
            json_report(Totals {
                files: 2,
                dirs: 1,
                size: 0,
            });
            let out = String::from_utf8(buf.lock().unwrap_or_else(|e| e.into_inner()).clone()).unwrap();
            assert!(out.contains("{\"type\":\"report\""));
            assert!(out.contains("\"directories\":1"));
            assert!(out.contains("\"files\":2"));
            assert!(out.starts_with(','));
        });
    }
}

