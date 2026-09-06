use crate::api::KeyData;
use crate::config::Config;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusClass {
    Normal,
    Warning,
    Critical,
    Disconnected,
}

impl StatusClass {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Warning => "warning",
            Self::Critical => "critical",
            Self::Disconnected => "disconnected",
        }
    }
}

#[derive(Debug, Serialize)]
pub struct WaybarOut {
    pub text: String,
    pub tooltip: String,
    pub class: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub percentage: Option<u32>,
}

pub fn usd(amount: f64) -> String {
    if amount == 0.0 {
        return "$0.00".into();
    }
    if amount.abs() < 0.01 {
        format!("${amount:.3}")
    } else {
        format!("${amount:.2}")
    }
}

pub fn classify(data: &KeyData, cfg: &Config) -> StatusClass {
    if let Some(remaining) = data.limit_remaining
        && remaining <= cfg.warn_remaining
    {
        return StatusClass::Critical;
    }
    if data.usage_daily >= cfg.warn_daily {
        return StatusClass::Warning;
    }
    if let Some(pct) = data.remaining_pct()
        && pct <= 25.0
    {
        return StatusClass::Warning;
    }
    StatusClass::Normal
}

pub fn waybar_ok(data: &KeyData, cfg: &Config) -> WaybarOut {
    let class = classify(data, cfg);
    let text_value = match data.limit_remaining {
        Some(remaining) => usd(remaining),
        None => usd(data.usage_daily),
    };
    let text = format!("<span foreground='#727169'>or</span>:{text_value}");
    WaybarOut {
        text,
        tooltip: tooltip(data),
        class: class.as_str().to_string(),
        percentage: data.remaining_pct().map(|p| p.clamp(0.0, 100.0) as u32),
    }
}

pub fn waybar_err(message: &str) -> WaybarOut {
    WaybarOut {
        text: "<span foreground='#727169'>or</span>: —".into(),
        tooltip: message.to_string(),
        class: StatusClass::Disconnected.as_str().to_string(),
        percentage: None,
    }
}

pub fn human(data: &KeyData) -> String {
    let mut lines = Vec::new();
    let title = match data.label.as_deref() {
        Some(label) if !label.is_empty() => format!("openrouter  {label}"),
        _ => "openrouter".into(),
    };
    lines.push(title);
    lines.push(format!("  today       {}", usd(data.usage_daily)));
    lines.push(format!("  week        {}", usd(data.usage_weekly)));
    lines.push(format!("  month       {}", usd(data.usage_monthly)));
    lines.push(format!("  lifetime    {}", usd(data.usage)));
    match (data.limit, data.limit_remaining) {
        (Some(limit), Some(remaining)) => {
            let reset = data
                .limit_reset
                .as_deref()
                .map(|r| format!("  resets {r}"))
                .unwrap_or_default();
            lines.push(format!(
                "  key cap     {} left of {}{reset}",
                usd(remaining),
                usd(limit)
            ));
        }
        _ => lines.push("  key cap     none (account balance still applies)".into()),
    }
    if data.is_free_tier {
        lines.push("  tier        free (no credits purchased yet)".into());
    }
    lines.join("\n")
}

fn tooltip(data: &KeyData) -> String {
    let mut lines = vec![
        match data.label.as_deref() {
            Some(label) if !label.is_empty() => format!("OpenRouter · {label}"),
            _ => "OpenRouter".into(),
        },
        format!("Today     {}", usd(data.usage_daily)),
        format!("Week      {}", usd(data.usage_weekly)),
        format!("Month     {}", usd(data.usage_monthly)),
        format!("Lifetime  {}", usd(data.usage)),
    ];
    match (data.limit, data.limit_remaining) {
        (Some(limit), Some(remaining)) => {
            lines.push(format!("Key cap   {} / {}", usd(remaining), usd(limit)));
        }
        _ => lines.push("Key cap   none".into()),
    }
    lines.push("Click: full report".into());
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data() -> KeyData {
        KeyData {
            label: Some("tui".into()),
            limit: Some(5.0),
            limit_reset: Some("monthly".into()),
            limit_remaining: Some(0.40),
            include_byok_in_limit: false,
            usage: 4.60,
            usage_daily: 0.18,
            usage_weekly: 1.02,
            usage_monthly: 4.60,
            is_free_tier: false,
        }
    }

    #[test]
    fn usd_formats_small_amounts() {
        assert_eq!(usd(0.0), "$0.00");
        assert_eq!(usd(1.2), "$1.20");
        assert_eq!(usd(0.004), "$0.004");
    }

    #[test]
    fn low_remaining_is_critical() {
        let cfg = Config::default();
        assert_eq!(classify(&data(), &cfg), StatusClass::Critical);
    }

    #[test]
    fn high_daily_is_warning() {
        let mut d = data();
        d.limit_remaining = Some(4.0);
        d.usage_daily = 2.5;
        assert_eq!(classify(&d, &Config::default()), StatusClass::Warning);
    }

    #[test]
    fn waybar_uses_remaining_when_capped() {
        let out = waybar_ok(&data(), &Config::default());
        assert!(out.text.contains("$0.40"), "{}", out.text);
        assert_eq!(out.class, "critical");
    }

    #[test]
    fn waybar_uses_daily_without_cap() {
        let mut d = data();
        d.limit = None;
        d.limit_remaining = None;
        let out = waybar_ok(&d, &Config::default());
        assert!(out.text.contains("$0.18"), "{}", out.text);
    }

    #[test]
    fn human_lists_periods() {
        let text = human(&data());
        assert!(text.contains("today"));
        assert!(text.contains("$0.40 left of $5.00"));
        assert!(text.contains("resets monthly"));
    }
}
