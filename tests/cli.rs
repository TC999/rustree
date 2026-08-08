// 集成测试：验证编译出的 tree 二进制的基本行为。
// 对应翻译计划"步骤 3：编写 tests/ 集成测试对照原 tree 输出"。
// 不使用外部 crate，通过 std::process::Command 直接运行二进制
//（env!("CARGO_BIN_EXE_tree") 由 cargo 注入编译产物的路径）。

use std::path::Path;
use std::process::Command;

fn run_in(args: &[&str], cwd: &Path) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_tree"))
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("运行 tree 二进制失败");
    assert!(
        out.status.success(),
        "tree 退出码非 0：{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// 构造独立的临时目录树并返回路径（测试结束由调用者清理）。
fn make_fixture() -> std::path::PathBuf {
    let tmp = std::env::temp_dir().join(format!(
        "rustree_cli_test_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(tmp.join("sub/deep")).unwrap();
    std::fs::write(tmp.join("a.txt"), "x").unwrap();
    std::fs::write(tmp.join("sub/b.txt"), "y").unwrap();
    std::fs::write(tmp.join("sub/deep/c.txt"), "z").unwrap();
    tmp
}

#[test]
fn test_basic_output() {
    let tmp = make_fixture();
    let out = run_in(&[], &tmp);
    // 默认按名称排序：a.txt 在前，sub 在后
    assert!(out.contains("|-- a.txt"), "缺少 a.txt：\n{}", out);
    assert!(out.contains("`-- sub"), "缺少 sub：\n{}", out);
    assert!(out.contains("3 directories, 3 files"), "统计错误：\n{}", out);
    std::fs::remove_dir_all(&tmp).ok();
}

#[test]
fn test_depth_limit() {
    let tmp = make_fixture();
    // -L 1 只显示顶层
    let out = run_in(&["-L", "1"], &tmp);
    assert!(!out.contains("deep"), "-L 1 不应显示深层：\n{}", out);
    assert!(out.contains("2 directories, 1 file"), "统计错误：\n{}", out);
    std::fs::remove_dir_all(&tmp).ok();
}

#[test]
fn test_dir_only() {
    let tmp = make_fixture();
    let out = run_in(&["-d"], &tmp);
    assert!(!out.contains("a.txt"), "-d 不应显示文件：\n{}", out);
    assert!(out.contains("3 directories"), "目录统计错误：\n{}", out);
    std::fs::remove_dir_all(&tmp).ok();
}

#[test]
fn test_gitignore_filter() {
    let tmp = make_fixture();
    // 忽略 sub 目录
    std::fs::write(tmp.join(".gitignore"), "sub\n").unwrap();
    let out = run_in(&["--gitignore"], &tmp);
    assert!(!out.contains("sub"), "gitignore 应过滤 sub：\n{}", out);
    assert!(out.contains("a.txt"), "应保留 a.txt：\n{}", out);
    assert!(out.contains("1 directory, 1 file"), "统计错误：\n{}", out);
    std::fs::remove_dir_all(&tmp).ok();
}

#[test]
fn test_version_flag() {
    let out = run_in(&["--version"], Path::new("."));
    assert!(out.starts_with("tree v"), "版本输出错误：{}", out);
}

#[test]
fn test_xml_output() {
    let tmp = make_fixture();
    let out = run_in(&["-X"], &tmp);
    assert!(out.contains("<?xml version=\"1.0\""), "XML 头缺失：\n{}", out);
    assert!(out.contains("<tree>"), "XML 根缺失：\n{}", out);
    assert!(out.contains("<file name=\"a.txt\""), "XML 文件缺失：\n{}", out);
    assert!(out.contains("<report>"), "XML 报告缺失：\n{}", out);
    std::fs::remove_dir_all(&tmp).ok();
}

#[test]
fn test_json_output() {
    let tmp = make_fixture();
    let out = run_in(&["-J"], &tmp);
    assert!(out.contains("\"type\":\"file\",\"name\":\"a.txt\""), "JSON 文件缺失：\n{}", out);
    assert!(out.contains("\"type\":\"report\""), "JSON 报告缺失：\n{}", out);
    std::fs::remove_dir_all(&tmp).ok();
}

#[test]
fn test_output_to_file() {
    let tmp = make_fixture();
    let outfile = tmp.join("out.txt");
    // -o 输出到文件（验证 process::exit 前的显式 flush）
    let _ = run_in(&["-o", "out.txt"], &tmp);
    let content = std::fs::read_to_string(&outfile).expect("输出文件应存在");
    assert!(content.contains("a.txt"), "-o 输出内容错误：\n{}", content);
    // 输出文件本身也会被遍历（与 C 一致）：3 目录（. sub deep）+ 4 文件（a out b c）
    assert!(content.contains("3 directories, 4 files"), "-o 统计错误：\n{}", content);
    std::fs::remove_dir_all(&tmp).ok();
}

#[test]
fn test_invalid_option() {
    let out = Command::new(env!("CARGO_BIN_EXE_tree"))
        .arg("-Z")
        .output()
        .expect("运行 tree 失败");
    assert!(!out.status.success(), "-Z 应返回非 0 退出码");
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("Invalid argument"), "错误信息缺失：{}", err);
}

