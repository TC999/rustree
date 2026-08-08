// 文件路径：src/globals.rs
// 对应 C 源码中跨文件共享的全局变量（tree.c 定义、其余 .c 文件 extern 引用）。
//
// C 的全局变量在 Rust 中没有直接对应物。本程序为单线程，因此使用
// `static mut` 保持与 C 完全一致的全局可变语义，所有访问点均以
// `unsafe` 块 + 中文注释标明原因。
//
// 仅 tree.c 内部使用、不被其他文件 extern 引用的静态数据
// （sorts 排序表、fmt 等）留在 main.rs（tree.c 的对应模块）中。

use std::io::Write;

use crate::tree::{
    Flags, Getfulltree, ListingCalls, SortFn, S_IFBLK, S_IFCHR, S_IFDIR, S_IFIFO, S_IFLNK,
    S_IFREG, S_IFSOCK,
};

/* =====================================================================
 * 输出流与输出宏
 * 对应 C 的 FILE *outfile 以及 fprintf(outfile,...)/fputs/fputc。
 * ===================================================================== */

// 测试专用的全局串行锁：多个测试模块共享 FLAG/OUTFILE/DIRS 等全局
// 状态，cargo test 多线程并行时需串行化
#[cfg(test)]
pub static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

// 对应 C 的 scopy()/字符串常量：把 String 泄漏为 'static 字符串
// （用于填入 static mut 的 Option<&str> 全局，模拟 C 中 malloc 后不释放的字符串）
pub fn leak_str(s: String) -> &'static str {
    Box::leak(s.into_boxed_str())
}

// C: FILE *outfile = NULL;
// 全局输出流：stdout 或 -o 指定的文件。Box<dyn Write> 使 stdout 与文件可统一处理。
pub static mut OUTFILE: Option<Box<dyn Write>> = None;

// 对应 C 的 fprintf(outfile, fmt, ...)/fputs/fputc。
// 宏仅做参数转发，unsafe 访问集中在辅助函数中（避免宏内展开 metavariable，
// 同时使 static mut 参数在调用点的求值位置明确）。

// unsafe：访问全局可变输出流（单线程程序，与 C 的全局 outfile 语义一致）
pub fn out_write(args: std::fmt::Arguments<'_>) {
    unsafe {
        let w: &mut dyn Write = OUTFILE.as_mut().unwrap();
        let _ = w.write_fmt(args);
    }
}

// unsafe：访问全局可变输出流
pub fn out_write_bytes(bytes: &[u8]) {
    unsafe {
        let _ = OUTFILE.as_mut().unwrap().write_all(bytes);
    }
}

// unsafe：访问全局可变输出流
pub fn out_write_byte(c: u8) {
    unsafe {
        let _ = OUTFILE.as_mut().unwrap().write_all(&[c]);
    }
}

#[macro_export]
macro_rules! out {
    ($($arg:tt)*) => {
        $crate::globals::out_write(format_args!($($arg)*))
    };
}

// 对应 C 的 fwrite(bytes, 1, len, outfile)
// 用于输出 linedraw 表中含非 UTF-8 字节的序列（ANSI 转义、Shift-JIS 等）
#[macro_export]
macro_rules! outbytes {
    ($bytes:expr) => {
        $crate::globals::out_write_bytes($bytes)
    };
}

// 对应 C 的 fputc(c, outfile)
#[macro_export]
macro_rules! outc {
    ($c:expr) => {
        $crate::globals::out_write_byte($c)
    };
}

/* =====================================================================
 * tree.c / 全局选项与回调
 * ===================================================================== */

// C: struct Flags flag;
pub static mut FLAG: Flags = Flags::new();

// C: struct listingcalls lc;
// 输出回调集合，由 main() 根据输出格式（终端/HTML/XML/JSON）赋值
pub static mut LC: Option<ListingCalls> = None;

/* =====================================================================
 * tree.c / 模式列表（-P/-I 参数）
 * ===================================================================== */

// C: int pattern = 0, maxpattern = 0;
pub static mut PATTERN: i32 = 0;
pub static mut MAXPATTERN: i32 = 0;

// C: int ipattern = 0, maxipattern = 0;
pub static mut IPATTERN: i32 = 0;
pub static mut MAXIPATTERN: i32 = 0;

// C: char **patterns = NULL;（-P 模式数组，NULL 结尾）
pub static mut PATTERNS: Vec<String> = Vec::new();

// C: char **ipatterns = NULL;（-I 模式数组，NULL 结尾）
pub static mut IPATTERNS: Vec<String> = Vec::new();

/* =====================================================================
 * tree.c / 字符串全局
 * 原 C 中这些 char* 多指向 argv 或字符串常量，Rust 中以 'static str
 * 表达；需要动态字符串时用 Box::leak 获得 'static 生命周期。
 * ===================================================================== */

// C: char *host = NULL;（HTML 的 baseHREF / 超链接主机）
pub static mut HOST: Option<&'static str> = None;

// C: char *title = "Directory Tree";
pub static mut TITLE: &str = "Directory Tree";

// C: char *sp = " ";（空格，HTML 模式下为 "&nbsp;"）
pub static mut SP: &str = " ";

// C: char *_nl = "\n";（换行，-i 时置为 ""）
pub static mut NL: &str = "\n";

// C: char *Hintro = NULL;（HTML 自定义 intro 文件）
pub static mut HINTRO: Option<&'static str> = None;

// C: char *Houtro = NULL;（HTML 自定义 outro 文件）
pub static mut HOUTRO: Option<&'static str> = None;

// C: char *scheme = "file://";（OSC 8 超链接协议）
pub static mut SCHEME: &str = "file://";

// C: char *authority = NULL;（OSC 8 超链接 authority/主机名）
pub static mut AUTHORITY: Option<&'static str> = None;

// C: char *file_comment = "#";（--fromfile 时的注释行前缀）
pub static mut FILE_COMMENT: &str = "#";

// C: char *file_pathsep = "/";（路径分隔符）
pub static mut FILE_PATHSEP: &str = "/";

// C: char *timefmt = NULL;（--timefmt 自定义时间格式）
pub static mut TIMEFMT: Option<&'static str> = None;

// C: const char *charset = NULL;（字符集，由 getcharset() 决定）
pub static mut CHARSET: Option<&'static str> = None;

/* =====================================================================
 * tree.c / 函数指针全局
 * ===================================================================== */

// C: struct _info **(*getfulltree)(...) = unix_getfulltree;
// 读取完整目录树的函数（默认 unix_getfulltree，与 C 的全局初始化一致）；
// --fromfile/--fromtabfile 会替换为 file.c 的实现。
pub static mut GETFULLTREE: Option<Getfulltree> = Some(crate::unix_getfulltree);

// C: int (*basesort)(struct _info **, struct _info **) = alnumsort;
// 基础排序比较器（默认 alnumsort，与 C 的全局初始化一致）；
// -U/-t/-c/-v/--sort 会修改。
pub static mut BASESORT: Option<SortFn> = Some(crate::alnumsort);

// C: int (*topsort)(struct _info **, struct _info **) = NULL;
// 顶层排序比较器；--dirsfirst/--filesfirst 设置，为 NULL 表示不排序。
pub static mut TOPSORT: Option<SortFn> = None;

/* =====================================================================
 * tree.c / 遍历状态
 * ===================================================================== */

// C: int *dirs;（缩进级别数组，动态增长）
pub static mut DIRS: Vec<i32> = Vec::new();

// C: ssize_t Level;（-L 最大深度，-1 表示无限）
pub static mut LEVEL: i64 = -1;

// C: size_t maxdirs;（dirs 数组的容量）
pub static mut MAXDIRS: usize = 0;

// C: int errors;（统计的错误数，影响 main 的退出码）
pub static mut ERRORS: i32 = 0;

// C: char xpattern[PATH_MAX];（跨函数复用的工作缓冲区）
// Rust 实现以局部 String 替代，声明保留以对应 C 全局。
#[allow(dead_code)]
pub static mut XPATTERN: String = String::new();

// C: int mb_cur_max;（当前 locale 下多字节字符的最大字节数）
pub static mut MB_CUR_MAX: i32 = 1;

/* =====================================================================
 * list.c 与 unix.c 共享
 * ===================================================================== */

// C: char realbasepath[PATH_MAX];（hyper 模式下 realpath() 的结果）
pub static mut REALBASEPATH: String = String::new();

// C: size_t dirpathoffset = 0;（超链接路径偏移）
pub static mut DIRPATHOFFSET: usize = 0;

/* =====================================================================
 * html.c
 * ===================================================================== */

// C: size_t htmldirlen = 0;（HTML 模式下当前目录名的长度）
pub static mut HTMLDIRLEN: usize = 0;

/* =====================================================================
 * tree.c / 静态数据表（json.c、xml.c 通过 extern 引用）
 * ===================================================================== */

// C: const mode_t ifmt[] = {S_IFREG, S_IFDIR, S_IFLNK, S_IFCHR, S_IFBLK, S_IFSOCK, S_IFIFO, 0};
// 文件类型掩码表，以 0 结尾（哨兵索引对应 FTYPE 中的 "unknown"）
pub static IFMT: [u32; 8] = [
    S_IFREG, S_IFDIR, S_IFLNK, S_IFCHR, S_IFBLK, S_IFSOCK, S_IFIFO, 0,
];

// C: const char *ftype[] = {"file", "directory", "link", "char", "block", "socket", "fifo", "unknown", NULL};
pub static FTYPE: [&str; 8] = [
    "file",
    "directory",
    "link",
    "char",
    "block",
    "socket",
    "fifo",
    "unknown",
];

// C: const char fmt[] = "-dlcbsp?";（prot() 中的文件类型字符表）
// 8 项：ifmt 循环停在哨兵索引 7 时取 '?'（与 ftype 的 "unknown" 对应）
#[allow(clippy::byte_char_slices)]
pub static FMT: [u8; 8] = [b'-', b'd', b'l', b'c', b'b', b's', b'p', b'?'];

// C: char *version = "$Version: $ tree v2.3.2 %s 1996 - 2026 ... $";
pub static VERSION: &str = "$Version: $ tree v2.3.2 %s 1996 - 2026 by Steve Baker, Thomas Moore, Francesc Rocher, Florian Sesser, Kyosuke Tokoro $";

// C: char *hversion = "\t\t tree v2.3.2 %s ...";（HTML 输出版本脚注，html.c 引用）
pub static HVERSION: &str = "\t\t tree v2.3.2 %s 1996 - 2026 by Steve Baker and Thomas Moore <br>\n\
\t\t HTML output hacked and copyleft %s 1998 by Francesc Rocher <br>\n\
\t\t JSON output hacked and copyleft %s 2014 by Florian Sesser <br>\n\
\t\t Charsets / OS/2 support %s 2001 by Kyosuke Tokoro\n";



