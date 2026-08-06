// 文件路径：src/strverscmp.rs
// 对应 C 源文件：strverscmp.c
//
// 将字符串按"版本号"比较：字符串中的数字序列按数值而非字典序比较。
// 原 C 代码来自 glibc/libiberty（Jean-François Bignolles 贡献，1997）。
// 在 C 中该函数仅非 Linux 平台使用（Linux 由 glibc 提供）；
// Rust 没有系统级 strverscmp 可用，因此始终编译本实现。

// 状态常量（C: #define S_N 0x0 / S_I 0x4 / S_F 0x8 / S_Z 0xC）
// S_N: 正常状态；S_I: 比较整数部分；S_F: 比较小数部分；S_Z: 仅含前导零
const S_N: i32 = 0x0;
const S_I: i32 = 0x4;
const S_F: i32 = 0x8;
const S_Z: i32 = 0xC;

// 结果类型（C: #define CMP 2 / LEN 3）
// CMP: 返回字符差值；LEN: 按数字序列长度比较
const CMP: i32 = 2;
const LEN: i32 = 3;

// C 的 isdigit(c)
fn is_digit(c: u8) -> bool {
    c >= b'0' && c <= b'9'
}

// 状态转换表（与 C 源码逐项对应）。
// 一维布局：S_N/S_I/S_F/S_Z 的行值 (0/4/8/12) 恰为各行的起始索引，
// 因此 state（含符号位的值）可直接作为表索引。
// 列顺序：x(其他) d(数字) 0(零) -(padding)
static NEXT_STATE: [i32; 16] = [
    // S_N 行
    S_N, S_I, S_Z, S_N,
    // S_I 行
    S_N, S_I, S_I, S_I,
    // S_F 行
    S_N, S_F, S_F, S_F,
    // S_Z 行
    S_N, S_F, S_Z, S_Z,
];

// 结果类型表。每行 16 项，按 (c1符号×4 + c2符号) 排列：
// x/x x/d x/0 x/-  d/x d/d d/0 d/-  0/x 0/d 0/0 0/-  -/x -/d -/0 -/-
static RESULT_TYPE: [i32; 60] = [
    // S_N 行
    CMP, CMP, CMP, CMP, CMP, LEN, CMP, CMP, CMP, CMP, CMP, CMP, CMP, CMP, CMP, CMP,
    // S_I 行
    CMP, -1, -1, CMP, 1, LEN, LEN, CMP, 1, LEN, LEN, CMP, CMP, CMP, CMP, CMP,
    // S_F 行
    CMP, CMP, CMP, CMP, CMP, LEN, CMP, CMP, CMP, CMP, CMP, CMP, CMP, CMP, CMP, CMP,
    // S_Z 行（仅 12 项）
    CMP, 1, 1, CMP, -1, CMP, CMP, CMP, -1, CMP, CMP, CMP,
];

// === 原 C 函数：int strverscmp (const char *s1, const char *s2) ===
/// 比较两个版本号字符串，返回 <0 / ==0 / >0。
/// 实现与 C 源码逐行对应；C 的 `'\0'` 终止符在 Rust 中以索引越界返回 0 模拟。
pub fn strverscmp(s1: &str, s2: &str) -> i32 {
    // C: const unsigned char *p1 = (const unsigned char *) s1;（按字节处理）
    let p1 = s1.as_bytes();
    let p2 = s2.as_bytes();

    // C: if (p1 == p2) return 0; —— 两个参数是同一个字符串对象
    if std::ptr::eq(s1.as_ptr(), s2.as_ptr()) {
        return 0;
    }

    // C: c1 = *p1++; c2 = *p2++;（p1/p2 前进到第二个字符）
    let mut i1: usize = 1;
    let mut i2: usize = 1;
    let mut c1: u8 = *p1.first().unwrap_or(&0);
    let mut c2: u8 = *p2.first().unwrap_or(&0);

    // C: state = S_N | ((c1 == '0') + (isdigit (c1) != 0));
    let mut state = S_N | ((c1 == b'0') as i32) + (is_digit(c1) as i32);

    // C: while ((diff = c1 - c2) == 0 && c1 != '\0')
    let mut diff = c1 as i32 - c2 as i32;
    while diff == 0 && c1 != 0 {
        // C: state = next_state[state];
        state = NEXT_STATE[state as usize];
        // C: c1 = *p1++; c2 = *p2++;（越界即字符串结束，视作 '\0'）
        c1 = if i1 < p1.len() { p1[i1] } else { 0 };
        c2 = if i2 < p2.len() { p2[i2] } else { 0 };
        i1 += 1;
        i2 += 1;
        // C: state |= (c1 == '0') + (isdigit (c1) != 0);
        state |= ((c1 == b'0') as i32) + (is_digit(c1) as i32);
        diff = c1 as i32 - c2 as i32;
    }

    // C: state = result_type[state << 2 | (((c2 == '0') + (isdigit (c2) != 0)))];
    let sym2 = ((c2 == b'0') as i32) + (is_digit(c2) as i32);
    state = RESULT_TYPE[((state << 2) | sym2) as usize];

    match state {
        CMP => diff,
        LEN => {
            // C: while (isdigit (*p1++)) if (!isdigit (*p2++)) return 1;
            loop {
                if i1 >= p1.len() || !is_digit(p1[i1]) {
                    break;
                }
                i1 += 1;
                if i2 >= p2.len() || !is_digit(p2[i2]) {
                    return 1;
                }
                i2 += 1;
            }
            // C: return isdigit (*p2) ? -1 : diff;
            if i2 < p2.len() && is_digit(p2[i2]) {
                -1
            } else {
                diff
            }
        }
        _ => state,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // glibc 文档中的示例（tree.h 中 strverscmp 的注释也引用了它们）
    #[test]
    fn test_no_digit() {
        // 同 strcmp 行为
        assert_eq!(strverscmp("no digit", "no digit"), 0);
    }

    #[test]
    fn test_item_99_vs_100() {
        // 相同前缀，但 99 < 100
        assert!(strverscmp("item#99", "item#100") < 0);
    }

    #[test]
    fn test_alpha1_vs_alpha001() {
        // 小数部分劣于整数部分
        assert!(strverscmp("alpha1", "alpha001") > 0);
    }

    #[test]
    fn test_part1_f012_vs_part1_f01() {
        // 两个小数部分
        assert!(strverscmp("part1_f012", "part1_f01") > 0);
    }

    #[test]
    fn test_foo_009_vs_foo_0() {
        // 同上，但仅含前导零
        assert!(strverscmp("foo.009", "foo.0") < 0);
    }

    #[test]
    fn test_equal() {
        assert_eq!(strverscmp("file1", "file1"), 0);
        assert_eq!(strverscmp("", ""), 0);
    }

    #[test]
    fn test_less() {
        assert!(strverscmp("file1", "file2") < 0);
        assert!(strverscmp("", "a") < 0);
        assert!(strverscmp("a", "a0") < 0);
    }

    #[test]
    fn test_greater() {
        assert!(strverscmp("file2", "file1") > 0);
        assert!(strverscmp("a", "") > 0);
    }

    #[test]
    fn test_version_like() {
        // 常见版本号排序：1.9 < 1.10
        assert!(strverscmp("1.9", "1.10") < 0);
        assert!(strverscmp("2.0.1", "2.0.10") < 0);
        assert!(strverscmp("v1.2.3", "v1.2.3") == 0);
    }
}
