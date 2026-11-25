//! Assistant usage and turn polish shared by the panel and its tests: the pushed
//! `usage_update` totals, the context meter, failed-turn codes, the thinking
//! block's open state, and the transcript's scroll pinning.

use serde_json::{Map, Value};

/// Latest provider usage (`usage_update`); each event carries the known fields.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Usage {
    pub input_tokens: Option<u64>,
    pub cached_input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub thinking_tokens: Option<u64>,
    pub cost_usd: Option<f64>,
    pub context_used: Option<u64>,
    pub context_max: Option<u64>,
}

impl Usage {
    pub fn merge(&mut self, event: &Map<String, Value>) {
        let int = |key: &str| event.get(key).and_then(Value::as_f64).filter(|v| v.is_finite() && *v >= 0.0).map(|v| v as u64);
        let set = |slot: &mut Option<u64>, value: Option<u64>| {
            if value.is_some() {
                *slot = value;
            }
        };
        set(&mut self.input_tokens, int("inputTokens"));
        set(&mut self.cached_input_tokens, int("cachedInputTokens"));
        set(&mut self.output_tokens, int("outputTokens"));
        set(&mut self.thinking_tokens, int("thinkingTokens"));
        set(&mut self.context_used, int("contextUsed"));
        set(&mut self.context_max, int("contextMax"));
        if let Some(cost) = event.get("costUsd").and_then(Value::as_f64).filter(|v| v.is_finite() && *v >= 0.0) {
            self.cost_usd = Some(cost);
        }
    }
}

/// Paseo's meter colors: amber from 70 %, red above 90 %.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MeterBand {
    Normal,
    Warning,
    Danger,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ContextMeter {
    /// Used share of the window, clamped to 0–100.
    pub percent: f64,
    pub used: u64,
    pub max: u64,
}

impl ContextMeter {
    pub fn new(used: u64, max: u64) -> Option<Self> {
        (max > 0).then(|| Self { percent: (used as f64 / max as f64 * 100.0).clamp(0.0, 100.0), used, max })
    }

    pub fn band(&self) -> MeterBand {
        if self.percent > 90.0 {
            MeterBand::Danger
        } else if self.percent >= 70.0 {
            MeterBand::Warning
        } else {
            MeterBand::Normal
        }
    }

    /// "38% · 76k/200k".
    pub fn label(&self) -> String {
        format!("{}% · {}/{}", self.percent.round() as u64, compact_tokens(self.used), compact_tokens(self.max))
    }
}

/// 950, 7.6k, 76k, 1.2M.
pub fn compact_tokens(value: u64) -> String {
    match value {
        0..=999 => value.to_string(),
        1_000..=9_999 => format!("{:.1}k", value as f64 / 1_000.0),
        10_000..=999_999 => format!("{}k", (value as f64 / 1_000.0).round() as u64),
        _ => {
            let m = value as f64 / 1_000_000.0;
            if (m - m.round()).abs() < 0.05 { format!("{}M", m.round() as u64) } else { format!("{m:.1}M") }
        }
    }
}

/// `agent_end.code` from the bridge, plus `Crashed` for a process that exited mid-chat.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum ErrorCode {
    SignedOut,
    RateLimited,
    Network,
    ContextFull,
    #[default]
    Other,
    Crashed,
}

impl ErrorCode {
    pub fn decode(code: Option<&str>) -> Self {
        match code {
            Some("signed_out") => Self::SignedOut,
            Some("rate_limited") => Self::RateLimited,
            Some("network") => Self::Network,
            Some("context_full") => Self::ContextFull,
            _ => Self::Other,
        }
    }

    /// The `Localizable.strings` key of the friendly line; `%@` is the backend name.
    pub fn message_key(self, can_compact: bool) -> &'static str {
        match self {
            Self::SignedOut => "assistant.error.signed_out",
            Self::RateLimited => "assistant.error.rate_limited",
            Self::Network => "assistant.error.network",
            Self::ContextFull if can_compact => "assistant.error.context_full_compact",
            Self::ContextFull => "assistant.error.context_full",
            Self::Other => "assistant.error.other",
            Self::Crashed => "assistant.error.crashed",
        }
    }
}

/// The CLI a provider id drives, for "Sign in to Claude Code in a terminal".
pub fn cli_name(provider: &str) -> &str {
    match provider {
        "claude" => "Claude Code",
        "agy" => "Antigravity",
        "codex" => "Codex",
        "grok" => "Grok",
        "pi" => "Pitex Agent",
        other => other,
    }
}

/// A thinking block is open while it streams, then closed, unless the user toggled it.
pub fn thinking_expanded(streaming: bool, user_choice: Option<bool>) -> bool {
    user_choice.unwrap_or(streaming)
}

/// How close to the bottom still counts as "at the bottom" (T3 and Paseo use about this).
pub const PIN_SLACK: f64 = 40.0;

/// Auto-scroll follows new content only while the view is at the bottom; otherwise a
/// "Jump to latest" button appears until the user goes back down.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScrollPin {
    pub pinned: bool,
    /// New content arrived while unpinned: show "Jump to latest".
    pub unseen: bool,
}

impl Default for ScrollPin {
    fn default() -> Self {
        Self { pinned: true, unseen: false }
    }
}

impl ScrollPin {
    /// The user (or layout) moved the view: `value + page` against `upper`.
    pub fn scrolled(&mut self, value: f64, page: f64, upper: f64) {
        self.pinned = upper - (value + page) <= PIN_SLACK;
        if self.pinned {
            self.unseen = false;
        }
    }

    /// New content: true when the view should scroll to the bottom.
    pub fn content_added(&mut self) -> bool {
        if !self.pinned {
            self.unseen = true;
        }
        self.pinned
    }

    /// "Jump to latest" or a new message from the user.
    pub fn jump(&mut self) {
        self.pinned = true;
        self.unseen = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn meter_clamps_and_bands_at_70_and_90() {
        let meter = ContextMeter::new(76_000, 200_000).unwrap();
        assert_eq!(meter.label(), "38% · 76k/200k");
        assert_eq!(meter.band(), MeterBand::Normal);
        assert_eq!(ContextMeter::new(139_999, 200_000).unwrap().band(), MeterBand::Normal);
        assert_eq!(ContextMeter::new(140_000, 200_000).unwrap().band(), MeterBand::Warning);
        assert_eq!(ContextMeter::new(180_000, 200_000).unwrap().band(), MeterBand::Warning);
        assert_eq!(ContextMeter::new(180_001, 200_000).unwrap().band(), MeterBand::Danger);
        assert_eq!(ContextMeter::new(300_000, 200_000).unwrap().percent, 100.0);
        assert!(ContextMeter::new(5, 0).is_none());
        assert_eq!(compact_tokens(950), "950");
        assert_eq!(compact_tokens(7_600), "7.6k");
        assert_eq!(compact_tokens(1_000_000), "1M");
        assert_eq!(compact_tokens(1_250_000), "1.2M");
    }

    #[test]
    fn usage_merge_keeps_fields_an_update_omits() {
        let mut usage = Usage::default();
        usage.merge(json!({"inputTokens": 10, "contextUsed": 100, "contextMax": 1000}).as_object().unwrap());
        usage.merge(json!({"outputTokens": 5, "costUsd": 0.25, "contextUsed": 120, "turnId": "t"}).as_object().unwrap());
        assert_eq!(usage.input_tokens, Some(10));
        assert_eq!(usage.output_tokens, Some(5));
        assert_eq!(usage.context_used, Some(120));
        assert_eq!(usage.context_max, Some(1000));
        assert_eq!(usage.cost_usd, Some(0.25));
        usage.merge(json!({"contextUsed": -1}).as_object().unwrap());
        assert_eq!(usage.context_used, Some(120));
    }

    #[test]
    fn error_codes_and_their_lines() {
        assert_eq!(ErrorCode::decode(Some("rate_limited")), ErrorCode::RateLimited);
        assert_eq!(ErrorCode::decode(Some("mystery")), ErrorCode::Other);
        assert_eq!(ErrorCode::decode(None), ErrorCode::Other);
        assert_eq!(ErrorCode::ContextFull.message_key(true), "assistant.error.context_full_compact");
        assert_eq!(ErrorCode::ContextFull.message_key(false), "assistant.error.context_full");
    }

    #[test]
    fn thinking_is_open_while_streaming_then_closed_unless_toggled() {
        assert!(thinking_expanded(true, None));
        assert!(!thinking_expanded(false, None));
        assert!(thinking_expanded(false, Some(true)));
        assert!(!thinking_expanded(true, Some(false)));
    }

    #[test]
    fn scroll_pin_follows_at_the_bottom_and_offers_jump_when_scrolled_up() {
        let mut pin = ScrollPin::default();
        assert!(pin.content_added());
        pin.scrolled(560.0, 400.0, 1000.0); // 40 px from the bottom: still pinned
        assert!(pin.pinned);
        pin.scrolled(300.0, 400.0, 1000.0);
        assert!(!pin.pinned);
        assert!(!pin.content_added(), "scrolled up: new content does not move the view");
        assert!(pin.unseen);
        pin.jump();
        assert!(pin.pinned && !pin.unseen);
        pin.scrolled(0.0, 400.0, 1000.0);
        pin.content_added();
        pin.scrolled(600.0, 400.0, 1000.0); // the user went back down
        assert!(pin.pinned && !pin.unseen);
    }
}
