// 文件路径：src/unix.rs
// 对应 C 源文件：unix.c
// 终端（Unix/ANSI）输出回调：条目打印、OSC 8 超链接、统计报告。

use crate::globals::{AUTHORITY, DIRPATHOFFSET, DIRS, FLAG, REALBASEPATH, SCHEME};
use crate::html::url_encode;
use crate::info::printcomment;
use crate::out;
use crate::outc;
use crate::tree::{Info, Totals};
use crate::{fillinfo, indent, printit, psize, Ftype};

// C: static char info[512] = {0};
// 文件级工作缓冲：unix_printinfo 填充，unix_newline 读取
static mut INFO: String = String::new();

// === 原 C 函数：int unix_printinfo(char *dirname, struct _info *file, int level) ===
pub fn unix_printinfo(_dirname: &str, file: Option<&Info>, level: i32) -> i32 {
    // unsafe：读写全局 INFO/FLAG 并输出
    unsafe {
        // C: fillinfo(info, file);
        fillinfo(&mut INFO, file);
        if FLAG.metafirst {
            // C: if (info[0] == '[') fprintf(outfile, "%s  ", info);
            if INFO.starts_with('[') {
                out!("{}  ", INFO);
            }
            // C: if (!flag.noindent) indent(level);
            if !FLAG.noindent {
                indent(level);
            }
        } else {
            // C: if (!flag.noindent) indent(level);
            if !FLAG.noindent {
                indent(level);
            }
            // C: if (info[0] == '[') fprintf(outfile, "%s  ", info);
            if INFO.starts_with('[') {
                out!("{}  ", INFO);
            }
        }
    }
    0
}

// === 原 C 函数：void open_hyperlink(char *dirname, char *filename) ===
/// 输出 OSC 8 终端超链接（scheme://authority:realbasepath/dirname/filename）。
fn open_hyperlink(dirname: &str, filename: &str) {
    // unsafe：读取全局 SCHEME/AUTHORITY/REALBASEPATH/DIRPATHOFFSET 并输出
    unsafe {
        // C: fprintf(outfile, "\033]8;;%s", scheme);
        out!("\x1B]8;;{}", SCHEME);
        // C: url_encode(outfile, authority);
        // C 中 authority 假定非 NULL（-H/--hyperlink 设置）；Rust 以空串兜底
        url_encode(AUTHORITY.unwrap_or(""));
        out!(":");
        // C: bool slash = url_encode(outfile, realbasepath);
        let mut slash = url_encode(&REALBASEPATH);
        // C: if (*(dirname+dirpathoffset)) —— dirname 在偏移后仍有内容
        let offset = DIRPATHOFFSET.min(dirname.len());
        if offset < dirname.len() {
            // C: slash = slash || (*(dirname+dirpathoffset) == '/');
            slash = slash || dirname.as_bytes()[offset] == b'/';
            // C: if (!slash) fputc('/', outfile);
            if !slash {
                outc!(b'/');
            }
            // C: if (!url_encode(outfile, dirname+dirpathoffset)) fputc('/', outfile);
            if !url_encode(&dirname[offset..]) {
                outc!(b'/');
            }
        } else if !slash {
            // C: else if (!slash) fputc('/', outfile);
            outc!(b'/');
        }
        // C: url_encode(outfile, filename);
        url_encode(filename);
        // C: fprintf(outfile, "\033\\");
        out!("\x1B\\");
    }
}

// === 原 C 函数：void close_hyperlink(void) ===
fn close_hyperlink() {
    // C: fprintf(outfile, "\033]8;;\033\\");
    out!("\x1B]8;;\x1B\\");
}

// === 原 C 函数：int unix_printfile(char *dirname, char *filename, struct _info *file, int descend) ===
pub fn unix_printfile(dirname: &str, filename: &str, file: Option<&Info>, _descend: i32) -> i32 {
    // unsafe：读取全局 FLAG 并输出
    unsafe {
        let mut colored = false;
        // C: int c;（未初始化，两个分支均先赋值后使用）
        let mut c;

        if let Some(file) = file {
            // C: if (flag.hyper) open_hyperlink(dirname, file->name);
            if FLAG.hyper {
                open_hyperlink(dirname, &file.name);
            }
            // C: if (flag.colorize) {
            //       if (file->lnk && flag.linktargetcolor)
            //         colored = color(file->lnkmode, file->name, file->orphan, false);
            //       else colored = color(file->mode, file->name, file->orphan, false);
            //     }
            if FLAG.colorize {
                if file.lnk.is_some() && FLAG.linktargetcolor {
                    colored = crate::color::color(file.lnkmode, &file.name, file.orphan, false);
                } else {
                    colored = crate::color::color(file.mode, &file.name, file.orphan, false);
                }
            }
        }

        // C: printit(filename);
        printit(filename);
        // C: if (colored) endcolor();
        if colored {
            crate::color::endcolor();
        }

        if let Some(file) = file {
            // C: if (flag.hyper) close_hyperlink();
            if FLAG.hyper {
                close_hyperlink();
            }
            // C: if (flag.F && !file->lnk) { if ((c = Ftype(file->mode))) fputc(c, outfile); }
            if FLAG.F && file.lnk.is_none() {
                c = Ftype(file.mode);
                if c != 0 {
                    outc!(c);
                }
            }
            // C: if (file->lnk) { ... }
            if let Some(lnk) = &file.lnk {
                out!(" -> ");
                // C: if (flag.hyper) open_hyperlink(dirname, file->name);
                if FLAG.hyper {
                    open_hyperlink(dirname, &file.name);
                }
                // C: if (flag.colorize) colored = color(file->lnkmode, file->lnk, file->orphan, true);
                if FLAG.colorize {
                    colored = crate::color::color(file.lnkmode, lnk, file.orphan, true);
                }
                printit(lnk);
                if colored {
                    crate::color::endcolor();
                }
                if FLAG.hyper {
                    close_hyperlink();
                }
                // C: if (flag.F) { if ((c = Ftype(file->lnkmode))) fputc(c, outfile); }
                if FLAG.F {
                    c = Ftype(file.lnkmode);
                    if c != 0 {
                        outc!(c);
                    }
                }
            }
        }
    }
    0
}

// === 原 C 函数：int unix_error(char *error) ===
pub fn unix_error(error: &str) -> i32 {
    // C: fprintf(outfile, "  [%s]", error);
    out!("  [{}]", error);
    0
}

// === 原 C 函数：void unix_newline(struct _info *file, int level, int postdir, int needcomma) ===
pub fn unix_newline(file: Option<&Info>, level: i32, postdir: i32, _needcomma: bool) {
    // unsafe：读写全局 DIRS/FLAG/INFO 并输出
    unsafe {
        // C: if (postdir <= 0) fprintf(outfile, "\n");
        if postdir <= 0 {
            out!("\n");
        }
        // C: if (file && file->comment)
        if let Some(file) = file {
            if !file.comment.is_empty() {
                // C: if (flag.metafirst) infosize = info[0] == '[' ? strlen(info)+2 : 0;
                let infosize = if FLAG.metafirst && INFO.starts_with('[') {
                    INFO.len() + 2
                } else {
                    0
                };
                // C: for(lines = 0; file->comment[lines]; lines++);
                let lines = file.comment.len();
                // C: dirs[level+1] = 1;
                DIRS[level as usize + 1] = 1;
                // C: for(line = 0; line < lines; line++)
                for line in 0..lines {
                    // C: if (flag.metafirst) printf("%*s", (int)infosize, "");
                    // 注意：C 源码此处使用 printf（写到 stdout）而非 fprintf(outfile, ...)，
                    // 属原 C 的笔误，此处原样保留
                    if FLAG.metafirst {
                        print!("{:>width$}", "", width = infosize);
                    }
                    indent(level);
                    printcomment(line, lines, &file.comment[line]);
                }
                // C: dirs[level+1] = 0;
                DIRS[level as usize + 1] = 0;
            }
        }
    }
}

// === 原 C 函数：void unix_report(struct totals tot) ===
pub fn unix_report(tot: Totals) {
    // unsafe：读取全局 FLAG 并输出
    unsafe {
        // C: fputc('\n', outfile);
        outc!(b'\n');
        // C: if (flag.du) { psize(buf, tot.size); fprintf("%s%s used in ", ...); }
        if FLAG.du {
            let mut buf = String::new();
            psize(&mut buf, tot.size);
            out!("{}{} used in ", buf, if FLAG.h || FLAG.si { "" } else { " bytes" });
        }
        // C: if (flag.d) "%ld director%s\n" else "%ld director%s, %ld file%s\n"
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

    // 初始化全局状态并捕获输出
    fn with_output<F: FnOnce(&Arc<Mutex<Vec<u8>>>)>(f: F) {
        // 共享全局 OUTFILE/FLAG/DIRS，串行化
        let _lock = crate::globals::TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let cap = Capture {
            buf: Arc::new(Mutex::new(Vec::new())),
        };
        // unsafe：测试中初始化全局状态
        unsafe {
            crate::globals::OUTFILE = Some(Box::new(cap.clone()));
            FLAG = crate::tree::Flags::new();
            DIRS.resize(crate::tree::PATH_MAX, 0);
        }
        f(&cap.buf);
        // unsafe：清理全局输出流
        unsafe {
            crate::globals::OUTFILE = None;
        }
    }

    #[test]
    fn test_unix_error() {
        with_output(|buf| {
            unix_error("oops");
            let out = String::from_utf8(buf.lock().unwrap_or_else(|e| e.into_inner()).clone()).unwrap();
            assert_eq!(out, "  [oops]");
        });
    }

    #[test]
    fn test_unix_printfile_plain() {
        with_output(|buf| {
            let r = unix_printfile("/dir", "file.txt", None, 0);
            assert_eq!(r, 0);
            let out = String::from_utf8(buf.lock().unwrap_or_else(|e| e.into_inner()).clone()).unwrap();
            assert_eq!(out, "file.txt");
        });
    }

    #[test]
    fn test_unix_report() {
        with_output(|buf| {
            unix_report(Totals {
                files: 3,
                dirs: 1,
                size: 0,
            });
            let out = String::from_utf8(buf.lock().unwrap_or_else(|e| e.into_inner()).clone()).unwrap();
            assert_eq!(out, "\n1 directory, 3 files\n");
        });
    }

    #[test]
    fn test_unix_report_du() {
        with_output(|buf| {
            // unsafe：读取全局 FLAG
            unsafe {
                FLAG.du = true;
                FLAG.si = true;
            }
            unix_report(Totals {
                files: 0,
                dirs: 0,
                size: 1500,
            });
            let out = String::from_utf8(buf.lock().unwrap_or_else(|e| e.into_inner()).clone()).unwrap();
            // psize 输出 " 1.5k"（1500/1000=1.5k）+ " used in "
            assert!(out.contains("used in "));
            assert!(out.contains("1.5k"));
        });
    }
}

