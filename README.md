# Rustree（rt, rust + tree）

English | 简体中文

**rustree** is a handy little utility to display a tree view of folders that I just rewrite with Rust. Original version [here][c-tree]. 100% compatible with the original tree command, and I just add the i18n.

## Building

rustree is written by Rust, so you'll need to install [Rust][rust-lang] to compile it. 

To build:
```bash
git clone https://github.com/TC999/rustree.git
cd rustree
cargo build --release
./target/release/rt --version
v0.1.0 ...
```

Support OS: Windows，Linux

## Using

You can use `--help` to show the detail usage.

## LICENSE

Rustree is under GPL-v3.

[c-tree]: https://github.com/Old-Man-Programmer/tree

[rust-lang]: https://rust-lang.org

