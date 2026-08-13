// build.rs
// 在编译时将项目根目录下的 locales/ 文件夹及其所有内容复制到二进制输出目录。
//
// 原因：i18n 模块（src/i18n.rs）按"程序所在目录 + locales"约定查找 FTL 语言文件。
// 若用户把 rt 二进制移到任意位置（例如安装到 /usr/local/bin），语言文件
// 必须随二进制一并携带；在打包/分发/CI 场景中，仅靠源仓库的相对路径不可靠。
// 因此 build.rs 负责把 locales 拷贝进 cargo 的输出目录，使构建产物自包含。
//
// 复制目标：
//   1. OUT_DIR —— cargo 传给构建脚本的工作目录，可被其他 build 阶段消费（稳定）
//   2. TARGET_BIN_DIR（CARGO_MANIFEST_DIR/../target/{profile}/）—— 二进制所在目录，
//      使"程序所在目录/locales"这一运行时约定成立；测试二进制在 deps/ 下，
//      也会同步一份，确保 cargo test 无需回退逻辑即可找到 locales。
//
// 使用标准库实现，无需任何 build 依赖，跨平台可移植。

use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let src = manifest_dir.join("locales");

    if !src.is_dir() {
        // 无 locales 目录时静默跳过（例如用户自定义配置剥离了语言文件）
        return;
    }

    // 目标 1：OUT_DIR（build.rs 的工作目录）
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let dest_out = out_dir.join("locales");

    // 目标 2：二进制所在目录（target/{profile}/），通过 CARGO_TARGET_DIR + PROFILE 拼接
    let target_dir = PathBuf::from(
        env::var("CARGO_TARGET_DIR").unwrap_or_else(|_| manifest_dir.join("target").to_string_lossy().to_string())
    );
    let profile = env::var("PROFILE").unwrap_or_else(|_| "debug".to_string());
    let target_bin_dir = target_dir.join(&profile);

    copy_dir_all(&src, &dest_out).expect("复制 locales 到 OUT_DIR 失败");

    if !target_bin_dir.as_os_str().is_empty() && target_bin_dir.is_dir() {
        let dest_bin = target_bin_dir.join("locales");
        copy_dir_all(&src, &dest_bin).expect("复制 locales 到目标二进制目录失败");
        // 测试二进制位于 target/{profile}/deps/，也需一份 locales
        let deps_dir = target_bin_dir.join("deps");
        fs::create_dir_all(&deps_dir).ok();
        copy_dir_all(&src, &deps_dir.join("locales")).ok();
    }

    // 重新构建触发条件：只要 locales 目录下任意文件内容变化，就重新执行本脚本
    for entry in fs::read_dir(&src).expect("读取 locales 目录失败") {
        let entry = entry.expect("读取 locales 条目失败");
        let path = entry.path();
        if path.is_file() {
            println!("cargo:rerun-if-changed={}", path.display());
        }
    }
    // 目录级检测：新增/删除 .ftl 时也能触发重建
    println!("cargo:rerun-if-changed=locales");
}

// 递归拷贝目录：源 -> 目标。若目标已存在则覆盖同名文件/子目录。
fn copy_dir_all(src: &std::path::Path, dst: &std::path::Path) -> std::io::Result<()> {
    if !dst.exists() {
        fs::create_dir_all(dst)?;
    }
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_all(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}
