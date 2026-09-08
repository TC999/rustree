use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::PathBuf;

pub struct I18n {
    messages: HashMap<String, String>,
    #[allow(dead_code)]
    lang: String,
}

// 获取 locales 文件夹路径。
// 查找顺序：
//   1. 所有平台：程序所在目录下的 locales/（由 build.rs 在编译时复制而来）
//   2. 非 Windows 系统：/usr/share/{包名}/locales（install.sh 安装的系统路径，作为兜底）
// 程序自带语言包优先，保证源码构建/升级后总是使用最新文案；
// 系统安装路径仅当程序旁无 locales 时（如旧安装布局）才作为 fallback。
fn get_locales_dir() -> PathBuf {
    let exe_path = env::current_exe().expect("无法获取可执行文件路径");
    let exe_dir = exe_path.parent().expect("无法获取可执行文件所在目录");
    let local = exe_dir.join("locales");
    if local.is_dir() {
        return local;
    }

    #[cfg(not(target_os = "windows"))]
    {
        // 包名来自 build.rs 导出的编译时常量（CARGO_PKG_NAME），不硬编码程序名
        let pkg_name = env!("LOCALES_PACKAGE_NAME");
        let sys_path = PathBuf::from(format!("/usr/share/{}/locales", pkg_name));
        if sys_path.is_dir() {
            return sys_path;
        }
    }

    local
}

// 语言文件格式（*.ftl）：
//   - 每行一条 `key = value`；`#` 开头为注释，空行忽略。
//   - `=` 后第一个空格是语法分隔符，其后所有内容（含前导空格）均为值的一部分。
//   - 值中 `{name}` 为变量占位符，运行时由 tr! 传入的参数替换。
//   - 转义序列：\n \t \r \b \f \\ 以及 \uXXXX（Unicode 码点）。
//     \b / \f / \r 分别对应 color::fancy 解释的粗体 / 斜体 / 结束颜色控制符。
fn parse(source: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for raw_line in source.lines() {
        if raw_line.is_empty() || raw_line.starts_with('#') {
            continue;
        }
        if let Some(eq) = raw_line.find('=') {
            let key = raw_line[..eq].trim_end();
            let rest = &raw_line[eq + 1..];
            // 剥离 `=` 后的一个分隔空格；余下内容（含缩进/前导空格）均为值
            let value = unescape(rest.strip_prefix(' ').unwrap_or(rest));
            map.insert(key.to_string(), value);
        }
    }
    map
}

// 还原转义序列为真实字符。
fn unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('r') => out.push('\r'),
            Some('b') => out.push('\u{8}'),
            Some('f') => out.push('\u{c}'),
            Some('\\') => out.push('\\'),
            Some('u') => {
                let hex: String = chars.by_ref().take(4).collect();
                match u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32) {
                    Some(ch) => out.push(ch),
                    None => {
                        out.push('\\');
                        out.push('u');
                        out.push_str(&hex);
                    }
                }
            }
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => out.push('\\'),
        }
    }
    out
}

// 将模板中的 {name} 占位符替换为对应参数值。
fn substitute(template: &str, names: &[&str], values: &[String]) -> String {
    let mut out = template.to_string();
    for (name, value) in names.iter().zip(values.iter()) {
        out = out.replace(&format!("{{{}}}", name), value);
    }
    out
}

impl I18n {
    // 按语言标签构造。文件查找顺序：
    //   1. {lang}.ftl        —— 精确匹配（如 zh-CN、en-US）
    //   2. {lang 主子标签}.ftl —— 如传入 zh → zh-CN，由 resolve_lang 补全
    //   3. en-US.ftl         —— 最终兜底（保证未知 locale / 缺失语言文件不崩溃）
    // 任何一步找到即返回；仅当连 en-US.ftl 都不存在（构建损坏）才 panic。
    pub fn new(lang: &str) -> Self {
        let locales_dir = get_locales_dir();
        let candidates = resolve_lang_files(lang);
        for cand in &candidates {
            let path = locales_dir.join(cand);
            if let Ok(source) = fs::read_to_string(&path) {
                return Self {
                    messages: parse(&source),
                    lang: lang.to_string(),
                };
            }
        }
        // 理论不可达：en-US.ftl 由 build.rs 随二进制携带
        panic!("语言文件 en-US.ftl 未找到（构建损坏）：{:?}", locales_dir);
    }

    pub fn text(&self, key: &str, names: &[&str], values: &[String]) -> String {
        match self.messages.get(key) {
            Some(template) => substitute(template, names, values),
            // key 未命中时回退为 key 本身，而非 panic：
            // 单条翻译漏项/拼写错误不应让整棵树崩溃（对齐原 C 版 gettext 缺失回退 msgid 语义）
            None => key.to_string(),
        }
    }
}

// 将任意语言标签解析为候选 .ftl 文件名列表（按优先级）。
//   "zh-CN"      → ["zh-CN.ftl", "en-US.ftl"]
//   "zh"         → ["zh-CN.ftl", "en-US.ftl"]     （补全区域后缀）
//   "en" / "C"   → ["en-US.ftl"]                 （C/POSIX locale 一律英文）
//   "" / 未知     → ["en-US.ftl"]                 （空或未知 locale 回退英文）
//   "fr-FR"      → ["fr-FR.ftl", "en-US.ftl"]     （未知语言也尝试精确名，失败回退英文）
fn resolve_lang_files(lang: &str) -> Vec<String> {
    let mut v: Vec<String> = Vec::new();
    let tag = lang.trim();
    if tag.is_empty() || tag.eq_ignore_ascii_case("c") || tag.eq_ignore_ascii_case("posix") {
        v.push("en-US.ftl".to_string());
        return v;
    }
    // 已含区域后缀（如 zh-CN）直接用
    if tag.contains('-') {
        v.push(format!("{}.ftl", tag));
    } else if tag.eq_ignore_ascii_case("en") {
        v.push("en-US.ftl".to_string());
    } else {
        // 纯语言码（如 zh）补全为 {lang}-CN 风格再兜底（仅对已知的 en/zh 精确补全；
        // 未知语言补全后多半也找不到文件，最终由 en-US.ftl 兜底）
        v.push(format!("{}-CN.ftl", tag));
    }
    // 兜底：任何未知/缺失都回退英文
    if !v.iter().any(|s| s == "en-US.ftl") {
        v.push("en-US.ftl".to_string());
    }
    v
}

// 全局 i18n 句柄：用 Mutex 持有 Option<I18n>，消除 static mut 的数据竞争 UB；
// 不再用 Box::leak 泄漏字符串（初始化可重复调用，旧值自然被替换释放）。
pub static BUNDLE: std::sync::Mutex<Option<I18n>> = std::sync::Mutex::new(None);
// 当前激活语言的主子标签（如 "en" / "zh"）。用 Mutex<String> 持有，
// 既避免 static mut 写入 UB，又允许测试中反复 init_with_lang 切换语言。
pub static ACTIVE_LANG: std::sync::Mutex<String> = std::sync::Mutex::new(String::new());

// 测试专用的串行锁：init_with_lang 会重新初始化全局状态，测试需串行以免竞态
#[cfg(test)]
pub static TEST_SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[macro_export]
macro_rules! tr {
    ($key:literal) => {
        $crate::i18n::tr($key, &[], Vec::new())
    };
    ($key:literal, $($arg:literal => $val:expr),+ $(,)?) => {
        $crate::i18n::tr(
            $key,
            &[$($arg),+],
            vec![$($val.to_string()),+]
        )
    };
}

// 初始化全局 i18n 句柄，按系统 locale 自动选择语言
pub fn init() {
    let lang = detect_lang();
    install(&lang);
}

// 以指定语言初始化（测试用）。传入 "en" → en-US.ftl；"zh-CN" → 精确文件名。
// 可重复调用：旧的全局句柄被替换，无内存泄漏。
#[cfg(test)]
pub fn init_with_lang(lang: &str) {
    // 与 detect_lang 一致的补全规则：纯语言码补 -US（en→en-US）
    let lang_file = if lang.contains('-') || lang.contains('.') {
        lang.to_string()
    } else if lang.eq_ignore_ascii_case("c") || lang.eq_ignore_ascii_case("posix") || lang.is_empty() {
        "en-US".to_string()
    } else {
        format!("{}-US", lang)
    };
    install(&lang_file);
}

// 写入全局句柄与主语言标签的公共实现
fn install(lang: &str) {
    let i = I18n::new(lang);
    // 主语言子标签：zh-CN → zh；en-US → en；供 main.rs 判断「是否英文」
    let primary = lang.split('-').next().unwrap_or("en").to_lowercase();
    *BUNDLE.lock().expect("i18n BUNDLE 锁中毒") = Some(i);
    *ACTIVE_LANG.lock().expect("i18n ACTIVE_LANG 锁中毒") = primary;
}

// 当前激活语言的主子标签（如 "en" / "zh"）。用于判断「是否英文」。
pub fn lang() -> String {
    ACTIVE_LANG
        .lock()
        .expect("i18n ACTIVE_LANG 锁中毒")
        .clone()
}

// 带命名参数的格式化调用（参数以 (name, value) 对传入）
pub fn tr(key: &str, param_names: &[&str], param_values: Vec<String>) -> String {
    let guard = BUNDLE.lock().expect("i18n BUNDLE 锁中毒");
    let bundle = guard.as_ref().expect("i18n 尚未初始化");
    bundle.text(key, param_names, &param_values)
}

// 自动检测系统语言。规范化输出完整 BCP47 标签（如 zh-CN / en-US）。
// 未知 / 空 / C / POSIX 一律回退 en-US，保证后续 I18n::new 必有文件可加载。
pub fn detect_lang() -> String {
    // 优先读取 LANG，其次 LC_ALL
    let raw = env::var("LANG")
        .ok()
        .filter(|v| !v.is_empty())
        .or_else(|| env::var("LC_ALL").ok().filter(|v| !v.is_empty()))
        .unwrap_or_else(|| "en-US".to_string());
    // 移除编码部分（如 zh_CN.UTF-8 → zh_CN）
    let tag = raw.split('.').next().unwrap_or("en-US");
    // 下划线转连字符（zh_CN → zh-CN）
    let tag = tag.replace('_', "-");
    // 规范化：空 / C / POSIX → en-US
    let tag = tag.trim();
    if tag.is_empty() || tag.eq_ignore_ascii_case("C") || tag.eq_ignore_ascii_case("POSIX") {
        return "en-US".to_string();
    }
    tag.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    // BUG 4 回归：text() 在 key 未命中时回退返回 key 本身，而非 panic。
    #[test]
    fn text_missing_key_falls_back_to_key() {
        let _g = TEST_SERIAL.lock().unwrap();
        init_with_lang("en");
        // 用一个不存在的 key，断言不 panic 且返回 key 名
        let s = tr("this-key-does-not-exist", &[], Vec::new());
        assert_eq!(s, "this-key-does-not-exist");
    }

    // BUG 1 回归：detect_lang 对空 / C / POSIX 规范化为 en-US；且 LANG 优先于 LC_ALL。
    #[test]
    fn detect_lang_normalizes_c_and_empty() {
        // LANG 优先；此处通过临时设置环境变量验证规范化逻辑
        // （detect_lang 是纯函数式读取环境，无全局状态，安全）
        let cases = [("C", "en-US"), ("POSIX", "en-US"), ("C.UTF-8", "en-US")];
        for (val, expect) in cases {
            env::set_var("LANG", val);
            env::remove_var("LC_ALL");
            assert_eq!(detect_lang(), expect, "LANG={} 时", val);
        }
        // LANG 优先于 LC_ALL：即使 LC_ALL 设中文，LANG 设 C 仍应取 LANG
        env::set_var("LANG", "C");
        env::set_var("LC_ALL", "zh_CN.UTF-8");
        assert_eq!(detect_lang(), "en-US", "LANG=C 应优先于 LC_ALL=zh_CN");
        // LANG 空时回退读 LC_ALL
        env::set_var("LANG", "");
        env::set_var("LC_ALL", "en_US.UTF-8");
        assert_eq!(detect_lang(), "en-US");
        env::remove_var("LC_ALL");
        env::remove_var("LANG");
        assert_eq!(detect_lang(), "en-US");
    }

    // resolve_lang_files 的兜底链路：未知语言最终都包含 en-US.ftl。
    #[test]
    fn resolve_lang_files_always_includes_en_us_fallback() {
        for input in ["", "C", "POSIX", "zh-CN", "zh", "en", "fr-FR", "de"] {
            let cands = resolve_lang_files(input);
            assert!(
                cands.iter().any(|c| c == "en-US.ftl"),
                "input={:?} 未包含 en-US.ftl 兜底: {:?}",
                input,
                cands
            );
        }
    }

    // BUG 1 回归：未知 locale 不再 panic，加载 en-US.ftl。
    #[test]
    fn new_unknown_lang_does_not_panic() {
        let _g = TEST_SERIAL.lock().unwrap();
        // fr-FR.ftl 不存在，应回退 en-US.ftl 加载，而非 panic
        let i = I18n::new("fr-FR");
        // en-US.ftl 里存在的 key 应可用
        assert_eq!(i.text("html-title", &[], &Vec::new()), "Directory Tree");
    }
}
