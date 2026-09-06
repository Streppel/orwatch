//! Where the OpenRouter API key comes from.
//!
//! Order: `OPENROUTER_API_KEY` env, then `~/.secrets` (the file sourced by zsh).
//! Waybar does not load `.zshrc`, so reading `~/.secrets` is what makes the
//! bar module work.

use std::env;
use std::fs;
use std::path::PathBuf;

const PLACEHOLDERS: &[&str] = &["REPLACE-ME", "sk-or-v1-REPLACE", "changeme"];

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum KeyError {
    #[error("OPENROUTER_API_KEY is not set (env or ~/.secrets)")]
    Missing,
    #[error("OPENROUTER_API_KEY in {0} is still the placeholder")]
    Placeholder(PathBuf),
}

pub fn load_api_key() -> Result<String, KeyError> {
    if let Ok(raw) = env::var("OPENROUTER_API_KEY") {
        return validate(raw, None);
    }
    let path = secrets_path();
    if !path.is_file() {
        return Err(KeyError::Missing);
    }
    let contents = fs::read_to_string(&path).map_err(|_| KeyError::Missing)?;
    match parse_secrets_file(&contents) {
        Some(raw) => validate(raw, Some(path)),
        None => Err(KeyError::Missing),
    }
}

fn secrets_path() -> PathBuf {
    env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/"))
        .join(".secrets")
}

/// Pull `OPENROUTER_API_KEY` out of a shell-style secrets file.
pub fn parse_secrets_file(contents: &str) -> Option<String> {
    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let line = line.strip_prefix("export ").unwrap_or(line).trim();
        let Some((name, value)) = line.split_once('=') else {
            continue;
        };
        if name.trim() != "OPENROUTER_API_KEY" {
            continue;
        }
        return Some(unquote(value.trim()));
    }
    None
}

fn unquote(value: &str) -> String {
    let value = value.trim();
    if value.len() >= 2 {
        let bytes = value.as_bytes();
        if (bytes[0] == b'"' && bytes[value.len() - 1] == b'"')
            || (bytes[0] == b'\'' && bytes[value.len() - 1] == b'\'')
        {
            return value[1..value.len() - 1].to_string();
        }
    }
    value.to_string()
}

fn validate(raw: String, from: Option<PathBuf>) -> Result<String, KeyError> {
    let key = raw.trim().to_string();
    if key.is_empty() {
        return Err(KeyError::Missing);
    }
    if PLACEHOLDERS.iter().any(|p| key.contains(p)) {
        return Err(KeyError::Placeholder(
            from.unwrap_or_else(|| PathBuf::from("environment")),
        ));
    }
    Ok(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skips_comments_and_unquotes() {
        let file = r#"
# export OPENROUTER_API_KEY="sk-or-v1-REPLACE-ME"
export OPENROUTER_API_KEY="sk-or-v1-live"
"#;
        assert_eq!(parse_secrets_file(file).as_deref(), Some("sk-or-v1-live"));
    }

    #[test]
    fn accepts_unexported_assignment() {
        assert_eq!(
            parse_secrets_file("OPENROUTER_API_KEY=sk-or-v1-abc").as_deref(),
            Some("sk-or-v1-abc")
        );
    }

    #[test]
    fn validate_rejects_placeholder() {
        let err =
            validate("sk-or-v1-REPLACE-ME".into(), Some(PathBuf::from("/tmp/x"))).unwrap_err();
        assert!(matches!(err, KeyError::Placeholder(_)));
    }

    #[test]
    fn validate_rejects_empty() {
        assert_eq!(validate("  ".into(), None).unwrap_err(), KeyError::Missing);
    }
}
