use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime};

const KEY_URL: &str = "https://openrouter.ai/api/v1/key";
const USER_AGENT: &str = concat!("orwatch/", env!("CARGO_PKG_VERSION"));

#[derive(Debug, Clone, thiserror::Error)]
pub enum ApiError {
    #[error("openrouter HTTP {status}: {body}")]
    Http { status: u16, body: String },
    #[error("openrouter request failed: {0}")]
    Transport(String),
    #[error("could not parse openrouter response: {0}")]
    Decode(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyResponse {
    pub data: KeyData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyData {
    pub label: Option<String>,
    pub limit: Option<f64>,
    pub limit_reset: Option<String>,
    pub limit_remaining: Option<f64>,
    #[serde(default)]
    pub include_byok_in_limit: bool,
    #[serde(default)]
    pub usage: f64,
    #[serde(default)]
    pub usage_daily: f64,
    #[serde(default)]
    pub usage_weekly: f64,
    #[serde(default)]
    pub usage_monthly: f64,
    #[serde(default)]
    pub is_free_tier: bool,
}

impl KeyData {
    pub fn remaining_pct(&self) -> Option<f64> {
        let limit = self.limit?;
        let remaining = self.limit_remaining?;
        if limit <= 0.0 {
            return None;
        }
        Some((remaining / limit) * 100.0)
    }
}

pub fn fetch_key(api_key: &str) -> Result<KeyResponse, ApiError> {
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(8))
        .user_agent(USER_AGENT)
        .build();

    let response = agent
        .get(KEY_URL)
        .set("Authorization", &format!("Bearer {api_key}"))
        .call()
        .map_err(|err| match err {
            ureq::Error::Status(status, resp) => {
                let body = resp.into_string().unwrap_or_default();
                ApiError::Http { status, body }
            }
            other => ApiError::Transport(other.to_string()),
        })?;

    response
        .into_json()
        .map_err(|err| ApiError::Decode(err.to_string()))
}

pub fn load_cache(path: &Path, ttl: Duration) -> Option<KeyResponse> {
    let meta = fs::metadata(path).ok()?;
    let age = SystemTime::now()
        .duration_since(meta.modified().ok()?)
        .ok()?;
    if age > ttl {
        return None;
    }
    let raw = fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

pub fn save_cache(path: &Path, snapshot: &KeyResponse) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(raw) = serde_json::to_string_pretty(snapshot) {
        let _ = fs::write(path, raw);
    }
}

/// Fresh API call, falling back to a *stale* cache if the network fails.
pub fn fetch_or_cache(
    api_key: &str,
    cache: &Path,
    ttl: Duration,
    allow_stale: bool,
) -> Result<KeyResponse, ApiError> {
    if let Some(hit) = load_cache(cache, ttl) {
        return Ok(hit);
    }
    match fetch_key(api_key) {
        Ok(fresh) => {
            save_cache(cache, &fresh);
            Ok(fresh)
        }
        Err(err) if allow_stale => {
            let raw = fs::read_to_string(cache).map_err(|_| err.clone())?;
            serde_json::from_str(&raw).map_err(|_| err)
        }
        Err(err) => Err(err),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn sample() -> KeyResponse {
        KeyResponse {
            data: KeyData {
                label: Some("tui".into()),
                limit: Some(5.0),
                limit_reset: None,
                limit_remaining: Some(4.12),
                include_byok_in_limit: false,
                usage: 0.88,
                usage_daily: 0.18,
                usage_weekly: 0.88,
                usage_monthly: 0.88,
                is_free_tier: false,
            },
        }
    }

    #[test]
    fn remaining_pct() {
        let pct = sample().data.remaining_pct().unwrap();
        assert!((pct - 82.4).abs() < 0.01);
    }

    #[test]
    fn cache_roundtrip() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("snap.json");
        let snap = sample();
        save_cache(&path, &snap);
        let loaded = load_cache(&path, Duration::from_secs(60)).unwrap();
        assert_eq!(loaded.data.usage_daily, 0.18);
    }

    #[test]
    fn expired_cache_is_none() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("snap.json");
        save_cache(&path, &sample());
        assert!(load_cache(&path, Duration::from_secs(0)).is_none());
    }
}
