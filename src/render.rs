use crate::api::KeyResponse;
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

/// Compact percentage for the waybar text (truncated, never rounds up).
pub fn pct_u32(pct: f64) -> u32 {
    pct.clamp(0.0, 100.0) as u32
}

/// One-decimal percentage for tooltips and the full report.
pub fn format_pct(pct: f64) -> String {
    if pct >= 99.95 {
        "100%".into()
    } else {
        format!("{pct:.1}%")
    }
}

pub fn classify(snapshot: &KeyResponse, cfg: &Config) -> StatusClass {
    let data = &snapshot.data;
    let balance = snapshot.balance();

    let low_remaining = data
        .limit_remaining
        .is_some_and(|remaining| remaining <= cfg.warn_remaining)
        || balance.is_some_and(|b| b.remaining <= cfg.warn_remaining);
    if low_remaining {
        return StatusClass::Critical;
    }

    let low_pct = data.remaining_pct().is_some_and(|percent| percent <= 25.0)
        || balance.is_some_and(|b| b.pct <= 25.0);
    if low_pct || data.usage_daily >= cfg.warn_daily {
        return StatusClass::Warning;
    }

    StatusClass::Normal
}

pub fn waybar_ok(snapshot: &KeyResponse, cfg: &Config) -> WaybarOut {
    let class = classify(snapshot, cfg);
    let data = &snapshot.data;
    let balance = snapshot.balance();

    // A key cap is the most specific budget; otherwise fall back to the
    // account balance. Only without either do we show today's spend.
    let (text_value, percent) = match data.limit_remaining {
        Some(remaining) => (usd(remaining), data.remaining_pct()),
        None => match balance {
            Some(balance) => (usd(balance.remaining), Some(balance.pct)),
            None => (usd(data.usage_daily), None),
        },
    };
    let text_value = match percent {
        Some(percent) => format!(
            "{text_value} <span foreground='#727169'>{}%</span>",
            pct_u32(percent)
        ),
        None => text_value,
    };
    let text = format!("<span foreground='#727169'>or</span>:{text_value}");

    WaybarOut {
        text,
        tooltip: tooltip(snapshot),
        class: class.as_str().to_string(),
        percentage: percent.map(pct_u32),
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

pub fn human(snapshot: &KeyResponse) -> String {
    let data = &snapshot.data;
    let mut lines = Vec::new();
    let title = match data.label.as_deref() {
        Some(label) if !label.is_empty() => format!("openrouter  {label}"),
        _ => "openrouter".into(),
    };
    lines.push(title);
    if let Some(balance) = snapshot.balance() {
        lines.push(format!(
            "  balance     {} left of {} ({})",
            usd(balance.remaining),
            usd(balance.total),
            format_pct(balance.pct)
        ));
    }
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

fn tooltip(snapshot: &KeyResponse) -> String {
    let data = &snapshot.data;
    let mut lines = vec![match data.label.as_deref() {
        Some(label) if !label.is_empty() => format!("OpenRouter · {label}"),
        _ => "OpenRouter".into(),
    }];
    if let Some(balance) = snapshot.balance() {
        lines.push(format!(
            "Balance   {} / {} ({})",
            usd(balance.remaining),
            usd(balance.total),
            format_pct(balance.pct)
        ));
    }
    lines.push(format!("Today     {}", usd(data.usage_daily)));
    lines.push(format!("Week      {}", usd(data.usage_weekly)));
    lines.push(format!("Month     {}", usd(data.usage_monthly)));
    lines.push(format!("Lifetime  {}", usd(data.usage)));
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
    use crate::api::{CreditsData, KeyData};

    fn key() -> KeyData {
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

    fn snapshot() -> KeyResponse {
        KeyResponse {
            data: key(),
            credits: Some(CreditsData {
                total_credits: 20.0,
                total_usage: 10.0,
            }),
        }
    }

    #[test]
    fn usd_formats_small_amounts() {
        assert_eq!(usd(0.0), "$0.00");
        assert_eq!(usd(1.2), "$1.20");
        assert_eq!(usd(0.004), "$0.004");
    }

    #[test]
    fn pct_formats() {
        assert_eq!(pct_u32(98.7), 98);
        assert_eq!(pct_u32(120.0), 100);
        assert_eq!(format_pct(98.7), "98.7%");
        assert_eq!(format_pct(100.0), "100%");
    }

    #[test]
    fn low_remaining_is_critical() {
        let cfg = Config::default();
        assert_eq!(classify(&snapshot(), &cfg), StatusClass::Critical);
    }

    #[test]
    fn high_daily_is_warning() {
        let mut snap = snapshot();
        snap.data.limit_remaining = Some(4.0);
        snap.data.usage_daily = 2.5;
        assert_eq!(classify(&snap, &Config::default()), StatusClass::Warning);
    }

    #[test]
    fn low_balance_is_warning() {
        let mut snap = snapshot();
        snap.data.limit = None;
        snap.data.limit_remaining = None;
        snap.credits = Some(CreditsData {
            total_credits: 20.0,
            total_usage: 17.0,
        });
        assert_eq!(classify(&snap, &Config::default()), StatusClass::Warning);
    }

    #[test]
    fn depleted_balance_is_critical() {
        let mut snap = snapshot();
        snap.data.limit = None;
        snap.data.limit_remaining = None;
        snap.credits = Some(CreditsData {
            total_credits: 20.0,
            total_usage: 19.5,
        });
        assert_eq!(classify(&snap, &Config::default()), StatusClass::Critical);
    }

    #[test]
    fn waybar_uses_remaining_when_capped() {
        let out = waybar_ok(&snapshot(), &Config::default());
        assert!(out.text.contains("$0.40"), "{}", out.text);
        assert!(out.text.contains("8%"), "{}", out.text);
        assert_eq!(out.class, "critical");
    }

    #[test]
    fn waybar_shows_balance_without_cap() {
        let mut snap = snapshot();
        snap.data.limit = None;
        snap.data.limit_remaining = None;
        let out = waybar_ok(&snap, &Config::default());
        assert!(out.text.contains("$10.00"), "{}", out.text);
        assert!(out.text.contains("50%"), "{}", out.text);
        assert_eq!(out.percentage, Some(50));
    }

    #[test]
    fn waybar_uses_daily_without_credits() {
        let mut snap = snapshot();
        snap.data.limit = None;
        snap.data.limit_remaining = None;
        snap.credits = None;
        let out = waybar_ok(&snap, &Config::default());
        assert!(out.text.contains("$0.18"), "{}", out.text);
        assert_eq!(out.percentage, None);
    }

    #[test]
    fn human_lists_balance_and_periods() {
        let text = human(&snapshot());
        assert!(text.contains("today"));
        assert!(text.contains("$10.00 left of $20.00 (50.0%)"), "{text}");
        assert!(text.contains("$0.40 left of $5.00"));
        assert!(text.contains("resets monthly"));
    }

    #[test]
    fn tooltip_lists_balance() {
        let text = tooltip(&snapshot());
        assert!(text.contains("Balance   $10.00 / $20.00 (50.0%)"), "{text}");
    }
}
