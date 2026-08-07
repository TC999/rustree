// 文件路径：src/main.rs
// 对应 C 源文件：tree.c
// 主程序入口。模块声明随翻译进度逐步添加。
//
// 翻译过程中临时允许 dead_code：尚未翻译完所有模块时，tree.rs 中已定义
// 但尚未被引用的类型/常量会触发该警告；全部模块翻译完成后移除本属性。
//
// 允许 static_mut_refs：本程序为单线程，所有对 static mut 全局变量的
// 访问均在 unsafe 块内并附中文注释，语义与 C 的全局变量一致；
// 直接对 static mut 取引用的方法调用（如 STATIC.take()）因此被允许。

#![allow(dead_code)]
#![allow(static_mut_refs)]

mod color;
mod filter;
mod globals;
mod hash;
mod info;
mod strverscmp;
mod tree;
mod util;

// =====================================================================
// 以下两函数提前翻译自 tree.c（patmatch/cond_lower），
// 因为 filter.c 的 filtercheck() 依赖 patmatch，必须先于 filter.rs 可用。
// 它们属于 tree.c，翻译 main.rs 时整体归档于此。
// =====================================================================

// === 原 C 函数：static char cond_lower(char c) ===
/// 若开启 --ignore-case 则转为小写，否则原样返回
fn cond_lower(c: u8) -> u8 {
    // unsafe：读取全局选项 FLAG
    if unsafe { globals::FLAG }.ignorecase {
        c.to_ascii_lowercase()
    } else {
        c
    }
}

// === 原 C 函数：int patmatch(const char *buf, const char *pat, bool isdir) ===
/// 通配符匹配。返回 1 匹配、0 不匹配、-1 模式语法错误。
/// 支持 '?'、'*'、'**'、'[...]'、'\\' 转义、'|' 或（递归子模式）、
/// 以及 '/' 与 isdir 的特殊交互。
pub fn patmatch(buf: &[u8], pat: &[u8], isdir: bool) -> i32 {
    let mut match_: i32 = 1;
    // C: char m, pprev = 0;（记录上一个模式字符，供 ** 跨 '/' 匹配判断）
    let mut pprev: u8 = 0;

    // C: bar = strchr(pat, '|') —— 若存在 '|'，递归比较两个子模式
    if let Some(bar) = pat.iter().position(|&b| b == b'|') {
        // C: if (bar == pat || !bar[1]) return -1;
        if bar == 0 || bar + 1 >= pat.len() {
            return -1;
        }
        // C: *bar = '\0'; match = patmatch(buf, pat, isdir);
        //     if (!match) match = patmatch(buf, bar+1, isdir);
        match_ = patmatch(buf, &pat[..bar], isdir);
        if match_ == 0 {
            match_ = patmatch(buf, &pat[bar + 1..], isdir);
        }
        return match_;
    }

    // buf/pat 的当前读取位置（C 中用指针推进）
    let mut bi: usize = 0;
    let mut pi: usize = 0;

    // C: while(*pat && match)
    while pi < pat.len() && match_ != 0 {
        match pat[pi] {
            b'[' => {
                pi += 1;
                // C: if(*pat != '^') { n = 1; match = 0; } else { pat++; n = 0; }
                let n: i32;
                if pi >= pat.len() || pat[pi] != b'^' {
                    n = 1;
                    match_ = 0;
                } else {
                    pi += 1;
                    n = 0;
                }
                // C: while(*pat != ']') { ... }
                loop {
                    // C: 循环内 if(!*pat) return -1
                    if pi >= pat.len() {
                        return -1;
                    }
                    if pat[pi] == b']' {
                        break;
                    }
                    // C: if(*pat == '\\') pat++;
                    if pat[pi] == b'\\' {
                        pi += 1;
                    }
                    // C: if(!*pat) return -1;
                    if pi >= pat.len() {
                        return -1;
                    }
                    // C: if(pat[1] == '-') —— 范围匹配
                    if pi + 1 < pat.len() && pat[pi + 1] == b'-' {
                        // C: m = *pat; pat += 2;
                        let m = pat[pi];
                        pi += 2;
                        // C: if(*pat == '\\' && *pat) pat++;
                        if pi < pat.len() && pat[pi] == b'\\' {
                            pi += 1;
                        }
                        // C: if(cond_lower(*buf) >= cond_lower(m) && cond_lower(*buf) <= cond_lower(*pat))
                        let bc = if bi < buf.len() { cond_lower(buf[bi]) } else { 0 };
                        let ec = if pi < pat.len() { cond_lower(pat[pi]) } else { 0 };
                        if bc >= cond_lower(m) && bc <= ec {
                            match_ = n;
                        }
                        // C: if(!*pat) pat--;（范围终点越界时回退，使外层循环最终 return -1）
                        if pi >= pat.len() {
                            pi -= 1;
                        }
                    } else if bi < buf.len() && cond_lower(buf[bi]) == cond_lower(pat[pi]) {
                        // C: else if(cond_lower(*buf) == cond_lower(*pat)) match = n;
                        match_ = n;
                    }
                    // C: pat++;
                    pi += 1;
                }
                // C: buf++;
                bi += 1;
            }
            b'*' => {
                pi += 1;
                // C: if(!*pat) { int f = (strchr(buf, '/') == NULL); return f; }
                if pi >= pat.len() {
                    let f = !buf[bi..].contains(&b'/');
                    return f as i32;
                }
                match_ = 0;
                // C: if (*pat == '*') —— "支持 **（与 * 基本等价，但可跨 / 匹配）"
                if pat[pi] == b'*' {
                    pi += 1;
                    // C: if(!*pat) return 1;
                    if pi >= pat.len() {
                        return 1;
                    }
                    // C: while(*buf && !(match = patmatch(buf, pat, isdir)))
                    loop {
                        if bi >= buf.len() {
                            break;
                        }
                        match_ = patmatch(&buf[bi..], &pat[pi..], isdir);
                        if match_ != 0 {
                            break;
                        }
                        // C: if (pprev == '/' && *pat == '/' && *(pat+1) &&
                        //        (match = patmatch(buf, pat+1, isdir))) return match;
                        if pprev == b'/' && pat[pi] == b'/' && pi + 1 < pat.len() {
                            let m = patmatch(&buf[bi..], &pat[pi + 1..], isdir);
                            if m != 0 {
                                return m;
                            }
                        }
                        // C: buf++; while(*buf && *buf != '/') buf++;
                        bi += 1;
                        while bi < buf.len() && buf[bi] != b'/' {
                            bi += 1;
                        }
                    }
                } else {
                    // C: while(*buf && !(match = patmatch(buf++, pat, isdir)))
                    //       if (*buf == '/') break;
                    loop {
                        if bi >= buf.len() {
                            break;
                        }
                        match_ = patmatch(&buf[bi..], &pat[pi..], isdir);
                        bi += 1;
                        if match_ != 0 {
                            break;
                        }
                        if bi < buf.len() && buf[bi] == b'/' {
                            break;
                        }
                    }
                }
                // C: if (!match && (!*buf || *buf == '/')) match = patmatch(buf, pat, isdir);
                if match_ == 0 && (bi >= buf.len() || buf[bi] == b'/') {
                    match_ = patmatch(&buf[bi..], &pat[pi..], isdir);
                }
                return match_;
            }
            b'?' => {
                // C: if(!*buf) return 0; buf++;
                if bi >= buf.len() {
                    return 0;
                }
                bi += 1;
            }
            b'/' => {
                // C: if (!*(pat+1) && !*buf) return isdir;
                if pi + 1 >= pat.len() && bi >= buf.len() {
                    return isdir as i32;
                }
                // C: match = (*buf++ == *pat);
                match_ = if bi < buf.len() && buf[bi] == b'/' { 1 } else { 0 };
                bi += 1;
            }
            b'\\' => {
                // C: if(*pat) pat++;（跳过反斜杠，落到 default 比较转义字符）
                if pi < pat.len() {
                    pi += 1;
                }
                // C: default: match = (cond_lower(*buf++) == cond_lower(*pat));
                let bc = if bi < buf.len() { cond_lower(buf[bi]) } else { 0 };
                let pc = if pi < pat.len() { cond_lower(pat[pi]) } else { 0 };
                match_ = if bc == pc { 1 } else { 0 };
                bi += 1;
            }
            _ => {
                // C: default: match = (cond_lower(*buf++) == cond_lower(*pat));
                let bc = if bi < buf.len() { cond_lower(buf[bi]) } else { 0 };
                let pc = cond_lower(pat[pi]);
                match_ = if bc == pc { 1 } else { 0 };
                bi += 1;
            }
        }
        // C: pprev = *pat++;（default 分支 pat 在循环末尾推进）
        if pi < pat.len() {
            pprev = pat[pi];
        }
        pi += 1;
        // C: if(match<1) return match;
        if match_ < 1 {
            return match_;
        }
    }

    // C: if(!*buf) return match; return 0;
    if bi >= buf.len() {
        match_
    } else {
        0
    }
}

fn main() {}
