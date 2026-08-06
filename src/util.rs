// 文件路径：src/util.rs
// 对应 C 源文件：util.c
// 工具函数：路径拼接、单例目录判断、内存分配（对应 xmalloc/xrealloc）。

use crate::globals::FILE_PATHSEP;
use crate::tree::Info;

// 判断字节是否为路径分隔符（C: strchr(file_pathsep, c) != NULL）
fn is_pathsep(b: u8) -> bool {
    // unsafe：读取全局可变变量 FILE_PATHSEP
    unsafe { FILE_PATHSEP }.as_bytes().contains(&b)
}

// === 原 C 函数：char *pathnpcat(char *dst, char *src, char *start, char *end) ===
/// 将 src 追加到 dst 末尾，同时消除重复的路径分隔符。
/// C 的 start 参数用于判断 dst 是否已有内容（Rust 中以 dst 非空等价）；
/// end 参数是缓冲区上限（Rust 的 String 动态分配，天然无上限，故省略）。
/// 分隔符均为单字节 ASCII，因此按字符迭代并仅对 ASCII 字符做分隔符判断，
/// 可保持与 C 逐字节处理完全一致且不破坏 UTF-8。
fn pathnpcat(dst: &mut String, src: &str) {
    for ch in src.chars() {
        // 当前字符是否为路径分隔符
        let is_sep = ch.is_ascii() && is_pathsep(ch as u8);
        // dst 末尾字节是否为路径分隔符
        let prev_is_sep = dst
            .as_bytes()
            .last()
            .map(|&b| is_pathsep(b))
            .unwrap_or(false);
        // 不允许路径分隔符重复出现：前一个是分隔符且当前也是，则跳过
        if prev_is_sep && is_sep {
            continue;
        }
        dst.push(ch);
    }
}

// === 原 C 函数：char *pathconcat(char *str, ...) ===
/// 拼接任意数量的路径，试图消除重复的路径分隔符。
/// C 版为可变参数（以 NULL 结尾），Rust 版以切片 &[&str] 表达；
/// C 版返回静态缓冲区（调用方须 scopy 保存），Rust 版返回拥有的 String。
pub fn pathconcat(str_: &str, args: &[&str]) -> String {
    let mut buf = String::new();
    // C: if (str == buf) 的"追加模式"在 Rust 中无需保留——
    //     buf 为每次调用新建的局部 String，不存在复用静态缓冲的场景。
    pathnpcat(&mut buf, str_);

    // C: while ((s = va_arg(ap, char *)) != NULL)
    for s in args {
        pathnpcat(&mut buf, unsafe { FILE_PATHSEP });
        pathnpcat(&mut buf, s);
    }
    buf
}

// === 原 C 函数：bool is_singleton(struct _info *dir) ===
/// 若目录是单例目录（恰好有一个子项且该子项是目录）则返回 true。
pub fn is_singleton(dir: &Info) -> bool {
    match &dir.child {
        // C: if (dir->child == NULL) return false;
        None => false,
        Some(children) => {
            // C: if (dir->child[0] == NULL) return false;
            if children.is_empty() {
                return false;
            }
            // C: if (dir->child[1] != NULL) return false;
            if children.len() > 1 {
                return false;
            }
            // C: return dir->child[0]->isdir;
            children[0].isdir
        }
    }
}

// === 原 C 函数：void *xmalloc(size_t size) ===
/// 分配 size 字节的零初始化缓冲区。
/// C 中 malloc 失败时打印 "tree: virtual memory exhausted." 并 exit(1)；
/// Rust 中 Vec 分配失败时直接中止进程，行为等价（均为致命错误终止），
/// 因此无需显式错误检查。主要供翻译中模拟"显式字节缓冲区"的场景使用。
pub fn xmalloc(size: usize) -> Vec<u8> {
    vec![0u8; size]
}

// === 原 C 函数：void *xrealloc(void *ptr, size_t size) ===
/// 将缓冲区调整为 size 字节，保留原内容，新增部分零填充。
/// 语义与 C 的 realloc 相同；分配失败行为同 xmalloc 的说明。
pub fn xrealloc(ptr: Vec<u8>, size: usize) -> Vec<u8> {
    let mut v = ptr;
    v.resize(size, 0);
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pathconcat_simple() {
        assert_eq!(pathconcat("a", &["b", "c"]), "a/b/c");
    }

    #[test]
    fn test_pathconcat_no_dup_sep() {
        // 消除重复的分隔符
        assert_eq!(pathconcat("a/", &["/b"]), "a/b");
        assert_eq!(pathconcat("a//", &["///b"]), "a/b");
    }

    #[test]
    fn test_pathconcat_empty_args() {
        assert_eq!(pathconcat("a", &[]), "a");
    }

    #[test]
    fn test_pathconcat_root() {
        // 根目录特殊情形："/" 后接路径
        assert_eq!(pathconcat("/", &["etc"]), "/etc");
    }

    #[test]
    fn test_pathconcat_utf8() {
        // 非 ASCII 路径不应被破坏
        assert_eq!(pathconcat("目录", &["文件"]), "目录/文件");
    }

    #[test]
    fn test_is_singleton() {
        let mut dir = Info::default();
        assert!(!is_singleton(&dir)); // child == None

        dir.child = Some(vec![]);
        assert!(!is_singleton(&dir)); // child[0] == NULL

        dir.child = Some(vec![Info::default()]);
        assert!(!is_singleton(&dir)); // child[0] 不是目录

        dir.child = Some(vec![Info {
            isdir: true,
            ..Info::default()
        }]);
        assert!(is_singleton(&dir));

        dir.child = Some(vec![
            Info {
                isdir: true,
                ..Info::default()
            },
            Info::default(),
        ]);
        assert!(!is_singleton(&dir)); // child[1] != NULL
    }

    #[test]
    fn test_xmalloc_realloc() {
        let buf = xmalloc(10);
        assert_eq!(buf.len(), 10);
        assert!(buf.iter().all(|&b| b == 0));
        let mut buf = xrealloc(buf, 20);
        assert_eq!(buf.len(), 20);
        buf[5] = 1;
        let buf = xrealloc(buf, 4); // 缩小保留前 4 字节
        assert_eq!(buf.len(), 4);
    }
}
