//! Localization — the same `Localizable.strings` tables the macOS bundle
//! ships, parsed at runtime. Language selection follows
//! `AppearanceSettings.language`; `system` resolves through `LANGUAGE`/`LANG`.

use crate::settings::AppLanguage;
use std::collections::HashMap;
use std::sync::OnceLock;

const EN: &str = include_str!("../../../../Mac/Resources/en.lproj/Localizable.strings");
const JA: &str = include_str!("../../../../Mac/Resources/ja.lproj/Localizable.strings");
const KO: &str = include_str!("../../../../Mac/Resources/ko.lproj/Localizable.strings");
const VI: &str = include_str!("../../../../Mac/Resources/vi.lproj/Localizable.strings");
const ES: &str = include_str!("../../../../Mac/Resources/es.lproj/Localizable.strings");
const RU: &str = include_str!("../../../../Mac/Resources/ru.lproj/Localizable.strings");
const ZH_HANS: &str = include_str!("../../../../Mac/Resources/zh-Hans.lproj/Localizable.strings");

static TABLES: OnceLock<HashMap<&'static str, HashMap<String, String>>> = OnceLock::new();

fn tables() -> &'static HashMap<&'static str, HashMap<String, String>> {
    TABLES.get_or_init(|| {
        let mut m = HashMap::new();
        m.insert("en", parse_strings(EN));
        m.insert("ja", parse_strings(JA));
        m.insert("ko", parse_strings(KO));
        m.insert("vi", parse_strings(VI));
        m.insert("es", parse_strings(ES));
        m.insert("ru", parse_strings(RU));
        m.insert("zh-Hans", parse_strings(ZH_HANS));
        m
    })
}

/// `.strings` parser: `"key" = "value";` lines, // comments, \-escapes.
fn parse_strings(raw: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("//") || !line.starts_with('"') {
            continue;
        }
        let Some(key_end) = line[1..].find('"') else { continue };
        let key = &line[1..1 + key_end];
        let rest = line[1 + key_end + 1..].trim_start();
        let Some(rest) = rest.strip_prefix('=') else { continue };
        let rest = rest.trim_start();
        if !rest.starts_with('"') {
            continue;
        }
        // Parse the value honoring backslash escapes.
        let mut value = String::new();
        let mut chars = rest[1..].chars().peekable();
        while let Some(c) = chars.next() {
            match c {
                '\\' => {
                    if let Some(next) = chars.next() {
                        match next {
                            'n' => value.push('\n'),
                            't' => value.push('\t'),
                            '"' => value.push('"'),
                            '\\' => value.push('\\'),
                            // `\uXXXX` / `\UXXXX` — the old-style plist
                            // Unicode escape Apple's strings parser honors.
                            'u' | 'U' => {
                                let mut code = 0u32;
                                let mut digits = 0;
                                while digits < 4 {
                                    match chars.peek().copied().and_then(|h| h.to_digit(16)) {
                                        Some(d) => {
                                            code = code * 16 + d;
                                            chars.next();
                                            digits += 1;
                                        }
                                        None => break,
                                    }
                                }
                                match (digits == 4).then(|| char::from_u32(code)).flatten() {
                                    Some(escaped) => value.push(escaped),
                                    None => {
                                        value.push(next);
                                        value.push_str(&format!("{code:0width$x}", width = digits));
                                    }
                                }
                            }
                            other => value.push(other),
                        }
                    }
                }
                '"' => break,
                other => value.push(other),
            }
        }
        map.insert(key.to_string(), value);
    }
    map
}

/// Resolved language for the session (AppearanceSettings or system locale).
pub fn resolve_language(setting: AppLanguage) -> &'static str {
    match setting {
        AppLanguage::En => "en",
        AppLanguage::Ko => "ko",
        AppLanguage::Ja => "ja",
        AppLanguage::Vi => "vi",
        AppLanguage::Ru => "ru",
        AppLanguage::ZhHans => "zh-Hans",
        AppLanguage::Es => "es",
        AppLanguage::System => {
            let raw = std::env::var("LANGUAGE")
                .or_else(|_| std::env::var("LC_ALL"))
                .or_else(|_| std::env::var("LANG"))
                .unwrap_or_default();
            let code = raw
                .split(['.', ':', '_'])
                .next()
                .unwrap_or("en")
                .to_lowercase();
            match code.as_str() {
                "ko" => "ko",
                "ja" => "ja",
                "vi" => "vi",
                "ru" => "ru",
                "es" => "es",
                "zh" => "zh-Hans",
                _ => "en",
            }
        }
    }
}

/// `String(localized:)` — look up `key` in `language` with English fallback,
/// and `%@`/`%u` printf-style substitution like the Swift strings.
pub fn tr(language: &str, key: &str) -> String {
    let tables = tables();
    tables
        .get(language)
        .and_then(|t| t.get(key))
        .or_else(|| tables.get("en").and_then(|t| t.get(key)))
        .cloned()
        .unwrap_or_else(|| key.to_string())
}
/// `tr` with a single substitution.
pub fn tr1(language: &str, key: &str, arg: &str) -> String {
    tr(language, key)
        .replacen("%@", arg, 1)
        .replacen("%u", arg, 1)
}

/// `String(format:localized:, args)` — substitution like the macOS
/// format strings, in one pass so substituted text is never re-scanned:
/// `%N$@`/`%N$ld`/`%N$d`/`%N$u` take `args[N-1]`, bare `%@`/`%ld`/`%d`/`%u`
/// take the next sequential argument, `%%` is a literal percent.
pub fn trn(language: &str, key: &str, args: &[&str]) -> String {
    substitute_args(&tr(language, key), args)
}

fn substitute_args(template: &str, args: &[&str]) -> String {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    let mut next = 0usize;
    'scan: while let Some(pos) = rest.find('%') {
        out.push_str(&rest[..pos]);
        let spec = &rest[pos + 1..];
        if spec.starts_with('%') {
            out.push('%');
            rest = &spec[1..];
            continue;
        }
        // `%N$kind` — the digits before `$` pick args[N-1].
        if let Some(dollar) = spec
            .find('$')
            .filter(|d| !spec[..*d].is_empty() && spec[..*d].chars().all(|c| c.is_ascii_digit()))
        {
            if let Ok(n) = spec[..dollar].parse::<usize>() {
                if n >= 1 && n <= args.len() {
                    for kind in ["@", "ld", "d", "u"] {
                        if spec[dollar + 1..].starts_with(kind) {
                            out.push_str(args[n - 1]);
                            rest = &spec[dollar + 1 + kind.len()..];
                            continue 'scan;
                        }
                    }
                }
            }
        }
        for kind in ["@", "ld", "d", "u"] {
            if spec.starts_with(kind) {
                if next < args.len() {
                    out.push_str(args[next]);
                    next += 1;
                }
                rest = &spec[kind.len()..];
                continue 'scan;
            }
        }
        // Not a specifier — a literal percent.
        out.push('%');
        rest = spec;
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::{parse_strings, substitute_args};

    #[test]
    fn parses_unicode_escapes() {
        let m = parse_strings("\"k\" = \"a\\u0041\\U00e9b\";");
        assert_eq!(m.get("k").map(String::as_str), Some("aA\u{e9}b"));
    }

    #[test]
    fn malformed_unicode_escape_stays_literal() {
        let m = parse_strings("\"k\" = \"x\\u12g\";");
        assert_eq!(m.get("k").map(String::as_str), Some("xu12g"));
        // Lone surrogate — not a valid scalar, preserved verbatim.
        let m = parse_strings("\"k\" = \"\\ud800\";");
        assert_eq!(m.get("k").map(String::as_str), Some("ud800"));
    }

    #[test]
    fn parses_standard_escapes() {
        let m = parse_strings("\"k\" = \"a\\nb\\t\\\"q\\\"\\\\\";");
        assert_eq!(m.get("k").map(String::as_str), Some("a\nb\t\"q\"\\"));
    }

    #[test]
    fn substitute_never_rescans_arguments() {
        // An argument that itself looks like a specifier stays text.
        assert_eq!(
            substitute_args("a %@ b", &["%2$@ inside %@"]),
            "a %2$@ inside %@ b"
        );
        assert_eq!(
            substitute_args("%1$@ / %2$@", &["%@", "tail %1$ld"]),
            "%@ / tail %1$ld"
        );
    }

    #[test]
    fn substitute_indexed_and_sequential_and_percent() {
        assert_eq!(
            substitute_args("%2$@ then %@ of %1$ld", &["7", "dev"]),
            "dev then 7 of 7"
        );
        assert_eq!(substitute_args("%d+%u=%ld", &["1", "2", "3"]), "1+2=3");
        assert_eq!(substitute_args("100%% done %@", &["x"]), "100% done x");
        // Unknown or out-of-range specifiers stay literal.
        assert_eq!(substitute_args("%9$@ %x %", &["a"]), "%9$@ %x %");
    }
}
