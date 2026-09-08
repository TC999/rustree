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
    pub fn new(lang: &str) -> Self {
        let path = get_locales_dir().join(format!("{}.ftl", lang));
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|_| panic!("语言文件未找到: {:?}", path));
        Self {
            messages: parse(&source),
            lang: lang.to_string(),
        }
    }

    pub fn text(&self, key: &str, names: &[&str], values: &[String]) -> String {
        let template = self
            .messages
            .get(key)
            .unwrap_or_else(|| panic!("未找到消息: {}", key));
        substitute(template, names, values)
    }
}

// 单线程全局 i18n 句柄（与全局 OUTFILE 等 static mut 全局状态相同语义）
pub static mut BUNDLE: Option<I18n> = None;
pub static mut ACTIVE_LANG: &str = "en";

// 测试专用的串行锁：防止多线程下并发写入 BUNDLE/ACTIVE_LANG
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
    unsafe {
        ACTIVE_LANG = std::boxed::Box::leak(lang.clone().into_boxed_str());
        BUNDLE = Some(I18n::new(ACTIVE_LANG));
    }
}

// 以指定语言初始化（测试用；自动补全为对应 *.ftl 文件名）
#[cfg(test)]
pub fn init_with_lang(lang: &str) {
    let lang_file = if lang.contains('.') || lang.contains('-') {
        lang.to_string()
    } else {
        // 测试传 "en" → 对应文件 en-US.ftl
        format!("{}-US", lang)
    };
    unsafe {
        ACTIVE_LANG = std::boxed::Box::leak(lang_file.clone().into_boxed_str());
        BUNDLE = Some(I18n::new(ACTIVE_LANG));
    }
}

// 当前激活语言的 bcp47 标签（如 "en" / "zh-CN"）
pub fn lang() -> &'static str {
    unsafe { ACTIVE_LANG }
}

// 带命名参数的格式化调用（参数以 (name, value) 对传入）
pub fn tr(key: &str, param_names: &[&str], param_values: Vec<String>) -> String {
    unsafe {
        let bundle = BUNDLE.as_ref().expect("i18n 尚未初始化");
        bundle.text(key, param_names, &param_values)
    }
}

// 自动检测系统语言
pub fn detect_lang() -> String {
    // 优先读取 LC_ALL，其次 LANG，最后默认为 en-US
    env::var("LC_ALL")
        .or_else(|_| env::var("LANG"))
        .unwrap_or_else(|_| "en-US".to_string())
        .split('.')
        .next() // 移除编码部分
        .unwrap_or("en-US")
        .replace('_', "-") // 变成 zh-CN 这种格式
}
