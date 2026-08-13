use std::fs;
use std::env;
use std::path::PathBuf;
use fluent_bundle::{FluentBundle, FluentResource, FluentArgs};
use fluent_bundle::types::FluentNumber;
use unic_langid::LanguageIdentifier;

pub struct I18n {
    bundle: FluentBundle<FluentResource>,
    #[allow(dead_code)]
    lang: String,
}

// 获取 locales 文件夹路径。
// 查找顺序：
//   1. 非 Windows 系统：/usr/share/{包名}/locales（由 pkg-config / 系统安装路径）
//   2. 所有平台：程序所在目录下的 locales/（由 build.rs 在编译时复制而来）
fn get_locales_dir() -> PathBuf {
    #[cfg(not(target_os = "windows"))]
    {
        // 包名来自 build.rs 导出的编译时常量（CARGO_PKG_NAME），不硬编码程序名
        let pkg_name = env!("LOCALES_PACKAGE_NAME");
        let sys_path = PathBuf::from(format!("/usr/share/{}/locales", pkg_name));
        if sys_path.is_dir() {
            return sys_path;
        }
    }

    let exe_path = env::current_exe().expect("无法获取可执行文件路径");
    let exe_dir = exe_path.parent().expect("无法获取可执行文件所在目录");
    exe_dir.join("locales")
}

impl I18n {
    pub fn new(lang: &str) -> Self {
        let langid: LanguageIdentifier = lang.parse().unwrap();
        let path = get_locales_dir().join(format!("{}.ftl", lang));
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|_| panic!("语言文件未找到: {:?}", path));
        let res = FluentResource::try_new(source)
            .unwrap_or_else(|_| panic!("Fluent资源解析失败: {:?}", path));
        let mut bundle = FluentBundle::new(vec![langid]);
        bundle.add_resource(res).expect("添加Fluent资源失败");
        Self {
            bundle,
            lang: lang.to_string(),
        }
    }

    pub fn text(&self, key: &str, args: Option<&FluentArgs>) -> String {
        let msg = self.bundle.get_message(key).expect("未找到消息");
        let pattern = msg.value().expect("未找到内容");
        let s = self
            .bundle
            .format_pattern(pattern, args, &mut vec![]);
        // 剥离 Fluent 的隔离标记 U+2068 / U+2069
        s.replace('\u{2068}', "").replace('\u{2069}', "").to_string()
    }
}

// 单线程全局 i18n 句柄（与全局 OUTFILE 等 static mut 全局状态相同语义）
pub static mut BUNDLE: Option<I18n> = None;
pub static mut ACTIVE_LANG: &'static str = "en";

// 测试专用的串行锁：防止多线程下并发写入 BUNDLE/ACTIVE_LANG
#[cfg(test)]
pub static TEST_SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());

// 将 `to_string()` 后的值转为 FluentValue：纯数字用 Int，否则用 String
// （这样 FTL 的 Select Expression [one]/[other] 才能正确匹配复数）
pub fn to_fluent_value(v: &str) -> fluent_bundle::FluentValue<'_> {
    if let Ok(n) = v.parse::<i64>() {
        let num = FluentNumber::new(n as f64, fluent_bundle::types::FluentNumberOptions::default());
        fluent_bundle::FluentValue::Number(num)
    } else {
        fluent_bundle::FluentValue::from(v)
    }
}

#[macro_export]
macro_rules! tr {
    ($key:literal) => {
        $crate::i18n::tr($key, &[], std::vec::Vec::new())
    };
    ($key:literal, $($arg:literal => $val:expr),+ $(,)?) => {
        $crate::i18n::tr(
            $key,
            &[$($arg),+],
            vec![ $(
                $crate::i18n::to_fluent_value($val.to_string().as_str()),
            )+ ]
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
pub fn tr(
    key: &str,
    _param_names: &[&str],
    param_values: Vec<fluent_bundle::FluentValue<'_>>,
) -> String {
    unsafe {
        let bundle = BUNDLE.as_ref().expect("i18n 尚未初始化");
        if param_values.is_empty() {
            return bundle.text(key, None);
        }
        let mut args = FluentArgs::new();
        for (name, value) in _param_names.iter().zip(param_values.iter()) {
            args.set(name.to_string(), value.clone());
        }
        bundle.text(key, Some(&args))
    }
}

// 自动检测系统语言
pub fn detect_lang() -> String {
    // 优先读取 LC_ALL，其次 LANG，最后默认为 en-US
    env::var("LC_ALL")
        .or_else(|_| env::var("LANG"))
        .unwrap_or_else(|_| "en-US".to_string())
        .split('.')
        .next()  // 移除编码部分
        .unwrap_or("en-US")
        .replace('_', "-") // 变成 zh-CN 这种格式
}
