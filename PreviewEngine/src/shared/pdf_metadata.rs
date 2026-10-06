// Pitex-authored PDF metadata/date policy. SPDX-License-Identifier: AGPL-3.0-or-later
// PDF date syntax and parameter behavior follow the public PDF/pdfTeX interfaces.
// SOURCE_DATE_EPOCH controls generated timestamps; explicitly provided dates are
// retained when automatic dates are suppressed, as in stock pdfTeX.
use std::{
    collections::BTreeMap,
    sync::OnceLock,
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Copy, Debug, Default)]
pub struct MetadataPolicy {
    pub omit_info_dictionary: bool,
    pub omit_automatic_dates: bool,
    pub suppress_ptex: i32,
    pub use_ptex_underscore: i32,
    pub pdf_major_version: i32,
}
impl MetadataPolicy {
    pub fn ptex_key(&self, name: &str) -> Option<String> {
        let bit = match name {
            "Fullbanner" => 1,
            "FileName" => 2,
            "PageNumber" => 4,
            "InfoDict" => 8,
            _ => return None,
        };
        if self.suppress_ptex & bit != 0 {
            return None;
        }
        Some(format!(
            "PTEX{}{name}",
            if self.use_ptex_underscore > 0 || self.pdf_major_version >= 2 {
                "_"
            } else {
                "."
            }
        ))
    }
    pub fn automatic_info(&self, info: &mut BTreeMap<String, String>, date: &str, banner: &str) {
        if self.omit_info_dictionary {
            return;
        }
        if !self.omit_automatic_dates {
            info.entry("CreationDate".into())
                .or_insert_with(|| pdf_literal(date));
            info.entry("ModDate".into())
                .or_insert_with(|| pdf_literal(date));
        }
        if let Some(key) = self.ptex_key("Fullbanner") {
            info.entry(key).or_insert_with(|| pdf_literal(banner));
        }
    }
}
fn pdf_literal(value: &str) -> String {
    format!(
        "({})",
        value
            .replace('\\', "\\\\")
            .replace('(', "\\(")
            .replace(')', "\\)")
    )
}
pub fn source_date_epoch(value: Option<&str>) -> Option<i64> {
    value
        .and_then(|value| value.parse::<i64>().ok())
        .filter(|value| *value >= 0)
}
pub fn date_at(epoch: i64, utc: bool) -> Option<String> {
    unsafe {
        let timestamp = epoch as libc::time_t;
        let mut broken = std::mem::MaybeUninit::<libc::tm>::zeroed();
        let result = if utc {
            libc::gmtime_r(&timestamp, broken.as_mut_ptr())
        } else {
            libc::localtime_r(&timestamp, broken.as_mut_ptr())
        };
        if result.is_null() {
            return None;
        }
        let tm = broken.assume_init();
        let offset = if utc { 0 } else { tm.tm_gmtoff };
        let zone = if offset == 0 {
            "Z".into()
        } else {
            let absolute = offset.abs() / 60;
            format!(
                "{}{:02}'{:02}'",
                if offset < 0 { '-' } else { '+' },
                absolute / 60,
                absolute % 60
            )
        };
        Some(format!(
            "D:{:04}{:02}{:02}{:02}{:02}{:02}{zone}",
            tm.tm_year + 1900,
            tm.tm_mon + 1,
            tm.tm_mday,
            tm.tm_hour,
            tm.tm_min,
            tm.tm_sec
        ))
    }
}
static CREATION_DATE: OnceLock<String> = OnceLock::new();
pub fn creation_date() -> &'static str {
    CREATION_DATE.get_or_init(|| {
        let source = std::env::var("SOURCE_DATE_EPOCH").ok();
        if let Some(epoch) = source_date_epoch(source.as_deref()) {
            if let Some(date) = date_at(epoch, true) {
                return date;
            }
        }
        let epoch = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|value| value.as_secs() as i64)
            .unwrap_or(0);
        date_at(epoch, false).unwrap_or_else(|| "D:19700101000000Z".into())
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reproducible_epoch_and_leap_day() {
        assert_eq!(date_at(32, true).unwrap(), "D:19700101000032Z");
        assert_eq!(date_at(951782400, true).unwrap(), "D:20000229000000Z");
        assert_eq!(source_date_epoch(Some("32")), Some(32));
        assert_eq!(source_date_epoch(Some("-1")), None);
    }
    #[test]
    fn omission_keeps_explicit_user_dates() {
        let mut info = BTreeMap::from([("CreationDate".into(), "(D:20000101000000Z)".into())]);
        MetadataPolicy {
            omit_automatic_dates: true,
            ..MetadataPolicy::default()
        }
        .automatic_info(&mut info, "D:19700101000032Z", "Pitex");
        assert_eq!(info["CreationDate"], "(D:20000101000000Z)");
        assert!(!info.contains_key("ModDate"));
    }
    #[test]
    fn ptex_masks_and_new_key_names() {
        let normal = MetadataPolicy::default();
        assert_eq!(
            normal.ptex_key("Fullbanner").as_deref(),
            Some("PTEX.Fullbanner")
        );
        let masked = MetadataPolicy {
            suppress_ptex: 5,
            ..normal
        };
        assert!(masked.ptex_key("Fullbanner").is_none());
        assert!(masked.ptex_key("PageNumber").is_none());
        assert_eq!(
            masked.ptex_key("FileName").as_deref(),
            Some("PTEX.FileName")
        );
        assert!(MetadataPolicy {
            suppress_ptex: -1,
            ..normal
        }
        .ptex_key("InfoDict")
        .is_none());
        assert_eq!(
            MetadataPolicy {
                use_ptex_underscore: 1,
                ..normal
            }
            .ptex_key("Fullbanner")
            .as_deref(),
            Some("PTEX_Fullbanner")
        );
        assert_eq!(
            MetadataPolicy {
                pdf_major_version: 2,
                ..normal
            }
            .ptex_key("InfoDict")
            .as_deref(),
            Some("PTEX_InfoDict")
        );
    }
}
