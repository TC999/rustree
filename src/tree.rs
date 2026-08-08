// 文件路径：src/tree.rs
// 对应 C 源文件：tree.h
// 公共定义、结构体、常量与类型别名。
// 原 C 头文件中的函数原型不在此实现，其实现分布在各自对应模块中，
// 此处保留为注释块以便对照。


/* =====================================================================
 * 宏常量（对应 tree.h 中的 #define）
 * ===================================================================== */

// 路径缓冲上限（tree.h: #ifndef PATH_MAX / #define PATH_MAX 4096）
pub const PATH_MAX: usize = 4096;

// 全局 .info 注释文件的默认路径（tree.h: #define INFO_PATH）
pub const INFO_PATH: &str = "/usr/share/finfo/global_info";

// 初始分配的目录项数量（tree.h: #define MINIT 30）
pub const MINIT: usize = 30;

// 目录项数组的分配增量（tree.h: #define MINC 20）
#[allow(dead_code)]
pub const MINC: usize = 20;

// 半年秒数，用于 do_date() 判断时间是否"过近"（tree.c: #define SIXMONTHS）
pub const SIXMONTHS: i64 = 6 * 31 * 24 * 60 * 60;

// tree.h 中 __linux__ 分支的宏：STDDATA_FD 环境变量名与默认 fd 号
#[cfg(target_os = "linux")]
pub const ENV_STDDATA_FD: &str = "STDDATA_FD";
#[cfg(target_os = "linux")]
pub const STDDATA_FILENO: i32 = 3;

/* ---------------------------------------------------------------------
 * 文件模式位常量（原 C 来自 <sys/stat.h>，POSIX 标准值）
 * --------------------------------------------------------------------- */
pub const S_IFMT: u32 = 0o170000; // 类型掩码
pub const S_IFSOCK: u32 = 0o140000; // 套接字
pub const S_IFLNK: u32 = 0o120000; // 符号链接
pub const S_IFREG: u32 = 0o100000; // 普通文件
pub const S_IFBLK: u32 = 0o060000; // 块设备
pub const S_IFDIR: u32 = 0o040000; // 目录
pub const S_IFCHR: u32 = 0o020000; // 字符设备
pub const S_IFIFO: u32 = 0o010000; // FIFO
pub const S_ISUID: u32 = 0o004000; // set-user-ID
pub const S_ISGID: u32 = 0o002000; // set-group-ID
pub const S_ISVTX: u32 = 0o001000; // sticky 位
pub const S_IRWXU: u32 = 0o700; // 属主 rwx
pub const S_IRUSR: u32 = 0o400; // 属主 r
#[allow(dead_code)]
pub const S_IWUSR: u32 = 0o200; // 属主 w
pub const S_IXUSR: u32 = 0o100; // 属主 x
pub const S_IRWXG: u32 = 0o070; // 组 rwx
#[allow(dead_code)]
pub const S_IRGRP: u32 = 0o040; // 组 r
#[allow(dead_code)]
pub const S_IWGRP: u32 = 0o020; // 组 w
pub const S_IXGRP: u32 = 0o010; // 组 x
pub const S_IRWXO: u32 = 0o007; // 其他 rwx
#[allow(dead_code)]
pub const S_IROTH: u32 = 0o004; // 其他 r
pub const S_IWOTH: u32 = 0o002; // 其他 w
pub const S_IXOTH: u32 = 0o001; // 其他 x

/* ---------------------------------------------------------------------
 * 类型别名
 * --------------------------------------------------------------------- */

// C: struct _info **(*getfulltree)(char *d, u_long lev, dev_t dev, off_t *size, char **err)
// 读取完整目录树的函数指针类型（--fromfile/--fromtabfile 会替换它）
pub type Getfulltree =
    fn(d: &str, lev: u64, dev: u64, size: &mut i64, err: &mut Option<String>) -> Option<Vec<Info>>;

// C: int (*sortfunc)(struct _info **, struct _info **)
// 顶层排序比较器类型（qsort 比较器，返回负/零/正）
pub type SortFn = fn(a: &Info, b: &Info) -> i32;

/* =====================================================================
 * struct Flags —— 对应 tree.c / tree.h 中的全局选项标志
 * ===================================================================== */
// 字段名刻意与 C 源码 struct Flags 保持一致（D/F/H/J/N/Q/R/X 为大写单字母），
// 因此关闭 snake_case 命名检查。
#[allow(non_snake_case)]
#[derive(Debug, Clone, Copy)]
pub struct Flags {
    // TODO: 将这些单字母标志改为更有意义的名称（保留原 C 注释）
    pub a: bool, // 显示所有文件（含隐藏文件）
    pub c: bool, // 使用 ctime（状态改变时间）排序/显示
    pub d: bool, // 仅列出目录
    pub f: bool, // 打印每个文件的完整路径前缀
    pub g: bool, // 显示文件所属组名或 GID
    pub h: bool, // 以人类可读方式打印文件大小
    pub l: bool, // 将符号链接当作目录跟随
    pub p: bool, // 打印每个文件的保护位
    pub q: bool, // 将不可打印字符打印为 '?'
    pub s: bool, // 打印每个文件的大小（字节）
    pub u: bool, // 显示文件属主名或 UID
    pub D: bool, // 打印最后修改时间（或 -c 时的状态改变时间）
    pub F: bool, // 为条目追加指示符（'/'、'*'、'=' 等）
    pub H: bool, // HTML 输出模式
    pub J: bool, // JSON 输出模式
    pub N: bool, // 原样打印不可打印字符
    pub Q: bool, // 用双引号包裹文件名
    pub R: bool, // 达到最大目录深度时重新运行 tree
    pub X: bool, // XML 输出模式
    pub inode: bool, // 打印每个文件的 inode 号
    pub dev: bool, // 打印每个文件所属的设备 ID
    pub si: bool, // 类似 -h，但使用 SI 单位（1000 进制）
    pub du: bool, // 通过内容计算目录大小
    pub prune: bool, // 从输出中剪掉空目录
    pub hyper: bool, // 开启 OSC 8 终端超链接
    pub noindent: bool, // 不打印缩进行（-i）
    pub force_color: bool, // 始终开启颜色（-C）
    pub nocolor: bool, // 始终关闭颜色（-n）
    pub xdev: bool, // 仅停留在当前文件系统（-x）
    pub noreport: bool, // 关闭树列表末尾的文件/目录计数
    pub nolinks: bool, // 关闭 HTML 输出中的超链接
    pub ignorecase: bool, // 模式匹配时忽略大小写
    pub matchdirs: bool, // 在 -P 模式匹配中包含目录名
    pub fromfile: bool, // 从文件读取路径（--fromfile/--fromtabfile）
    pub metafirst: bool, // 将元数据打印在每行开头
    pub gitignore: bool, // 使用 .gitignore 文件过滤
    pub showinfo: bool, // 打印 .info 文件中的信息
    pub reverse: bool, // 反转排序顺序（-r）
    pub fflinks: bool, // 使用 --fromfile 时处理链接信息
    pub htmloffset: bool, // HTML 输出时对 baseHREF 做偏移
    // Linux 专属字段：Windows 编译时不被读取（cfg(target_os="linux") 分支）
    #[cfg_attr(not(target_os = "linux"), allow(dead_code))]
    pub acl: bool, // 若存在 ACL 则在权限后打印 '+'
    #[cfg_attr(not(target_os = "linux"), allow(dead_code))]
    pub selinux: bool, // 打印 SELinux 安全标签
    pub condense_singletons: bool, // 将单例目录压缩为一行输出
    pub colorize: bool, // 是否使用颜色（由 parse_dir_colors 决定）
    pub ansilines: bool, // 使用 ANSI 图形缩进线（-A）
    pub linktargetcolor: bool, // 对链接目标着色（LS_COLORS 的 ln=target）
    pub remove_space: bool, // 移除缩进线后的空格
    pub flimit: i32, // 文件数量上限（--filelimit）
    pub compress_indent: i32, // 缩进压缩级别（--compress）
}

impl Flags {
    // 对应 C 中 memset(&flag, 0, sizeof(flag)) 的初始状态
    pub const fn new() -> Flags {
        Flags {
            a: false, c: false, d: false, f: false, g: false, h: false, l: false,
            p: false, q: false, s: false, u: false,
            D: false, F: false, H: false, J: false, N: false, Q: false, R: false, X: false,
            inode: false, dev: false, si: false, du: false, prune: false, hyper: false,
            noindent: false, force_color: false, nocolor: false, xdev: false, noreport: false,
            nolinks: false, ignorecase: false, matchdirs: false, fromfile: false,
            metafirst: false, gitignore: false, showinfo: false, reverse: false, fflinks: false,
            htmloffset: false, acl: false, selinux: false, condense_singletons: false,
            colorize: false, ansilines: false, linktargetcolor: false, remove_space: false,
            flimit: 0, compress_indent: 0,
        }
    }
}

impl Default for Flags {
    fn default() -> Self {
        Flags::new()
    }
}

/* =====================================================================
 * struct _info —— 对应 tree.h 中的 struct _info
 * ===================================================================== */
#[derive(Debug, Default, Clone)]
pub struct Info {
    pub name: String, // C: char *name
    pub lnk: Option<String>, // C: char *lnk（符号链接目标，NULL 表示非链接）
    pub isdir: bool,
    pub issok: bool, // 是否为套接字
    pub isfifo: bool, // 是否为 FIFO
    pub isexe: bool, // 是否可执行
    pub orphan: bool, // 是否为孤儿链接（目标不存在）
    #[cfg(target_os = "linux")]
    pub hasacl: bool, // 是否有 POSIX ACL
    #[cfg(target_os = "linux")]
    pub secontext: Option<String>, // SELinux 上下文（strhash 缓存的字符串）
    pub mode: u32, // C: mode_t mode（lstat 的结果）
    pub lnkmode: u32, // C: mode_t lnkmode（stat 跟随链接的结果）
    pub uid: u32,
    pub gid: u32,
    pub size: i64, // C: off_t size
    pub atime: i64, // C: time_t atime
    pub ctime: i64,
    pub mtime: i64,
    pub dev: u64, // C: dev_t dev（stat 跟随链接）
    pub ldev: u64, // C: dev_t ldev（lstat）
    pub inode: u64, // C: ino_t inode（stat 跟随链接）
    pub linode: u64, // C: ino_t linode（lstat）
    pub err: Option<String>, // C: char *err（目录打开错误等信息）
    pub tag: Option<&'static str>, // C: const char *tag（XML 输出使用的类型标签，指向字符串常量）
    pub condensed: usize, // C: size_t condensed（--condense 压缩的层数）
    pub comment: Vec<String>, // C: char **comment（.info 注释行，NULL 结尾）
    pub child: Option<Vec<Info>>, // C: struct _info **child（子目录项数组）
    pub next: Option<Box<Info>>, // C: struct _info *next（链表后继）
    pub tchild: Option<Box<Info>>, // C: struct _info *tchild（文件树子节点，file.c 使用）
}

/* =====================================================================
 * struct extensions —— 对应 color.c 的 struct extensions（颜色扩展名表）
 * ===================================================================== */
#[derive(Debug, Default, Clone)]
pub struct Extensions {
    pub ext: String, // 扩展名（如 "bat"）
    pub term_flg: String, // 终端颜色代码
    pub nxt: Option<Box<Extensions>>, // 链表后继
}

/* =====================================================================
 * struct linedraw —— 对应 color.c 的 struct linedraw（线条绘制字符表）
 * 注意：原 C 表项含非 UTF-8 字节（ANSI 转义序列、Shift-JIS/EUC 双字节字符），
 *       因此字符字段一律使用字节切片 &'static [u8] 而非 &str。
 * ===================================================================== */
#[derive(Debug, Clone, Copy)]
pub struct Linedraw {
    pub name: &'static [&'static str], // 该表适用的字符集名称列表（NULL 结尾 → 空切片为哨兵）
    pub copy: &'static [u8], // 版权符号
    pub vert: [&'static [u8]; 3], // 垂直连接线（3 种压缩级别）
    pub vert_left: [&'static [u8]; 3], // 左侧垂直连接线
    pub corner: [&'static [u8]; 3], // 角落连接线
    pub ctop: &'static [u8], // 注释顶部标记
    pub cbot: &'static [u8], // 注释底部标记
    pub cmid: &'static [u8], // 注释中部标记
    pub cext: &'static [u8], // 注释扩展标记
    pub csingle: &'static [u8], // 单行注释标记
}

/* =====================================================================
 * struct meta_ids —— 对应 color.c 的 struct meta_ids（元数据 ID 表）
 * 原 C 源码中该结构体未实际使用，仅为头文件定义。
 * ===================================================================== */
#[derive(Debug, Default, Clone)]
#[allow(dead_code)]
pub struct MetaIds {
    pub name: String,
    pub term_flg: String,
}

/* =====================================================================
 * struct pattern —— 对应 filter.c 的 struct pattern（过滤模式）
 * ===================================================================== */
#[derive(Debug, Default, Clone)]
pub struct Pattern {
    pub pattern: String, // 模式字符串
    pub relative: i32, // 是否为相对模式（不含 '/'）
    pub next: Option<Box<Pattern>>, // 链表后继
}

/* =====================================================================
 * struct ignorefile —— 对应 filter.c 的 struct ignorefile（gitignore 文件）
 * ===================================================================== */
#[derive(Debug, Default, Clone)]
pub struct Ignorefile {
    pub path: String, // 基准路径
    pub remove: Option<Box<Pattern>>, // 移除模式链表
    pub reverse: Option<Box<Pattern>>, // 反向（!）模式链表
    pub next: Option<Box<Ignorefile>>, // 链表后继
}

/* =====================================================================
 * struct comment —— 对应 info.c 的 struct comment（.info 注释块）
 * ===================================================================== */
#[derive(Debug, Default, Clone)]
pub struct Comment {
    pub pattern: Option<Box<Pattern>>, // 关联的模式链表
    pub desc: Vec<String>, // 注释描述行（C: char **desc）
    pub next: Option<Box<Comment>>, // 链表后继
}

/* =====================================================================
 * struct infofile —— 对应 info.c 的 struct infofile（.info 文件）
 * ===================================================================== */
#[derive(Debug, Default, Clone)]
pub struct Infofile {
    pub path: String,
    pub comments: Option<Box<Comment>>, // 注释块链表
    pub next: Option<Box<Infofile>>, // 链表后继
}

/* =====================================================================
 * struct totals —— 对应 list.c 的 struct totals（统计信息）
 * ===================================================================== */
#[derive(Debug, Clone, Copy, Default)]
pub struct Totals {
    pub files: usize, // C: size_t files
    pub dirs: usize, // C: size_t dirs
    pub size: i64, // C: off_t size
}

/* =====================================================================
 * struct listingcalls —— 对应 list.c 的 struct listingcalls（输出回调集合）
 * 不同输出格式（终端/HTML/XML/JSON）通过替换这组函数指针切换。
 * ===================================================================== */
#[derive(Debug, Clone, Copy)]
pub struct ListingCalls {
    pub intro: fn(), // 输出开始
    pub outtro: fn(), // 输出结束
    // file 为 &mut：xml_printinfo 会设置 file->tag（C 中 file 指针可变）
    pub printinfo: fn(dirname: &str, file: Option<&mut Info>, level: i32) -> i32, // 打印条目元数据
    pub printfile: fn(dirname: &str, filename: &str, file: Option<&Info>, descend: i32) -> i32, // 打印条目名
    pub error: fn(error: &str) -> i32, // 打印错误
    pub newline: fn(file: Option<&Info>, level: i32, postdir: i32, needcomma: bool), // 换行
    pub close: fn(file: Option<&Info>, level: i32, needcomma: bool), // 关闭条目
    pub report: fn(tot: Totals), // 打印统计报告
}

/* =====================================================================
 * 平台 stat 抽象（对应 C 的 struct stat / lstat() / stat() 系统调用）
 * Rust 标准库的 std::os::unix::fs::MetadataExt 提供全部所需字段；
 * 非 Unix 平台（如 Windows）降级为有限字段，保证可编译。
 * ===================================================================== */
#[derive(Debug, Clone, Copy, Default)]
pub struct StatFields {
    pub mode: u32, // st_mode
    pub uid: u32, // st_uid
    pub gid: u32, // st_gid
    pub size: i64, // st_size（off_t）
    pub atime: i64, // st_atime
    pub ctime: i64, // st_ctime
    pub mtime: i64, // st_mtime
    pub dev: u64, // st_dev
    pub inode: u64, // st_ino
}

/* =====================================================================
 * tree.h 中的函数原型（实现分布在各对应模块中）：
 *
 * /* color.c */
 * void parse_dir_colors(void);                      -> color::parse_dir_colors
 * bool color(mode_t mode, const char *name, bool orphan, bool islink);
 * void endcolor(void);
 * void fancy(FILE *out, char *s);
 * void initlinedraw(bool help);
 *（const char *getcharset(void) 已随 --charset/TREE_CHARSET 机制移除）
 *
 * /* file.c */
 * struct _info **file_getfulltree(char *d, u_long lev, dev_t dev, off_t *size, char **err);
 * struct _info **tabedfile_getfulltree(...);
 *
 * /* filter.c */
 * void gittrim(char *s);
 * struct pattern *new_pattern(char *pattern);
 * struct ignorefile *gitignore_search(const char *startpath, int depth);
 * bool filtercheck(const char *path, const char *name, int isdir);
 * struct ignorefile *new_ignorefile(const char *basepath, const char *path, bool checkparents);
 * void push_filterstack(struct ignorefile *ig);
 * struct ignorefile *pop_filterstack(void);
 * struct ignorefile *flush_filterstack(void);
 *
 * /* hash.c */
 * void init_hashes(void);
 * char *uidtoname(uid_t uid);
 * char *gidtoname(gid_t gid);
 * bool findino(ino_t, dev_t);
 * void saveino(ino_t, dev_t);
 * #ifdef __linux__
 * char *strhash(char *str);
 * #endif
 *
 * /* html.c */
 * bool url_encode(FILE *fd, char *s);
 * void html_intro(void); ... void html_report(struct totals tot);
 * void html_encode(FILE *fd, char *s);
 *
 * /* info.c */
 * struct infofile *new_infofile(const char *path, bool checkparents);
 * void push_infostack(struct infofile *inf);
 * struct infofile *pop_infostack(void);
 * struct comment *infocheck(const char *path, const char *name, int top, bool isdir);
 * void printcomment(size_t line, size_t lines, char *s);
 *
 * /* json.c */
 * void json_indent(int maxlevel); void json_fillinfo(struct _info *ent);
 * void json_intro(void); ... void json_report(struct totals tot);
 *
 * /* list.c */
 * void null_intro(void); void null_outtro(void); void null_close(...);
 * void emit_tree(char **dirname, bool needfulltree);
 * struct totals listdir(char *dirname, struct _info **dir, int lev, dev_t dev, bool hasfulltree);
 *
 * /* tree.c */
 * void setoutput(const char *filename);
 * void print_version(int nl);
 * void usage(int);
 * void push_files(const char *dir, struct ignorefile **ig, struct infofile **inf, bool top);
 * int patignore(const char *name, bool isdir, bool checkpaths);
 * int patinclude(const char *name, bool isdir, bool checkpaths);
 * struct _info **unix_getfulltree(char *d, u_long lev, dev_t dev, off_t *size, char **err);
 * struct _info **read_dir(char *dir, ssize_t *n, int infotop);
 * int filesfirst(struct _info **, struct _info **); ... int fsizesort(...);
 * int patmatch(const char *buf, const char *pat, bool isdir);
 * void indent(int maxlevel);
 * void free_dir(struct _info **);
 * char *prot(mode_t);
 * char *do_date(time_t);
 * void printit(const char *);
 * int psize(char *buf, off_t size);
 * char Ftype(mode_t mode);
 * struct _info *stat2info(const struct stat *st);
 * char *fillinfo(char *buf, const struct _info *ent);
 *
 * /* unix.c */
 * int unix_printinfo(...); ... void unix_report(struct totals tot);
 *
 * /* util.c */
 * char *pathconcat(char *str, ...);
 * bool is_singleton(struct _info *dir);
 * void *xmalloc(size_t);
 * void *xrealloc(void *, size_t);
 *
 * /* xml.c */
 * void xml_intro(void); ... void xml_report(struct totals tot);
 *
 * /* strverscmp.c（仅非 Linux 平台在 C 中使用，Rust 版始终需要）*/
 * int strverscmp(const char *s1, const char *s2);
 * ===================================================================== */

