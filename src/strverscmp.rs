// 文件路径：src/strverscmp.rs
// 对应 C 源文件：strverscmp.c
// 版本字符串比较函数实现
// 此函数比较字符串时将数字字符视为数值进行比较

#[cfg(not(target_os = "linux"))]
pub fn strverscmp(s1: &str, s2: &str) -> i32 {
    let p1: &[u8] = s1.as_bytes();
    let p2: &[u8] = s2.as_bytes();

    let mut c1 = p1[0];
    let mut c2 = p2[0];

    // 状态常量定义
    const S_N: u8 = 0x0; // 正常状态
    const S_I: u8 = 0x4; // 比较整数部分
    const S_F: u8 = 0x8; // 比较小数部分
    const S_Z: u8 = 0xC; // 仅比较前导零

    // 结果类型常量
    const CMP: i8 = 2; // 返回差值
    const LEN: i8 = 3; // 使用长度差值比较

    // 状态转换表
    // 状态：x(非数字) d(数字) 0(零) -(其他)
    static next_state: [[u8; 4]; 4] = [
        // S_N
        [S_N, S_I, S_Z, S_N],
        // S_I
        [S_N, S_I, S_I, S_I],
        // S_F
        [S_N, S_F, S_F, S_F],
        // S_Z
        [S_N, S_F, S_Z, S_Z],
    ];

    // 结果类型表
    static result_type: [[i8; 16]; 4] = [
        // S_N
        [
            CMP, CMP, CMP, CMP, CMP, LEN, CMP, CMP,
            CMP, CMP, CMP, CMP, CMP, CMP, CMP, CMP,
        ],
        // S_I
        [
            CMP, -1, -1, CMP, 1, LEN, LEN, CMP,
            1, LEN, LEN, CMP, CMP, CMP, CMP, CMP,
        ],
        // S_F
        [
            CMP, CMP, CMP, CMP, CMP, LEN, CMP, CMP,
            CMP, CMP, CMP, CMP, CMP, CMP, CMP, CMP,
        ],
        // S_Z
        [
            CMP, 1, 1, CMP, -1, CMP, CMP, CMP,
            -1, CMP, CMP, CMP, CMP, CMP, CMP, CMP,
        ],
    ];

    if p1 == p2 {
        return 0;
    }

    // 初始状态：S_N | (c1=='0') + (isdigit(c1))
    let mut state = S_N as u8;
    let mut c1 = p1[0];
    let mut c2 = p2[0];

    // 初始状态：S_N | (c1=='0') + (isdigit(c1))
    let c1_is_zero = (c1 == b'0') as u8;
    let c1_is_digit = (c1 >= b'0' && c1 <= b'9') as u8;
    state |= c1_is_zero | c1_is_digit;

    let mut i = 1;
    let mut diff: i8 = 0;

    // 主循环：比较字符直到遇到差异或字符串结束
    while i < p1.len() && i < p2.len() && diff == 0 {
        c1 = p1[i];
        c2 = p2[i];
        i += 1;

        // 更新状态
        let c1_is_zero = (c1 == b'0') as u8;
        let c1_is_digit = (c1 >= b'0' && c1 <= b'9') as u8;
        state = next_state[state as usize][0];
        state |= c1_is_zero | c1_is_digit;

        // 计算差值
        diff = (c1 - c2) as i8;
    }

    // 如果到达字符串末尾且没有差异
    if diff == 0 && i >= p1.len() && i >= p2.len() {
        return 0;
    }

    // 处理最后一个字符的状态
    let c2_is_zero = (c2 == b'0') as u8;
    let c2_is_digit = (c2 >= b'0' && c2 <= b'9') as u8;
    let idx = (state as usize) << 2 | ((c2_is_zero + c2_is_digit) as usize);
    let state = result_type[state as usize][idx];

    match state {
        CMP => diff as i32,
        LEN => {
            // 比较数字部分的长度
            // 跳过 p1 中的数字
            while i < p1.len() && p1[i] >= b'0' && p1[i] <= b'9' {
                i += 1;
            }

            // 如果 p2 还有数字，返回 -1
            if i < p2.len() && p2[i] >= b'0' && p2[i] <= b'9' {
                return -1;
            }

            // 否则返回原来的差值
            diff as i32
        }
        _ => state as i32,
    }
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
