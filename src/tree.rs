// 文件路径：src/tree.rs
// 对应 C 源文件：tree.h
// 公共定义、结构体、常量和函数原型

use std::time::UNIX_EPOCH;

// 保留原有 C 宏常量
const MINIT: usize = 30;  // 初始分配的目录项数量
const MINC: usize = 20;   // 分配增量

/// 保持原有 C 宏：UNUSED(x) - 用于标记未使用的参数
#[inline(always)]
fn unused<T>(_x: T) {
    // 在 Rust 中，未使用的变量会在编译时警告
    // 这里显式忽略以保持与 C 代码等价
}

/// Flags 结构体对应 C 中的 struct Flags
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Flags {
    // TODO: 将这些单字母标志改为更有意义的名称
    pub a: bool,   // all: 显示隐藏文件
    pub c: bool,   // color: 使用颜色
    pub d: bool,   // dir: 仅显示目录
    pub f: bool,   // first: 文件在前
    pub g: bool,   // group: 显示组
    pub h: bool,   // human-readable: 人类可读的文件大小
    pub l: bool,   // link: 显示符号链接
    pub p: bool,   // prune: 剪枝模式
    pub q: bool,   // quote: 用引号包围名称
    pub s: bool,   // size: 显示文件大小
    pub u: bool,   // uid: 显示用户 ID
    pub D: bool,   // date: 显示修改日期
    pub F: bool,   // file-type: 显示文件类型标记
    pub H: bool,   // hyper: 超链接模式
    pub J: bool,   // json: JSON 输出
    pub N: bool,   // newline: 显示换行
    pub Q: bool,   // quote-again: 用引号包围名称
    pub R: bool,   // recurse: 递归
    pub X: bool,   // xml: XML 输出
    pub inode: bool,  // inode: 显示 inode 编号
    pub dev: bool,    // dev: 显示设备号
    pub si: bool,     // si: 使用 SI 单位
    pub du: bool,     // du: 显示磁盘使用量
    pub prune: bool,  // prune: 剪枝
    pub hyper: bool,  // hyper: 超链接
    pub noindent: bool,   // noindent: 不缩进
    pub force_color: bool, // force_color: 强制使用颜色
    pub nocolor: bool,    // nocolor: 禁用颜色
    pub xdev: bool,       // xdev: 不跨文件系统
    pub noreport: bool,   // noreport: 不显示统计信息
    pub nolinks: bool,    // nolinks: 不显示链接数
    pub ignorecase: bool, // ignorecase: 忽略大小写
    pub matchdirs: bool,  // matchdirs: 匹配目录
    pub fromfile: bool,   // fromfile: 从文件读取参数
    pub metafirst: bool,  // metafirst: 元数据在前
    pub gitignore: bool,  // gitignore: 使用 .gitignore
    pub showinfo: bool,   // showinfo: 显示注释信息
    pub reverse: bool,    // reverse: 反向排序
    pub fflinks: bool,    // fflinks: 跟踪符号链接
    pub htmloffset: bool, // htmloffset: HTML 偏移量
    pub acl: bool,        // acl: 显示 ACL
    pub selinux: bool,    // selinux: 显示 SELinux 上下文
    pub condense_singletons: bool, // condense_singletons: 合并单例
    pub colorize: bool,   // colorize: 颜色化
    pub ansilines: bool,  // ansilines: ANSI 行
    pub linktargetcolor: bool, // linktargetcolor: 链接目标颜色
    pub remove_space: bool, // remove_space: 移除空格
    pub flimit: i32,      // flimit: 文件限制
    pub compress_indent: i32, // compress_indent: 压缩缩进
}

/// _info 结构体对应 C 中的 struct _info
#[repr(C)]
#[derive(Debug, Default)]
pub struct Info {
    pub name: String,           // 文件名
    pub lnk: String,            // 符号链接目标
    pub isdir: bool,            // 是否是目录
    pub issok: bool,            // 是否是套接字
    pub isfifo: bool,           // 是否是 FIFO
    pub isexe: bool,            // 是否是可执行文件
    pub isfile: bool,           // 是否是普通文件
    pub orphan: bool,           // 是否是孤儿文件
    #[cfg(target_os = "linux")]
    pub hasacl: bool,           // 是否有 ACL
    #[cfg(target_os = "linux")]
    pub secontext: Option<String>, // SELinux 上下文
    pub mode: u32,              // 文件模式（权限）
    pub lnkmode: u32,           // 符号链接模式
    pub uid: u32,               // 用户 ID
    pub gid: u32,               // 组 ID
    pub size: i64,              // 文件大小
    pub atime: i64,             // 访问时间
    pub ctime: i64,             // 状态改变时间
    pub mtime: i64,             // 修改时间
    pub dev: u64,               // 设备号
    pub ldev: u64,              // 符号链接设备号
    pub inode: u64,             // inode 编号
    pub linode: u64,            // 符号链接 inode 编号
    #[cfg(__EMX__)]
    pub attr: i32,              // OS/2 属性
    pub err: Option<String>,    // 错误信息
    pub tag: Option<String>,    // 标签
    pub condensed: usize,       // 压缩计数
    pub comment: Vec<String>,   // 注释列表
    pub child: Option<Vec<Info>>, // 子节点（目录）
    pub next: Option<Box<Info>>,  // 下一个节点（链表）
    pub tchild: Option<Box<Info>>, // 树形子节点
}

impl Info {
    /// 从 Metadata 创建 Info
    pub fn from_metadata(
        name: String,
        metadata: &std::fs::Metadata,
        isdir: bool,
        lnk: Option<String>,
    ) -> Self {
        let size = metadata.len() as i64;
        let mtime = metadata.modified()
            .map(|t| t.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() as i64)
            .unwrap_or(0);
        let atime = metadata.accessed()
            .map(|t| t.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() as i64)
            .unwrap_or(0);
        let ctime = metadata.created()
            .map(|t| t.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() as i64)
            .unwrap_or(0);

        let (uid, gid) = if cfg!(target_os = "linux") {
            #[cfg(target_os = "linux")]
            {
                (metadata.permissions().uid() as u32, metadata.permissions().gid() as u32)
            }
            #[cfg(not(target_os = "linux"))]
            {
                (0, 0)
            }
        } else {
            (0, 0)
        };

        let (dev, inode) = if cfg!(target_os = "linux") {
            #[cfg(target_os = "linux")]
            {
                (metadata.dev() as u64, metadata.ino() as u64)
            }
            #[cfg(not(target_os = "linux"))]
            {
                (0, 0)
            }
        } else {
            (0, 0)
        };

        // 从文件模式中提取权限
        let mode = metadata.permissions().as_u32() as u32;

        Self {
            name,
            lnk: lnk.unwrap_or_default(),
            isdir,
            issok: false,
            isfifo: false,
            isexe: false,
            isfile: false,
            orphan: false,
            #[cfg(target_os = "linux")]
            hasacl: false,
            #[cfg(target_os = "linux")]
            secontext: None,
            mode,
            lnkmode: 0,
            uid,
            gid,
            size,
            atime,
            ctime,
            mtime,
            dev,
            ldev: 0,
            inode,
            linode: 0,
            #[cfg(__EMX__)]
            attr: 0,
            err: None,
            tag: None,
            condensed: 0,
            comment: Vec::new(),
            child: None,
            next: None,
            tchild: None,
        }
    }
}

/// extensions 结构体对应 C 中的 struct extensions（用于颜色扩展名）
#[repr(C)]
#[derive(Debug, Clone)]
pub struct Extensions {
    pub ext: String,           // 扩展名
    pub term_flg: String,      // 终端标志
    pub nxt: Option<Box<Extensions>>, // 下一个节点
}

/// linedraw 结构体对应 C 中的 struct linedraw（用于绘制线条）
#[derive(Debug, Clone, Copy)]
pub struct Linedraw {
    pub name: &'static [&'static str],  // 名称数组
    pub copy: &'static str,            // 复制字符
    pub vert: [&'static str; 3],        // 垂直线（3 种）
    pub vert_left: [&'static str; 3],   // 左垂直线
    pub corner: [&'static str; 3],      // 角落线
    pub ctop: &'static str,             // 顶部颜色
    pub cbot: &'static str,             // 底部颜色
    pub cmid: &'static str,             // 中部颜色
    pub cext: &'static str,             // 扩展名颜色
    pub csingle: &'static str,          // 单个文件颜色
}

/// meta_ids 结构体对应 C 中的 struct meta_ids（用于元数据 ID）
#[repr(C)]
#[derive(Debug, Clone)]
pub struct MetaIds {
    pub name: String,       // 名称
    pub term_flg: String,   // 终端标志
}

/// pattern 结构体对应 C 中的 struct pattern（用于过滤模式）
#[repr(C)]
#[derive(Debug, Clone)]
pub struct Pattern {
    pub pattern: String,    // 模式
    pub relative: i32,      // 相对标志
    pub next: Option<Box<Pattern>>, // 下一个节点
}

/// ignorefile 结构体对应 C 中的 struct ignorefile（用于忽略文件）
#[repr(C)]
#[derive(Debug, Clone)]
pub struct Ignorefile {
    pub path: String,              // 路径
    pub remove: Option<Box<Pattern>>, // 移除模式
    pub reverse: Option<Box<Pattern>>, // 反向模式
    pub next: Option<Box<Ignorefile>>, // 下一个节点
}

/// comment 结构体对应 C 中的 struct comment（用于注释）
#[repr(C)]
#[derive(Debug, Clone)]
pub struct Comment {
    pub pattern: Option<Box<Pattern>>, // 模式
    pub desc: Vec<String>,           // 描述
    pub next: Option<Box<Comment>>,   // 下一个节点
}

/// infofile 结构体对应 C 中的 struct infofile（用于信息文件）
#[repr(C)]
#[derive(Debug, Clone)]
pub struct Infofile {
    pub path: String,              // 路径
    pub comments: Option<Box<Comment>>, // 注释
    pub next: Option<Box<Infofile>>, // 下一个节点
}

/// totals 结构体对应 C 中的 struct totals（用于统计）
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Totals {
    pub files: usize,   // 文件数
    pub dirs: usize,    // 目录数
    pub size: i64,      // 总大小
}

/// listingcalls 结构体对应 C 中的 struct listingcalls（用于列表调用回调）
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ListingCalls {
    pub intro: fn(),                                    // 开场
    pub outtro: fn(),                                   // 结束
    pub printinfo: fn(String, &Info, i32) -> i32,       // 打印信息
    pub printfile: fn(String, String, &Info, bool) -> i32, // 打印文件
    pub error: fn(String) -> i32,                       // 错误处理
    pub newline: fn(&Info, i32, bool, bool),            // 换行
    pub close: fn(&Info, i32, bool),                    // 关闭
    pub report: fn(Totals),                             // 报告统计
}

// 函数原型声明（简化版本）

pub fn color(_mode: u32, _name: &str, _orphan: bool, _islink: bool) -> Option<String> {
    None
}

pub fn endcolor() {}
pub fn fancy(_out: &mut impl std::io::Write, _s: &str) {}
pub fn getcharset() -> &'static str {
    "UTF-8"
}
pub fn initlinedraw(_help: bool) {}

pub fn file_getfulltree(
    _d: &str,
    _lev: u64,
    _dev: u64,
    _size: &mut i64,
    _err: &mut Option<String>,
) -> Option<Vec<Info>> {
    None
}

pub fn tabedfile_getfulltree(
    _d: &str,
    _lev: u64,
    _dev: u64,
    _size: &mut i64,
    _err: &mut Option<String>,
) -> Option<Vec<Info>> {
    None
}

pub fn gittrim(_s: &mut String) {}
pub fn new_pattern(_pattern: &str) -> Option<Pattern> {
    Some(Pattern {
        pattern: _pattern.to_string(),
        relative: 0,
        next: None,
    })
}
pub fn gitignore_search(_startpath: &str, _depth: i32) -> Option<Ignorefile> {
    None
}
pub fn filtercheck(_path: &str, _name: &str, _isdir: bool) -> bool {
    true
}
pub fn new_ignorefile(_basepath: &str, _path: &str, _checkparents: bool) -> Option<Ignorefile> {
    None
}
pub fn push_filterstack(_ig: Ignorefile) {}
pub fn pop_filterstack() -> Option<Ignorefile> {
    None
}
pub fn flush_filterstack() -> Option<Ignorefile> {
    None
}

pub fn init_hashes() {}
pub fn uidtoname(_uid: u32) -> Option<String> {
    None
}
pub fn gidtoname(_gid: u32) -> Option<String> {
    None
}
pub fn findino(_inode: u64, _dev: u64) -> bool {
    false
}
pub fn saveino(_inode: u64, _dev: u64) {}

pub fn url_encode(_fd: &mut std::fs::File, _s: &str) -> bool {
    true
}
pub fn json_indent(_maxlevel: i32) {}
pub fn json_fillinfo(_ent: &Info) {}
pub fn json_intro() {}
pub fn json_outtro() {}
pub fn json_printinfo(_dirname: String, _file: &Info, _level: i32) -> i32 {
    0
}
pub fn json_printfile(_dirname: String, _filename: String, _file: &Info, _descend: bool) -> i32 {
    0
}
pub fn json_error(_error: String) -> i32 {
    0
}
pub fn json_newline(_file: &Info, _level: i32, _postdir: bool, _needcomma: bool) {}
pub fn json_close(_file: &Info, _level: i32, _needcomma: bool) {}
pub fn json_report(_tot: Totals) {}

pub fn null_intro() {}
pub fn null_outtro() {}
pub fn null_close(_file: &Info, _level: i32, _needcomma: bool) {}
pub fn emit_tree(_dirname: Vec<String>, _needfulltree: bool) {}
pub fn listdir(
    _dirname: &str,
    _dir: &mut Vec<Info>,
    _lev: i32,
    _dev: u64,
    _hasfulltree: bool,
) -> Totals {
    Totals {
        files: 0,
        dirs: 0,
        size: 0,
    }
}

pub fn setoutput(_filename: &str) {}
pub fn print_version(_nl: bool) {}
pub fn usage(_exit_code: i32) {}
pub fn push_files(_dir: &str, _ig: &mut Option<Ignorefile>, _inf: &mut Option<Infofile>, _top: bool) {}
pub fn patignore(_name: &str, _isdir: bool, _checkpaths: bool) -> i32 {
    0
}
pub fn patinclude(_name: &str, _isdir: bool, _checkpaths: bool) -> i32 {
    0
}
pub fn unix_getfulltree(
    _d: &str,
    _lev: u64,
    _dev: u64,
    _size: &mut i64,
    _err: &mut Option<String>,
) -> Option<Vec<Info>> {
    None
}
pub fn read_dir(
    _dir: &str,
    _n: &mut Option<usize>,
    _infotop: i32,
) -> Option<Vec<Info>> {
    None
}

pub fn filesfirst(_a: &Option<Vec<Info>>, _b: &Option<Vec<Info>>) -> i32 {
    0
}
pub fn dirsfirst(_a: &Option<Vec<Info>>, _b: &Option<Vec<Info>>) -> i32 {
    0
}
pub fn alnumsort(_a: &Option<Vec<Info>>, _b: &Option<Vec<Info>>) -> i32 {
    0
}
pub fn versort(_a: &Option<Vec<Info>>, _b: &Option<Vec<Info>>) -> i32 {
    0
}
pub fn reversealnumsort(_a: &Option<Vec<Info>>, _b: &Option<Vec<Info>>) -> i32 {
    0
}
pub fn mtimesort(_a: &Option<Vec<Info>>, _b: &Option<Vec<Info>>) -> i32 {
    0
}
pub fn ctimesort(_a: &Option<Vec<Info>>, _b: &Option<Vec<Info>>) -> i32 {
    0
}
pub fn sizecmp(_a: i64, _b: i64) -> i32 {
    0
}
pub fn fsizesort(_a: &Option<Vec<Info>>, _b: &Option<Vec<Info>>) -> i32 {
    0
}

pub fn patmatch(_buf: &str, _pat: &str, _isdir: bool) -> bool {
    true
}
pub fn indent(_maxlevel: i32) {}
pub fn free_dir(_dir: &mut Vec<Info>) {}
pub fn prot(_mode: u32) -> String {
    format!("{:o}", _mode)
}
pub fn do_date(_time: i64) -> String {
    // TODO: 实现日期格式化
    String::new()
}
pub fn printit(_s: &str) {
    println!("{}", _s);
}
pub fn psize(_buf: &mut String, _size: i64) -> i32 {
    _buf.push_str(&format!("{}", _size));
    0
}
pub fn Ftype(_mode: u32) -> char {
    if _mode & 0o040000 != 0 {
        'd'
    } else if _mode & 0o100000 != 0 {
        'l'
    } else if _mode & 0o011000 != 0 {
        'p'
    } else if _mode & 0o020000 != 0 {
        's'
    } else {
        '-'
    }
}
pub fn stat2info(_st: &std::fs::Metadata) -> Option<Info> {
    None
}
pub fn fillinfo(_buf: &mut String, _ent: &Info) -> i32 {
    0
}

pub fn unix_printinfo(_dirname: String, _file: &Info, _level: i32) -> i32 {
    0
}
pub fn unix_printfile(_dirname: String, _filename: String, _file: &Info, _descend: bool) -> i32 {
    0
}
pub fn unix_error(_error: String) -> i32 {
    0
}
pub fn unix_newline(_file: &Info, _level: i32, _postdir: bool, _needcomma: bool) {}
pub fn unix_report(_tot: Totals) {}

pub fn pathconcat(_str: &str, _args: Vec<&str>) -> String {
    format!("{}", _str)
}
pub fn is_singleton(_dir: &Info) -> bool {
    false
}
pub unsafe fn xmalloc(_size: usize) -> *mut u8 {
    std::alloc::alloc(std::alloc::Layout::from_size_align_unchecked(_size, 1))
}
pub unsafe fn xrealloc(_ptr: *mut u8, _size: usize) -> *mut u8 {
    if _ptr.is_null() {
        return xmalloc(_size);
    }
    let layout = std::alloc::Layout::from_size_align_unchecked(_size, 1);
    std::alloc::realloc(_ptr, layout, _size)
}

pub fn xml_intro() {}
pub fn xml_outtro() {}
pub fn xml_printinfo(_dirname: String, _file: &Info, _level: i32) -> i32 {
    0
}
pub fn xml_printfile(_dirname: String, _filename: String, _file: &Info, _descend: bool) -> i32 {
    0
}
pub fn xml_error(_error: String) -> i32 {
    0
}
pub fn xml_newline(_file: &Info, _level: i32, _postdir: bool, _needcomma: bool) {}
pub fn xml_close(_file: &Info, _level: i32, _needcomma: bool) {}
pub fn xml_report(_tot: Totals) {}

#[cfg(not(target_os = "linux"))]
pub fn strverscmp(s1: &str, s2: &str) -> i32 {
    strverscmp::strverscmp(s1, s2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strverscmp_no_digit() {
        assert_eq!(strverscmp("no digit", "no digit"), 0);
    }

    #[test]
    fn test_strverscmp_item_99_vs_100() {
        assert!(strverscmp("item#99", "item#100") < 0);
    }

    #[test]
    fn test_strverscmp_alpha1_vs_alpha001() {
        assert!(strverscmp("alpha1", "alpha001") > 0);
    }

    #[test]
    fn test_strverscmp_part1_f012_vs_part1_f01() {
        assert!(strverscmp("part1_f012", "part1_f01") > 0);
    }

    #[test]
    fn test_strverscmp_foo_009_vs_foo_0() {
        assert!(strverscmp("foo.009", "foo.0") < 0);
    }

    #[test]
    fn test_strverscmp_equal() {
        assert_eq!(strverscmp("file1", "file1"), 0);
    }

    #[test]
    fn test_strverscmp_less() {
        assert!(strverscmp("file1", "file2") < 0);
    }

    #[test]
    fn test_strverscmp_greater() {
        assert!(strverscmp("file2", "file1") > 0);
    }
}
