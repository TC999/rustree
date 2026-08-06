// 文件路径：src/main.rs
// 对应 C 源文件：tree.c
// 主程序入口

use std::env;
use std::fs;
use std::io;
use std::path::Path;

mod tree;
mod strverscmp;

// 全局变量初始化
lazy_static::lazy_static! {
    pub static ref FLAGS: std::sync::Mutex<tree::Flags> = {
        let mut flags = tree::Flags::default();
        // 设置默认值
        flags.a = true;  // 显示隐藏文件
        flags.c = false; // 不使用颜色
        flags.d = false; // 显示所有目录
        flags.R = true;  // 递归
        std::sync::Mutex::new(flags)
    };
}

// 设置输出文件
pub fn setoutput(filename: &str) {
    // TODO: 实现输出文件设置
    println!("Setting output to: {}", filename);
}

// 打印版本信息
pub fn print_version(nl: bool) {
    println!("tree v0.1.0 (Rust implementation of GNU tree)");
    if nl {
        println!();
    }
}

// 显示使用信息
pub fn usage(exit_code: i32) {
    eprintln!("Usage: tree [OPTION]... [DIR]...");
    eprintln!();
    eprintln!("List contents of directories in a tree-like format.");
    eprintln!();
    eprintln!("Options:");
    eprintln!("  -a               List all files, including hidden files (starting with '.')");
    eprintln!("  -d               List directories only");
    eprintln!("  -L level         Descend only to specified level (max depth)");
    eprintln!("  -R               Recurse into subdirectories");
    eprintln!("  -q               Quote non-printable characters");
    eprintln!("  -Q               Quote names and escape non-printable characters");
    eprintln!("  -f               Force printing of full file names");
    eprintln!("  -i               Don't print indentation lines");
    eprintln!("  -p               Print the file type and permissions");
    eprintln!("  -s               Print the size in bytes");
    eprintln!("  -h               Print the size in human-readable format");
    eprintln!("  -u               Print the user name or UID number associated with each file");
    eprintln!("  -g               Print the group name or GID number associated with each file");
    eprintln!("  -D               Print the last modification time (instead of creation time)");
    eprintln!("  -F               Append indicator (e.g. '/', '*', '=') to entries");
    eprintln!("  --charset CHARSET Use specified charset");
    eprintln!("  --filelist FILE  Read directory list from FILE");
    eprintln!("  --fromfile       Read command line arguments from file");
    eprintln!("  --help           Display this help and exit");
    eprintln!("  --version        Output version information and exit");
    eprintln!();
    eprintln!("Report bugs to: https://github.com/rustree/rustree/issues");
    std::process::exit(exit_code);
}

// 处理命令行参数
fn parse_args(args: &[String]) -> Option<String> {
    let mut i = 1;
    let mut target_dir = ".".to_string();

    while i < args.len() {
        let arg = &args[i];

        match arg.as_str() {
            "-a" => {
                FLAGS.lock().unwrap().a = true;
            }
            "-d" => {
                FLAGS.lock().unwrap().d = true;
            }
            "-L" => {
                if i + 1 < args.len() {
                    i += 1;
                    // TODO: 解析深度级别
                }
            }
            "-R" => {
                FLAGS.lock().unwrap().R = true;
            }
            "-q" => {
                FLAGS.lock().unwrap().q = true;
            }
            "-Q" => {
                FLAGS.lock().unwrap().Q = true;
            }
            "-f" => {
                FLAGS.lock().unwrap().fflinks = true;
            }
            "-i" => {
                FLAGS.lock().unwrap().noindent = true;
            }
            "-p" => {
                FLAGS.lock().unwrap().p = true;
            }
            "-s" => {
                FLAGS.lock().unwrap().s = true;
            }
            "-h" => {
                FLAGS.lock().unwrap().h = true;
            }
            "-u" => {
                FLAGS.lock().unwrap().u = true;
            }
            "-g" => {
                FLAGS.lock().unwrap().g = true;
            }
            "-D" => {
                FLAGS.lock().unwrap().D = true;
            }
            "-F" => {
                FLAGS.lock().unwrap().F = true;
            }
            "--charset" => {
                if i + 1 < args.len() {
                    i += 1;
                    // TODO: 设置字符集
                }
            }
            "--filelist" => {
                if i + 1 < args.len() {
                    i += 1;
                    // TODO: 从文件读取目录列表
                }
            }
            "--fromfile" => {
                FLAGS.lock().unwrap().fromfile = true;
            }
            "--help" => {
                usage(0);
            }
            "--version" => {
                print_version(true);
                std::process::exit(0);
            }
            _ => {
                // 假设是目录路径
                target_dir = arg.clone();
                break;
            }
        }

        i += 1;
    }

    Some(target_dir)
}

// 读取目录并生成树形结构
fn read_directory(path: &str) -> io::Result<Vec<tree::Info>> {
    let path = Path::new(path);
    let entries = fs::read_dir(path)?;

    let mut infos = Vec::new();

    for entry in entries {
        let entry = entry?;
        let metadata = entry.metadata()?;

        let name = entry.file_name().into_string().unwrap_or_else(|_| "unknown".to_string());

        let mut info = tree::Info::from_metadata(
            name,
            &metadata,
            metadata.is_dir(),
            None,
        );

        // 设置文件类型标记
        let mode = metadata.permissions().as_u32() as u32;
        if mode & 0o040000 != 0 {
            info.mode = 0o40755; // 目录
            info.isdir = true;
        } else if mode & 0o120000 != 0 {
            info.mode = 0o120777; // 符号链接
            info.isdir = false;
        } else {
            info.mode = 0o100644; // 普通文件
            info.isfile = true;
        }

        // 处理隐藏文件
        if !FLAGS.lock().unwrap().a && name.starts_with('.') {
            continue;
        }

        infos.push(info);
    }

    Ok(infos)
}

// 递归读取目录树
fn read_tree_recursive(
    path: &str,
    level: i32,
    max_depth: i32,
) -> io::Result<Vec<tree::Info>> {
    if level > max_depth {
        return Ok(Vec::new());
    }

    let mut infos = read_directory(path)?;

    if level < max_depth && FLAGS.lock().unwrap().R {
        for info in infos.iter_mut() {
            if info.isdir {
                let dir_path = format!("{}/{}", path, info.name);
                let children = read_tree_recursive(&dir_path, level + 1, max_depth)?;

                if !children.is_empty() {
                    info.child = Some(children);
                }
            }
        }
    }

    Ok(infos)
}

// 打印树形结构
fn print_tree(infos: &[tree::Info], level: i32, prefix: &str) {
    for (i, info) in infos.iter().enumerate() {
        // 计算前缀
        let is_last = i == infos.len() - 1;
        let current_prefix = if level == 0 {
            String::new()
        } else if is_last {
            format!("{}└── ", prefix)
        } else {
            format!("{}├── ", prefix)
        };

        // 打印文件名
        let name = if FLAGS.lock().unwrap().Q {
            format!("{:?}", info.name)
        } else {
            info.name.clone()
        };

        println!("{}{}", current_prefix, name);

        // 递归打印子目录
        if info.isdir {
            let child_prefix = if level == 0 {
                String::new()
            } else if is_last {
                format!("{}    ", prefix)
            } else {
                format!("{}│   ", prefix)
            };

            if let Some(children) = &info.child {
                print_tree(children, level + 1, &child_prefix);
            }
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() <= 1 {
        usage(1);
    }

    // 解析参数
    let target_dir = match parse_args(&args) {
        Some(dir) => dir,
        None => {
            usage(1);
            return;
        }
    };

    // 读取目录树
    let max_depth = 10; // 默认最大深度
    let infos = match read_tree_recursive(&target_dir, 0, max_depth) {
        Ok(infos) => infos,
        Err(e) => {
            eprintln!("tree: Error reading directory {}: {}", target_dir, e);
            std::process::exit(1);
        }
    };

    if infos.is_empty() {
        println!("tree: {} is empty", target_dir);
        return;
    }

    // 打印树形结构
    print_tree(&infos, 0, "");
}

// 添加必要的依赖
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_args() {
        let args = vec![
            "tree".to_string(),
            "-a".to_string(),
            "-L".to_string(),
            "2".to_string(),
            ".".to_string(),
        ];

        let target_dir = parse_args(&args).unwrap();
        assert_eq!(target_dir, ".");
    }

    #[test]
    fn test_usage() {
        // 测试 usage 函数是否正常调用
        let _ = std::panic::catch_unwind(|| usage(0));
    }

    #[test]
    fn test_print_version() {
        print_version(false);
    }
}
