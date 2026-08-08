// 文件路径：src/i18n.rs
// 本地化（i18n）模块：使用 fluent（Mozilla 标准的纯 Rust 本地化框架）。
// 编译期嵌入 locales/en.ftl 与 locales/zh-CN.ftl；
// 启动时按系统 locale（LC_ALL > LC_MESSAGES > LANG）协商语言，默认英文。
// 说明：JSON/XML 输出的 type 值（"directory"/"file" 等）是数据格式值，保持英文不译。

use fluent_bundle::{FluentArgs, FluentBundle, FluentResource, FluentValue};
use fluent_langneg::{negotiate_languages, NegotiationStrategy};
#[cfg(test)]
use std::sync::Mutex;

const EN_FTL: &str = include_str!("../locales/en.ftl");
const ZH_FTL: &str = include_str!("../locales/zh-CN.ftl");

type Bundle = FluentBundle<FluentResource>;

// FluentBundle 含内部可变状态（RefCell/type_map，含 dyn Any），非 Send/Sync；
// 本程序为单线程，沿用项目全局 static mut + unsafe 访问模式（语义同 C 全局）
static mut BUNDLE: Option<Bundle> = None;
static mut LANG: Option<String> = None;

/// 串行锁：i18n 相关测试（含 unix/html 的 report 测试）共享，
/// 防止并行测试对全局 BUNDLE/LANG 的写竞争（static mut 非线程安全）。
/// 仅测试使用；main 运行路径单线程无需加锁。
#[cfg(test)]
pub static TEST_SERIAL: Mutex<()> = Mutex::new(());

/// 当前语言标识（"en" 或 "zh-CN"）
pub fn lang() -> &'static str {
    // unsafe：读取全局 LANG（单线程）
    unsafe { LANG.as_deref() }.unwrap_or("en")
}

/// 从系统环境变量检测语言并初始化 bundle。main() 开头调用一次。
pub fn init() {
    let lang_id = detect_language();
    init_with_lang(&lang_id);
}

/// 以显式语言初始化 bundle（"en" 或 "zh-CN"）。
/// 测试用：可绕过环境变量获得确定性语言。
pub fn init_with_lang(lang_id: &str) {
    let ftl = if lang_id.starts_with("zh") { ZH_FTL } else { EN_FTL };
    let resource = FluentResource::try_new(ftl.to_string()).expect("FTL 资源解析失败");
    let mut bundle = FluentBundle::new(vec![lang_id.parse().expect("语言 ID 合法")]);
    bundle
        .add_resource(resource)
        .expect("FTL 资源添加到 bundle 失败");
    // 关闭 Fluent 默认在变量两侧插入的隔离符（U+2068/2069），中文排版更自然
    bundle.set_use_isolating(false);
    // unsafe：写全局 LANG/BUNDLE（单线程）
    unsafe {
        LANG = Some(lang_id.to_string());
        BUNDLE = Some(bundle);
    }
}

/// 语言检测：按 LC_ALL > LC_MESSAGES > LANG 顺序，与 C 语言程序的惯例一致。
fn detect_language() -> String {
    let requested = std::env::var("LC_ALL")
        .or_else(|_| std::env::var("LC_MESSAGES"))
        .or_else(|_| std::env::var("LANG"))
        .unwrap_or_default();
    parse_requested(&requested)
}

/// 将单个 locale 名解析为可用语言（"en" / "zh-CN"）。
/// POSIX locale 名（zh_CN.UTF-8、zh-CN、zh）统一归一化。
fn parse_requested(requested: &str) -> String {
    // 下划线转连字符（zh_CN → zh-CN），去掉 .UTF-8 等编码后缀
    let normalized = requested.replace('_', "-");
    let normalized = normalized.split('.').next().unwrap_or(&normalized).to_string();
    let requested_langs: Vec<unic_langid::LanguageIdentifier> = vec![normalized.clone()]
        .into_iter()
        .filter_map(|s| s.parse().ok())
        .collect();
    let available_langs: Vec<unic_langid::LanguageIdentifier> = vec!["en", "zh-CN"]
        .into_iter()
        .filter_map(|s| s.parse().ok())
        .collect();
    let default: unic_langid::LanguageIdentifier = "en".parse().unwrap();
    // fluent-langneg 0.13：negotiate_languages 需显式策略参数
    let negotiated = negotiate_languages(
        &requested_langs,
        &available_langs,
        Some(&default),
        NegotiationStrategy::Filtering,
    );
    let best = negotiated
        .first()
        .map(|l| l.to_string())
        .unwrap_or_default();
    // 主语言回退：zh 系（含 zh-TW/zh-HK 等未与 zh-CN 精确匹配的区域）统一归入 zh-CN
    if best.starts_with("zh") || normalized.starts_with("zh") {
        "zh-CN".to_string()
    } else {
        "en".to_string()
    }
}

/// 参数值 trait：数字保持数字（Fluent 的复数选择依赖数字类型），
/// 字符串转 owned 字符串（避免借用临时值）。
pub trait ToFluentValue {
    fn to_fluent_value(self) -> FluentValue<'static>;
}

impl ToFluentValue for i32 {
    fn to_fluent_value(self) -> FluentValue<'static> {
        FluentValue::Number(self.into())
    }
}
impl ToFluentValue for i64 {
    fn to_fluent_value(self) -> FluentValue<'static> {
        FluentValue::Number(self.into())
    }
}
impl ToFluentValue for u64 {
    fn to_fluent_value(self) -> FluentValue<'static> {
        FluentValue::Number(self.into())
    }
}
impl ToFluentValue for usize {
    fn to_fluent_value(self) -> FluentValue<'static> {
        FluentValue::Number(self.into())
    }
}
impl ToFluentValue for &str {
    fn to_fluent_value(self) -> FluentValue<'static> {
        FluentValue::from(self.to_string())
    }
}
impl ToFluentValue for String {
    fn to_fluent_value(self) -> FluentValue<'static> {
        FluentValue::from(self)
    }
}
impl ToFluentValue for &String {
    fn to_fluent_value(self) -> FluentValue<'static> {
        FluentValue::from(self.clone())
    }
}
impl ToFluentValue for char {
    fn to_fluent_value(self) -> FluentValue<'static> {
        FluentValue::from(self.to_string())
    }
}

/// 按消息 ID 格式化并返回本地化文本。
pub fn tr(id: &str, args: &[(&str, FluentValue<'static>)]) -> String {
    // unsafe：读取全局 BUNDLE（单线程，main 中 init 后只读）
    let bundle = unsafe { BUNDLE.as_ref() }.expect("i18n 未初始化：main() 中未调用 i18n::init()");
    let message = bundle
        .get_message(id)
        .unwrap_or_else(|| panic!("FTL 消息缺失: {id}"));
    let pattern = message
        .value()
        .unwrap_or_else(|| panic!("FTL 消息无 value: {id}"));
    let mut fargs = FluentArgs::new();
    for (k, v) in args {
        fargs.set(*k, v.clone());
    }
    let mut errs = Vec::new();
    bundle.format_pattern(pattern, Some(&fargs), &mut errs).to_string()
}

/// 本地化宏：`tr!("message-id")` 或 `tr!("message-id", "arg" => value, ...)`。
/// 数字参数保持数字类型（复数规则生效），字符串参数自动转换。
#[macro_export]
macro_rules! tr {
    ($id:literal) => {
        $crate::i18n::tr($id, &[])
    };
    ($id:literal, $($k:literal => $v:expr),+ $(,)?) => {
        $crate::i18n::tr(
            $id,
            &[ $( ($k, $crate::i18n::ToFluentValue::to_fluent_value($v)) ),+ ],
        )
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_lang(lang: &str, f: impl FnOnce()) {
        // 测试串行执行：持锁期间独占全局 BUNDLE/LANG
        let _guard = crate::i18n::TEST_SERIAL.lock().unwrap();
        // 测试中直接重建 bundle（忽略环境变量，显式指定语言）
        let ftl = if lang.starts_with("zh") { ZH_FTL } else { EN_FTL };
        let resource = FluentResource::try_new(ftl.to_string()).unwrap();
        let mut bundle = FluentBundle::new(vec![lang.parse().unwrap()]);
        bundle.add_resource(resource).unwrap();
        bundle.set_use_isolating(false);
        // unsafe：测试中写全局（单线程）
        unsafe {
            LANG = Some(lang.to_string());
            BUNDLE = Some(bundle);
        }
        f();
    }

    #[test]
    fn test_english_error() {
        with_lang("en", || {
            assert_eq!(
                crate::tr!("invalid-option", "arg" => "xyz"),
                "rt: Invalid argument `xyz'."
            );
        });
    }

    #[test]
    fn test_chinese_error() {
        with_lang("zh-CN", || {
            assert_eq!(
                crate::tr!("invalid-option", "arg" => "xyz"),
                "rt: 无效参数 `xyz'。"
            );
        });
    }

    #[test]
    fn test_english_plural_report() {
        with_lang("en", || {
            // 1 目录 → director y；2 目录 → director ies
            let one = crate::tr!("report-full", "dirs" => 1u64, "files" => 1u64);
            assert!(one.contains("1 director"), "复数错误: {one}");
            assert!(one.contains("1 file"), "复数错误: {one}");
            let many = crate::tr!("report-full", "dirs" => 3u64, "files" => 2u64);
            assert!(many.contains("3 director"), "复数错误: {many}");
            assert!(many.contains("2 file"), "复数错误: {many}");
        });
    }

    #[test]
    fn test_chinese_report() {
        with_lang("zh-CN", || {
            // 中文无复数；du 变体含 size/unit
            let out = crate::tr!("report-full-du", "size" => " 46", "unit" => " 字节", "dirs" => 3u64, "files" => 2u64);
            assert_eq!(out, "共 3 个目录，2 个文件，占用  46 字节");
            let plain = crate::tr!("report-full", "dirs" => 3u64, "files" => 2u64);
            assert_eq!(plain, "共 3 个目录，2 个文件");
        });
    }

    #[test]
    fn test_detect_language() {
        // 纯函数测试（不触碰进程环境变量，避免并行测试串扰）
        assert_eq!(parse_requested("zh_CN.UTF-8"), "zh-CN");
        assert_eq!(parse_requested("zh-CN"), "zh-CN");
        assert_eq!(parse_requested("zh"), "zh-CN");
        assert_eq!(parse_requested("zh_TW"), "zh-CN");
        assert_eq!(parse_requested("en_US.UTF-8"), "en");
        assert_eq!(parse_requested("en"), "en");
        assert_eq!(parse_requested(""), "en");
        assert_eq!(parse_requested("fr_FR"), "en");
    }
}
