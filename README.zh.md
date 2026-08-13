# Rustree（rt, rust + tree）

[English](README.md) | 简体中文

**rustree** 是一个用于显示文件夹树状视图的小工具，使用 Rust 编写。它与原始的 tree 命令兼容，只是添加了多语言以及跨平台支持。原始仓库在 [此处][c-tree]。

当前支持系统：Windows，Linux。

## 安装

### 从源代码编译
rustree 是用 Rust 编写的，因此你需要安装 [Rust][rust-lang] 来编译它。

编译命令：
```bash
git clone https://github.com/TC999/rustree.git
cd rustree
cargo build --release
./target/release/rt --version
v0.1.0 ...
```

## 使用

你可以使用 `--help` 来显示详细的使用方法。

## 许可证

rustree 使用 GPL-v3 许可证。

## 致谢

- [tree 命令的所有开发者][c-tree]

[c-tree]: https://github.com/Old-Man-Programmer/tree

[rust-lang]: https://rust-lang.org/zh-CN/