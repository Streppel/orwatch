use serde::Deserialize;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Daily spend (USD) that turns the waybar module yellow.
    pub warn_daily: f64,
    /// Key remaining (USD) at or below which the module goes red.
    pub warn_remaining: f64,
    /// How long a successful API snapshot is reused.
    pub cache_secs: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            warn_daily: 2.0,
            warn_remaining: 1.0,
            cache_secs: 50,
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let path = config_path();
        let Ok(raw) = fs::read_to_string(path) else {
            return Self::default();
        };
        toml::from_str(&raw).unwrap_or_default()
    }

    pub fn cache_ttl(&self) -> Duration {
        Duration::from_secs(self.cache_secs.max(5))
    }
}

pub fn config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("orwatch/config.toml")
}

pub fn cache_path() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("orwatch/snapshot.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toml_overrides_defaults() {
        let cfg: Config = toml::from_str("warn_daily = 0.5\nwarn_remaining = 0.2\n").unwrap();
        assert_eq!(cfg.warn_daily, 0.5);
        assert_eq!(cfg.warn_remaining, 0.2);
        assert_eq!(cfg.cache_secs, 50);
    }
}
