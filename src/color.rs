// 文件路径：src/color.rs
// 对应 C 源文件：color.c
// LS_COLORS 支持（Linux dircolors 格式）：颜色代码解析、颜色输出、
// 字符集检测（getcharset）与线条绘制表（linedraw / cstable）。

use std::io::{IsTerminal, Write};

use crate::globals::{leak_str, FLAG};
use crate::outbytes;
use crate::tree::{
    Extensions, Linedraw, S_IFBLK, S_IFCHR, S_IFDIR, S_IFIFO, S_IFLNK, S_IFMT, S_IFREG, S_IFSOCK,
    S_ISGID, S_ISUID, S_ISVTX, S_IWOTH, S_IXGRP, S_IXOTH, S_IXUSR,
};

/* ---------------------------------------------------------------------
 * 颜色代码索引常量（C: enum { ERROR = -1, CMD_COLOR = 0, ... DOT_EXTENSION }）
 * 注意：原 C 中 '#' 注释掉的 vgacolor/colortable 数组未翻译。
 * --------------------------------------------------------------------- */
pub const ERROR: i32 = -1;
#[allow(dead_code)]
pub const CMD_COLOR: i32 = 0;
#[allow(dead_code)]
pub const CMD_OPTIONS: i32 = 1;
#[allow(dead_code)]
pub const CMD_TERM: i32 = 2;
#[allow(dead_code)]
pub const CMD_EIGHTBIT: i32 = 3;
#[allow(dead_code)]
pub const COL_RESET: i32 = 4;
pub const COL_NORMAL: i32 = 5;
pub const COL_FILE: i32 = 6;
pub const COL_DIR: i32 = 7;
pub const COL_LINK: i32 = 8;
pub const COL_FIFO: i32 = 9;
pub const COL_DOOR: i32 = 10;
pub const COL_BLK: i32 = 11;
pub const COL_CHR: i32 = 12;
pub const COL_ORPHAN: i32 = 13;
pub const COL_SOCK: i32 = 14;
pub const COL_SETUID: i32 = 15;
pub const COL_SETGID: i32 = 16;
pub const COL_STICKY_OTHER_WRITABLE: i32 = 17;
pub const COL_OTHER_WRITABLE: i32 = 18;
pub const COL_STICKY: i32 = 19;
pub const COL_EXEC: i32 = 20;
pub const COL_MISSING: i32 = 21;
pub const COL_LEFTCODE: i32 = 22;
pub const COL_RIGHTCODE: i32 = 23;
pub const COL_ENDCODE: i32 = 24;
pub const COL_BOLD: i32 = 25;
pub const COL_ITALIC: i32 = 26;
// 保持此值为最后一个，决定 color_code 数组的大小：
pub const DOT_EXTENSION: i32 = 27;

// C: enum { MCOL_INODE, MCOL_PERMS, MCOL_USER, MCOL_GROUP, MCOL_SIZE, MCOL_DATE, MCOL_INDENTLINES }
// 元数据 ID 常量（原 C 中未实际使用，仅为头文件级定义）
#[allow(dead_code)]
pub const MCOL_INODE: i32 = 0;
#[allow(dead_code)]
pub const MCOL_PERMS: i32 = 1;
#[allow(dead_code)]
pub const MCOL_USER: i32 = 2;
#[allow(dead_code)]
pub const MCOL_GROUP: i32 = 3;
#[allow(dead_code)]
pub const MCOL_SIZE: i32 = 4;
#[allow(dead_code)]
pub const MCOL_DATE: i32 = 5;
#[allow(dead_code)]
pub const MCOL_INDENTLINES: i32 = 6;

// C: char *color_code[DOT_EXTENSION+1] = {NULL};
// 颜色代码表；parse_dir_colors 填充
static mut COLOR_CODE: [Option<&str>; (DOT_EXTENSION + 1) as usize] = [const { None }; 28];

// C: struct extensions *ext = NULL;
// 扩展名颜色链表（*.ext=code）
static mut EXT: Option<Box<Extensions>> = None;

// C: const struct linedraw *linedraw;
// 当前选中的线条绘制表（由 initlinedraw 按字符集选择）
pub static mut LINEDRAW: &Linedraw = &CSTABLE[0];

// === 原 C 函数：char **split(char *str, const char *delim, size_t *nwrds) ===
/// 用分隔符切分字符串。
/// C 版使用 strtok（连续分隔符视为单个，跳过开头分隔符），
/// 因此这里用 split 后过滤空项以保持相同语义。C 的 nwrds 输出即返回 Vec 的长度。
fn split(str_: &str, delim: &str) -> Vec<String> {
    str_
        .split(delim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}

// === 原 C 函数：int cmd(char *s) ===
/// 将 LS_COLORS 指令名映射为颜色代码索引；'*' 开头返回 DOT_EXTENSION。
fn cmd(s: &str) -> i32 {
    // C: if (s == NULL) return ERROR;
    if s.is_empty() {
        return ERROR;
    }
    // C: if (s[0] == '*') return DOT_EXTENSION;
    if s.starts_with('*') {
        return DOT_EXTENSION;
    }
    // C: 静态指令表（{NULL, 0} 哨兵省略，用数组长度）
    const CMDS: &[(&str, i32)] = &[
        ("rs", COL_RESET),
        ("no", COL_NORMAL),
        ("fi", COL_FILE),
        ("di", COL_DIR),
        ("ln", COL_LINK),
        ("pi", COL_FIFO),
        ("do", COL_DOOR),
        ("bd", COL_BLK),
        ("cd", COL_CHR),
        ("or", COL_ORPHAN),
        ("so", COL_SOCK),
        ("su", COL_SETUID),
        ("sg", COL_SETGID),
        ("tw", COL_STICKY_OTHER_WRITABLE),
        ("ow", COL_OTHER_WRITABLE),
        ("st", COL_STICKY),
        ("ex", COL_EXEC),
        ("mi", COL_MISSING),
        ("lc", COL_LEFTCODE),
        ("rc", COL_RIGHTCODE),
        ("ec", COL_ENDCODE),
    ];
    for (name, num) in CMDS {
        // C: if (!strcmp(cmds[i].cmd, s)) return cmds[i].cmdnum;
        if *name == s {
            return *num;
        }
    }
    ERROR
}

// 默认 LS_COLORS 值（C 源码中的长字符串常量，原样保留）
const DEFAULT_LS_COLORS: &str = ":no=00:rs=0:fi=00:di=01;34:ln=01;36:pi=40;33:so=01;35:bd=40;33;01:cd=40;33;01:or=40;31;01:ex=01;32:*.bat=01;32:*.BAT=01;32:*.btm=01;32:*.BTM=01;32:*.cmd=01;32:*.CMD=01;32:*.com=01;32:*.COM=01;32:*.dll=01;32:*.DLL=01;32:*.exe=01;32:*.EXE=01;32:*.arj=01;31:*.bz2=01;31:*.deb=01;31:*.gz=01;31:*.lzh=01;31:*.rpm=01;31:*.tar=01;31:*.taz=01;31:*.tb2=01;31:*.tbz2=01;31:*.tbz=01;31:*.tgz=01;31:*.tz2=01;31:*.z=01;31:*.Z=01;31:*.zip=01;31:*.ZIP=01;31:*.zoo=01;31:*.asf=01;35:*.ASF=01;35:*.avi=01;35:*.AVI=01;35:*.bmp=01;35:*.BMP=01;35:*.flac=01;35:*.FLAC=01;35:*.gif=01;35:*.GIF=01;35:*.jpg=01;35:*.JPG=01;35:*.jpeg=01;35:*.JPEG=01;35:*.m2a=01;35:*.M2a=01;35:*.m2v=01;35:*.M2V=01;35:*.mov=01;35:*.MOV=01;35:*.mp3=01;35:*.MP3=01;35:*.mpeg=01;35:*.MPEG=01;35:*.mpg=01;35:*.MPG=01;35:*.ogg=01;35:*.OGG=01;35:*.ppm=01;35:*.rm=01;35:*.RM=01;35:*.tga=01;35:*.TGA=01;35:*.tif=01;35:*.TIF=01;35:*.wav=01;35:*.WAV=01;35:*.wmv=01;35:*.WMV=01;35:*.xbm=01;35:*.xpm=01;35:";

// === 原 C 函数：void parse_dir_colors(void) ===
/// 解析环境变量中的 LS_COLORS/TREE_COLORS，初始化颜色表。
pub fn parse_dir_colors() {
    // unsafe：读写全局选项 FLAG
    unsafe {
        // C: if (flag.H) return;（HTML 模式不解析颜色）
        if FLAG.H {
            return;
        }

        // C: s = getenv("NO_COLOR"); if (s && s[0]) flag.nocolor = true;
        if let Some(v) = std::env::var_os("NO_COLOR") {
            if !v.is_empty() {
                FLAG.nocolor = true;
            }
        }

        // C: if (getenv("TERM") == NULL) { flag.colorize = false; return; }
        if std::env::var_os("TERM").is_none() {
            FLAG.colorize = false;
            return;
        }

        // C: cc = getenv("CLICOLOR") != NULL;
        let cc = std::env::var_os("CLICOLOR").is_some();
        // C: if (getenv("CLICOLOR_FORCE") != NULL && !flag.nocolor) flag.force_color=true;
        if std::env::var_os("CLICOLOR_FORCE").is_some() && !FLAG.nocolor {
            FLAG.force_color = true;
        }

        // C: s = getenv("TREE_COLORS"); if (s == NULL) s = getenv("LS_COLORS");
        let mut s = std::env::var("TREE_COLORS").ok();
        if s.is_none() {
            s = std::env::var("LS_COLORS").ok();
        }

        // C: if ((s == NULL || strlen(s) == 0) && (flag.force_color || cc)) s = 默认值;
        let s_empty = s.as_deref().map(|v| v.is_empty()).unwrap_or(true);
        if s_empty && (FLAG.force_color || cc) {
            s = Some(DEFAULT_LS_COLORS.to_string());
        }

        // C: if (s == NULL || (!flag.force_color && (flag.nocolor || !isatty(1))))
        // isatty(1) 用 std::io::stdout().is_terminal() 替代
        let s_empty = s.as_deref().map(|v| v.is_empty()).unwrap_or(true);
        if s_empty || (!FLAG.force_color && (FLAG.nocolor || !std::io::stdout().is_terminal())) {
            FLAG.colorize = false;
            return;
        }

        FLAG.colorize = true;

        // C: for(i=0; i < DOT_EXTENSION; i++) color_code[i] = NULL;
        COLOR_CODE = [const { None }; (DOT_EXTENSION + 1) as usize];

        let colors = s.unwrap();
        // C: arg = split(colors, ":", &n);
        let arg = split(&colors, ":");
        for entry in arg {
            // C: c = split(arg[i], "=", &n);
            let c = split(&entry, "=");
            // C: switch(col = cmd(c[0]))
            let col = cmd(c.first().map(String::as_str).unwrap_or(""));
            match col {
                ERROR => {}
                DOT_EXTENSION => {
                    // C: if (c[1]) { 新扩展名节点，头插到 ext 链表 }
                    if c.len() > 1 {
                        let mut e = Box::new(Extensions {
                            // C: e->ext = scopy(c[0]+1);（去掉 '*'）
                            ext: c[0].strip_prefix('*').unwrap_or(&c[0]).to_string(),
                            term_flg: c[1].clone(),
                            nxt: None,
                        });
                        // C: e->nxt = ext; ext = e;
                        // unsafe：访问全局扩展名链表 EXT
                        e.nxt = EXT.take();
                        EXT = Some(e);
                    }
                }
                COL_LINK => {
                    // C: if (c[1] && (strcasecmp("target", c[1]) == 0))
                    if c.len() > 1 && c[1].eq_ignore_ascii_case("target") {
                        FLAG.linktargetcolor = true;
                        // C: color_code[COL_LINK] = "01;36";（字符串常量，应永远不会真正用到）
                        COLOR_CODE[COL_LINK as usize] = Some("01;36");
                        break;
                    }
                    // C: 直接落入 default 分支
                    if c.len() > 1 {
                        // C: color_code[col] = scopy(c[1]);
                        COLOR_CODE[col as usize] = Some(leak_str(c[1].clone()));
                    }
                }
                _ => {
                    // C: if (c[1]) color_code[col] = scopy(c[1]);
                    if c.len() > 1 {
                        COLOR_CODE[col as usize] = Some(leak_str(c[1].clone()));
                    }
                }
            }
        }

        // 确保至少定义了 reset（而非 normal）。假设 ANSI/vt100 支持：
        if COLOR_CODE[COL_LEFTCODE as usize].is_none() {
            COLOR_CODE[COL_LEFTCODE as usize] = Some("\x1B[");
        }
        if COLOR_CODE[COL_RIGHTCODE as usize].is_none() {
            COLOR_CODE[COL_RIGHTCODE as usize] = Some("m");
        }
        if COLOR_CODE[COL_RESET as usize].is_none() {
            COLOR_CODE[COL_RESET as usize] = Some("0");
        }
        if COLOR_CODE[COL_BOLD as usize].is_none() {
            // C: sprintf(color_code[COL_BOLD], "%s1%s", leftcode, rightcode);
            let lc = COLOR_CODE[COL_LEFTCODE as usize].unwrap();
            let rc = COLOR_CODE[COL_RIGHTCODE as usize].unwrap();
            COLOR_CODE[COL_BOLD as usize] = Some(leak_str(format!("{}1{}", lc, rc)));
        }
        if COLOR_CODE[COL_ITALIC as usize].is_none() {
            // C: sprintf(color_code[COL_ITALIC], "%s3%s", leftcode, rightcode);
            let lc = COLOR_CODE[COL_LEFTCODE as usize].unwrap();
            let rc = COLOR_CODE[COL_RIGHTCODE as usize].unwrap();
            COLOR_CODE[COL_ITALIC as usize] = Some(leak_str(format!("{}3{}", lc, rc)));
        }
        if COLOR_CODE[COL_ENDCODE as usize].is_none() {
            // C: sprintf(color_code[COL_ENDCODE], "%s%s%s", leftcode, reset, rightcode);
            let lc = COLOR_CODE[COL_LEFTCODE as usize].unwrap();
            let rs = COLOR_CODE[COL_RESET as usize].unwrap();
            let rc = COLOR_CODE[COL_RIGHTCODE as usize].unwrap();
            COLOR_CODE[COL_ENDCODE as usize] = Some(leak_str(format!("{}{}{}", lc, rs, rc)));
        }
    }
}

// === 原 C 函数：bool print_color(int color) ===
/// 若颜色代码存在则输出 "\033[<code>m" 并返回 true。
fn print_color(color: i32) -> bool {
    // unsafe：读取全局颜色表 COLOR_CODE 并输出到全局输出流
    unsafe {
        // C: if (!color_code[color]) return false;
        let code = match COLOR_CODE[color as usize] {
            Some(c) => c,
            None => return false,
        };
        // parse_dir_colors 保证 LEFT/RIGHTCODE 已初始化（C 中 fputs(NULL) 为崩溃，此处等价 panic）
        let lc = COLOR_CODE[COL_LEFTCODE as usize].expect("LEFTCODE 未初始化");
        let rc = COLOR_CODE[COL_RIGHTCODE as usize].expect("RIGHTCODE 未初始化");
        outbytes!(lc.as_bytes());
        outbytes!(code.as_bytes());
        outbytes!(rc.as_bytes());
    }
    true
}

// === 原 C 函数：void endcolor(void) ===
/// 输出颜色结束码。
pub fn endcolor() {
    // unsafe：读取全局颜色表 COLOR_CODE 并输出
    unsafe {
        if let Some(c) = COLOR_CODE[COL_ENDCODE as usize] {
            outbytes!(c.as_bytes());
        }
    }
}

// === 原 C 函数：void fancy(FILE *out, char *s) ===
/// 将含 \b（粗体）、\f（斜体）、\r（结束颜色）控制符的文本输出到 out。
pub fn fancy(out: &mut dyn Write, s: &str) {
    for &c in s.as_bytes() {
        match c {
            b'\x08' => {
                // C: case '\b': if (flag.colorize && color_code[COL_BOLD]) ...
                // unsafe：读取全局 FLAG 与颜色表
                unsafe {
                    if FLAG.colorize {
                        if let Some(b) = COLOR_CODE[COL_BOLD as usize] {
                            let _ = out.write_all(b.as_bytes());
                        }
                    }
                }
            }
            b'\x0c' => {
                // C: case '\f': ... COL_ITALIC ...
                // unsafe：读取全局 FLAG 与颜色表
                unsafe {
                    if FLAG.colorize {
                        if let Some(b) = COLOR_CODE[COL_ITALIC as usize] {
                            let _ = out.write_all(b.as_bytes());
                        }
                    }
                }
            }
            b'\r' => {
                // C: case '\r': ... COL_ENDCODE ...
                // unsafe：读取全局 FLAG 与颜色表
                unsafe {
                    if FLAG.colorize {
                        if let Some(b) = COLOR_CODE[COL_ENDCODE as usize] {
                            let _ = out.write_all(b.as_bytes());
                        }
                    }
                }
            }
            _ => {
                // C: default: fputc(*s, out);
                let _ = out.write_all(&[c]);
            }
        }
    }
}

// === 原 C 函数：bool color(mode_t mode, const char *name, bool orphan, bool islink) ===
/// 按文件模式与扩展名选择颜色并输出，返回是否输出了颜色。
pub fn color(mode: u32, name: &str, orphan: bool, islink: bool) -> bool {
    // C: if (orphan) { ... COL_MISSING / COL_ORPHAN ... }
    if orphan {
        if islink {
            if print_color(COL_MISSING) {
                return true;
            }
        } else if print_color(COL_ORPHAN) {
            return true;
        }
    }

    // 大概率可安全假设短路求值，但这里按 C 原样逐分支处理：
    match mode & S_IFMT {
        S_IFIFO => print_color(COL_FIFO),
        S_IFCHR => print_color(COL_CHR),
        S_IFDIR => {
            if mode & S_ISVTX != 0 {
                if mode & S_IWOTH != 0
                    && print_color(COL_STICKY_OTHER_WRITABLE) {
                        return true;
                    }
                if mode & S_IWOTH == 0
                    && print_color(COL_STICKY) {
                        return true;
                    }
            }
            if mode & S_IWOTH != 0
                && print_color(COL_OTHER_WRITABLE) {
                    return true;
                }
            print_color(COL_DIR)
        }
        S_IFBLK => print_color(COL_BLK),
        S_IFLNK => print_color(COL_LINK),
        S_IFSOCK => print_color(COL_SOCK),
        S_IFREG => {
            if mode & S_ISUID != 0
                && print_color(COL_SETUID) {
                    return true;
                }
            if mode & S_ISGID != 0
                && print_color(COL_SETGID) {
                    return true;
                }
            if mode & (S_IXUSR | S_IXGRP | S_IXOTH) != 0
                && print_color(COL_EXEC) {
                    return true;
                }

            // 不是目录、链接、特殊设备等：检查扩展名匹配
            // unsafe：遍历全局扩展名链表 EXT
            unsafe {
                let mut e = EXT.as_deref();
                while let Some(en) = e {
                    let xl = en.ext.len();
                    // C: !strcmp((l>xl)?name+(l-xl):name, e->ext) —— 比较文件名后缀（字节语义）
                    let name_bytes = name.as_bytes();
                    let suffix = if name_bytes.len() > xl {
                        &name_bytes[name_bytes.len() - xl..]
                    } else {
                        name_bytes
                    };
                    if suffix == en.ext.as_bytes() {
                        // C: fputs(color_code[COL_LEFTCODE]); fputs(e->term_flg); fputs(color_code[COL_RIGHTCODE]);
                        if let Some(lc) = COLOR_CODE[COL_LEFTCODE as usize] {
                            outbytes!(lc.as_bytes());
                        }
                        outbytes!(en.term_flg.as_bytes());
                        if let Some(rc) = COLOR_CODE[COL_RIGHTCODE as usize] {
                            outbytes!(rc.as_bytes());
                        }
                        return true;
                    }
                    e = en.nxt.as_deref();
                }
            }
            // 普通文件也着色：
            print_color(COL_FILE)
        }
        _ => print_color(COL_NORMAL),
    }
}

// === 原 C 函数：const char *getcharset(void) ===
/// 返回 TREE_CHARSET 环境变量指定的字符集；未设置返回 None。
/// C 中非 __EMX__ 分支仅返回该环境变量（复制到静态缓冲区）。
pub fn getcharset() -> Option<&'static str> {
    // C: cs = getenv("TREE_CHARSET"); if (cs) return strncpy(buffer, cs, 255);
    match std::env::var("TREE_CHARSET") {
        Ok(cs) => Some(leak_str(cs)),
        Err(_) => None,
    }
}

/* ---------------------------------------------------------------------
 * 线条绘制表（C: initlinedraw 中的 cstable）。
 * 注意：C 字符串使用八进制转义（如 \251），Rust 字符串/字节串不支持
 * 八进制转义，已全部转换为十六进制（\xA9）。含非 UTF-8 字节的表项
 * （ANSI 转义、Shift-JIS、EUC 等）使用字节串 b"..."。
 * --------------------------------------------------------------------- */

// C: static const char *ansi[] = { "ANSI", NULL };
const ANSI: &[&str] = &["ANSI"];

// C: static const char *latin1_3[] = { ... NULL };
const LATIN1_3: &[&str] = &[
    "ISO-8859-1",
    "ISO-8859-1:1987",
    "ISO_8859-1",
    "latin1",
    "l1",
    "IBM819",
    "CP819",
    "csISOLatin1",
    "ISO-8859-3",
    "ISO_8859-3:1988",
    "ISO_8859-3",
    "latin3",
    "ls",
    "csISOLatin3",
];

// C: static const char *iso8859_789[] = { ... NULL };
const ISO8859_789: &[&str] = &[
    "ISO-8859-7",
    "ISO_8859-7:1987",
    "ISO_8859-7",
    "ELOT_928",
    "ECMA-118",
    "greek",
    "greek8",
    "csISOLatinGreek",
    "ISO-8859-8",
    "ISO_8859-8:1988",
    "iso-ir-138",
    "ISO_8859-8",
    "hebrew",
    "csISOLatinHebrew",
    "ISO-8859-9",
    "ISO_8859-9:1989",
    "iso-ir-148",
    "ISO_8859-9",
    "latin5",
    "l5",
    "csISOLatin5",
];

// C: static const char *shift_jis[] = { "Shift_JIS", "MS_Kanji", "csShiftJIS", NULL };
const SHIFT_JIS: &[&str] = &["Shift_JIS", "MS_Kanji", "csShiftJIS"];

// C: static const char *euc_jp[] = { ... NULL };
const EUC_JP: &[&str] = &[
    "EUC-JP",
    "Extended_UNIX_Code_Packed_Format_for_Japanese",
    "csEUCPkdFmtJapanese",
];

// C: static const char *euc_kr[] = { "EUC-KR", "csEUCKR", NULL };
const EUC_KR: &[&str] = &["EUC-KR", "csEUCKR"];

// C: static const char *iso2022jp[] = { ... NULL };
const ISO2022JP: &[&str] = &[
    "ISO-2022-JP",
    "csISO2022JP",
    "ISO-2022-JP-2",
    "csISO2022JP2",
];

// C: static const char *ibm_pc[] = { ... NULL };
const IBM_PC: &[&str] = &[
    "IBM437",
    "cp437",
    "437",
    "csPC8CodePage437",
    "IBM852",
    "cp852",
    "852",
    "csPCp852",
    "IBM863",
    "cp863",
    "863",
    "csIBM863",
    "IBM855",
    "cp855",
    "855",
    "csIBM855",
    "IBM865",
    "cp865",
    "865",
    "csIBM865",
    "IBM866",
    "cp866",
    "866",
    "csIBM866",
];

// C: static const char *ibm_ps2[] = { ... NULL };
const IBM_PS2: &[&str] = &[
    "IBM850",
    "cp850",
    "850",
    "csPC850Multilingual",
    "IBM00858",
    "CCSID00858",
    "CP00858",
    "PC-Multilingual-850+euro",
];

// C: static const char *ibm_gr[] = { "IBM869", "cp869", "869", "cp-gr", "csIBM869", NULL };
const IBM_GR: &[&str] = &["IBM869", "cp869", "869", "cp-gr", "csIBM869"];

// C: static const char *gb[] = { "GB2312", "csGB2312", NULL };
const GB: &[&str] = &["GB2312", "csGB2312"];

// C: static const char *utf8[] = { "UTF-8", "utf8", NULL };
const UTF8: &[&str] = &["UTF-8", "utf8"];

// C: static const char *big5[] = { "Big5", "csBig5", NULL };
const BIG5: &[&str] = &["Big5", "csBig5"];

// C: static const char *viscii[] = { "VISCII", "csVISCII", NULL };
const VISCII: &[&str] = &["VISCII", "csVISCII"];

// C: static const char *koi8ru[] = { "KOI8-R", "csKOI8R", "KOI8-U", NULL };
const KOI8RU: &[&str] = &["KOI8-R", "csKOI8R", "KOI8-U"];

// C: static const char *windows[] = { ... NULL };
const WINDOWS: &[&str] = &[
    "ISO-8859-1-Windows-3.1-Latin-1",
    "csWindows31Latin1",
    "ISO-8859-2-Windows-Latin-2",
    "csWindows31Latin2",
    "windows-1250",
    "windows-1251",
    "windows-1253",
    "windows-1254",
    "windows-1255",
    "windows-1256",
    "windows-1256",
    "windows-1257",
];

// C: static const struct linedraw cstable[] = { ... };
// 最后一项 { NULL, "(c)", ... } 为哨兵：name 为空切片表示表结束。
pub const CSTABLE: [Linedraw; 17] = [
    // ANSI（vt100 图形字符）
    Linedraw {
        name: ANSI,
        copy: b"\x1B(0\xA9\x1B(B",
        vert: [b"\x1B(0\x78  \x1B(B", b"\x1B(0\x78 \x1B(B", b"\x1B(0\x78\x1B(B"],
        vert_left: [
            b"\x1B(0\x74\x71\x71\x1B(B",
            b"\x1B(0\x74\x71\x1B(B",
            b"\x1B(0\x74\x1B(B",
        ],
        corner: [
            b"\x1B(0\x6D\x71\x71\x1B(B",
            b"\x1B(0\x6D\x71\x1B(B",
            b"\x1B(0\x6D\x1B(B",
        ],
        ctop: b" [",
        cbot: b" [",
        cmid: b" [",
        cext: b" [",
        csingle: b" [",
    },
    // latin1 / latin3
    Linedraw {
        name: LATIN1_3,
        copy: b"&copy;",
        vert: [b"|  ", b"| ", b"|"],
        vert_left: [b"|--", b"|-", b"+"],
        corner: [b"&middot;--", b"&middot;-", b"&middot;"],
        ctop: b" [",
        cbot: b" [",
        cmid: b" [",
        cext: b" [",
        csingle: b" [",
    },
    // iso8859-7/8/9
    Linedraw {
        name: ISO8859_789,
        copy: b"(c)",
        vert: [b"|  ", b"| ", b"|"],
        vert_left: [b"|--", b"|-", b"+"],
        corner: [b"&middot;--", b"&middot;-", b"&middot;"],
        ctop: b" [",
        cbot: b" [",
        cmid: b" [",
        cext: b" [",
        csingle: b" [",
    },
    // Shift_JIS
    Linedraw {
        name: SHIFT_JIS,
        copy: b"(c)",
        vert: [b"\x84\xA0  ", b"\x84\xA0 ", b"\x84\xA0"],
        vert_left: [b"\x84\xA5\x84\x9F\x84\x9F", b"\x84\xA5\x84\x9F", b"\x84\xA5"],
        corner: [b"\x84\xA4\x84\x9F\x84\x9F", b"\x84\xA4\x84\x9F", b"\x84\xA4"],
        ctop: b" [",
        cbot: b" [",
        cmid: b" [",
        cext: b" [",
        csingle: b" [",
    },
    // EUC-JP
    Linedraw {
        name: EUC_JP,
        copy: b"(c)",
        vert: [b"\xA8\xA2  ", b"\xA8\xA2 ", b"\xA8\xA2"],
        vert_left: [b"\xA8\xA7\xA8\xA1\xA8\xA1", b"\xA8\xA7\xA8\xA1", b"\xA8\xA7"],
        corner: [b"\xA8\xA6\xA8\xA1\xA8\xA1", b"\xA8\xA6\xA8\xA1", b"\xA8\xA6"],
        ctop: b" [",
        cbot: b" [",
        cmid: b" [",
        cext: b" [",
        csingle: b" [",
    },
    // EUC-KR
    Linedraw {
        name: EUC_KR,
        copy: b"(c)",
        vert: [b"\xA6\xA2  ", b"\xA6\xA2 ", b"\xA6\xA2"],
        vert_left: [b"\xA6\xA7\xA6\xA1\xA6\xA1", b"\xA6\xA7\xA6\xA1", b"\xA6\xA7"],
        corner: [b"\xA6\xA6\xA6\xA1\xA6\xA1", b"\xA6\xA6\xA6\xA1", b"\xA6\xA6"],
        ctop: b" [",
        cbot: b" [",
        cmid: b" [",
        cext: b" [",
        csingle: b" [",
    },
    // ISO-2022-JP
    Linedraw {
        name: ISO2022JP,
        copy: b"(c)",
        vert: [b"\x1B$B(\"\x1B(B  ", b"\x1B$B(\"\x1B(B ", b"\x1B$B(\"\x1B(B"],
        vert_left: [
            b"\x1B$B('\x1B$B(!\x1B$B(!\x1B(B",
            b"\x1B$B('\x1B$B(!\x1B(B",
            b"\x1B$B('\x1B(B",
        ],
        corner: [
            b"\x1B$B(&\x1B$B(!\x1B$B(!\x1B(B",
            b"\x1B$B(&\x1B$B(!\x1B(B",
            b"\x1B$B(&\x1B(B",
        ],
        ctop: b" [",
        cbot: b" [",
        cmid: b" [",
        cext: b" [",
        csingle: b" [",
    },
    // IBM PC（cp437 等）
    Linedraw {
        name: IBM_PC,
        copy: b"(c)",
        vert: [b"\xB3  ", b"\xB3 ", b"\xB3"],
        vert_left: [b"\xC3\xC4\xC4", b"\xC3\xC4", b"\xC3"],
        corner: [b"\xC0\xC4\xC4", b"\xC0\xC4", b"\xC0"],
        ctop: b" [",
        cbot: b" [",
        cmid: b" [",
        cext: b" [",
        csingle: b" [",
    },
    // IBM PS/2（cp850 等）
    Linedraw {
        name: IBM_PS2,
        copy: b"\x97",
        vert: [b"\xB3  ", b"\xB3 ", b"\xB3"],
        vert_left: [b"\xC3\xC4\xC4", b"\xC3\xC4", b"\xC3"],
        corner: [b"\xC0\xC4\xC4", b"\xC0\xC4", b"\xC0"],
        ctop: b" [",
        cbot: b" [",
        cmid: b" [",
        cext: b" [",
        csingle: b" [",
    },
    // IBM Greek（cp869）
    Linedraw {
        name: IBM_GR,
        copy: b"\xB8",
        vert: [b"\xB3  ", b"\xB3 ", b"\xB3"],
        vert_left: [b"\xC3\xC4\xC4", b"\xC3\xC4", b"\xC3"],
        corner: [b"\xC0\xC4\xC4", b"\xC0\xC4", b"\xC0"],
        ctop: b" [",
        cbot: b" [",
        cmid: b" [",
        cext: b" [",
        csingle: b" [",
    },
    // GB2312
    Linedraw {
        name: GB,
        copy: b"(c)",
        vert: [b"\xA9\xA6  ", b"\xA9\xA6 ", b"\xA9\xA6"],
        vert_left: [b"\xA9\xC0\xA9\xA4\xA9\xA4", b"\xA9\xC0\xA9\xA4", b"\xA9\xC0"],
        corner: [b"\xA9\xB8\xA9\xA4\xA9\xA4", b"\xA9\xB8\xA9\xA4", b"\xA9\xB8"],
        ctop: b" [",
        cbot: b" [",
        cmid: b" [",
        cext: b" [",
        csingle: b" [",
    },
    // UTF-8
    Linedraw {
        name: UTF8,
        copy: b"\xC2\xA9",
        vert: [
            b"\xE2\x94\x82\xC2\xA0\xC2\xA0",
            b"\xE2\x94\x82\xC2\xA0",
            b"\xE2\x94\x82",
        ],
        vert_left: [
            b"\xE2\x94\x9C\xE2\x94\x80\xE2\x94\x80",
            b"\xE2\x94\x9C\xE2\x94\x80",
            b"\xE2\x94\x9C",
        ],
        corner: [
            b"\xE2\x94\x94\xE2\x94\x80\xE2\x94\x80",
            b"\xE2\x94\x94\xE2\x94\x80",
            b"\xE2\x94\x94",
        ],
        ctop: b" \xE2\x8E\xA7",
        cbot: b" \xE2\x8E\xA9",
        cmid: b" \xE2\x8E\xA8",
        cext: b" \xE2\x8E\xAA",
        csingle: b" {",
    },
    // Big5
    Linedraw {
        name: BIG5,
        copy: b"(c)",
        vert: [b"\xA2x  ", b"\xA2x ", b"\xA2x"],
        vert_left: [b"\xA2u\xA2\x77\xA2\x77", b"\xA2u\xA2\x77", b"\xA2u"],
        corner: [b"\xA2|\xA2\x77\xA2\x77", b"\xA2|\xA2\x77", b"\xA2|"],
        ctop: b" [",
        cbot: b" [",
        cmid: b" [",
        cext: b" [",
        csingle: b" [",
    },
    // VISCII
    Linedraw {
        name: VISCII,
        copy: b"\xF9",
        vert: [b"|  ", b"| ", b"|"],
        vert_left: [b"|--", b"|-", b"+"],
        corner: [b"`--", b"`-", b"`"],
        ctop: b" [",
        cbot: b" [",
        cmid: b" [",
        cext: b" [",
        csingle: b" [",
    },
    // KOI8-R/KOI8-U
    Linedraw {
        name: KOI8RU,
        copy: b"\xBF",
        vert: [b"\x81  ", b"\x81 ", b"\x81"],
        vert_left: [b"\x86\x80\x80", b"\x86\x80", b"\x86"],
        corner: [b"\x84\x80\x80", b"\x84\x80", b"\x84"],
        ctop: b" [",
        cbot: b" [",
        cmid: b" [",
        cext: b" [",
        csingle: b" [",
    },
    // windows 系列
    Linedraw {
        name: WINDOWS,
        copy: b"\xA9",
        vert: [b"|  ", b"| ", b"|"],
        vert_left: [b"|--", b"|-", b"+"],
        corner: [b"`--", b"`-", b"`"],
        ctop: b" [",
        cbot: b" [",
        cmid: b" [",
        cext: b" [",
        csingle: b" [",
    },
    // 哨兵项（C: { NULL, "(c)", ... }）—— name 为空切片表示表结束
    Linedraw {
        name: &[],
        copy: b"(c)",
        vert: [b"|  ", b"| ", b"|"],
        vert_left: [b"|--", b"|-", b"+"],
        corner: [b"`--", b"`-", b"`"],
        ctop: b" [",
        cbot: b" [",
        cmid: b" [",
        cext: b" [",
        csingle: b" [",
    },
];

// === 原 C 函数：void initlinedraw(bool help) ===
/// 按字符集选择线条绘制表；help 为 true 时列出所有支持的字符集。
pub fn initlinedraw(help: bool) {
    if help {
        // C: fprintf(stderr, "Valid charsets include:\n");
        eprintln!("Valid charsets include:");
        // C: for(linedraw=cstable; linedraw->name; ++linedraw)
        //       for(s=linedraw->name; *s; ++s) fprintf(stderr, "  %s\n", *s);
        for ld in CSTABLE.iter() {
            for name in ld.name.iter() {
                eprintln!("  {}", name);
            }
        }
        return;
    }

    // unsafe：读写全局 LINEDRAW 与 FLAG/CHARSET
    unsafe {
        // 如果需要 ANSI 线条，假设用户用的是 vt100：
        // C: if (flag.ansilines) { linedraw = cstable; return; }
        if FLAG.ansilines {
            LINEDRAW = &CSTABLE[0];
            return;
        }
        // C: if (charset) { 遍历表，strcasecmp 匹配字符集名 }
        if let Some(cs) = crate::globals::CHARSET {
            for ld in CSTABLE.iter() {
                for name in ld.name.iter() {
                    // C: if(!strcasecmp(charset, *s)) return;
                    if cs.eq_ignore_ascii_case(name) {
                        LINEDRAW = ld;
                        return;
                    }
                }
            }
        }
        // C: linedraw = cstable + sizeof cstable/sizeof*cstable - 1;（默认最后一项）
        LINEDRAW = CSTABLE.last().unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_strtok_semantics() {
        // strtok 语义：跳过连续分隔符
        assert_eq!(split("a::b:c", ":"), vec!["a", "b", "c"]);
        assert_eq!(split("::a::", ":"), vec!["a"]);
        assert_eq!(split("ab=cd", "="), vec!["ab", "cd"]);
    }

    #[test]
    fn test_cmd_mapping() {
        assert_eq!(cmd("di"), COL_DIR);
        assert_eq!(cmd("ln"), COL_LINK);
        assert_eq!(cmd("rs"), COL_RESET);
        assert_eq!(cmd("ec"), COL_ENDCODE);
        assert_eq!(cmd("*.jpg"), DOT_EXTENSION);
        assert_eq!(cmd(""), ERROR);
        assert_eq!(cmd("unknown"), ERROR);
    }

    #[test]
    fn test_cstable_utf8_table() {
        // UTF-8 表使用标准 Unicode 线条字符
        let utf8_ld = &CSTABLE[11];
        assert_eq!(utf8_ld.name, &["UTF-8", "utf8"]);
        // 竖线 "│"
        assert_eq!(utf8_ld.vert[0], "│\u{A0}\u{A0}".as_bytes());
        // 最后一项是哨兵（空 name）
        assert!(CSTABLE.last().unwrap().name.is_empty());
    }

    #[test]
    fn test_fancy_plain() {
        // 未开启颜色时 fancy 原样输出
        let mut out = Vec::new();
        let mut writer = out.by_ref();
        // unsafe：测试中临时关闭颜色
        unsafe {
            FLAG.colorize = false;
        }
        fancy(&mut writer, "plain text");
        assert_eq!(out, b"plain text");
    }
}
