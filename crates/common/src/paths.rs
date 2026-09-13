use std::path::PathBuf;

/// Returns the root configuration and state directory for Sidecrab.
/// Respects `SIDECRAB_HOME` environment variable override if set (useful for test harnesses).
/// Defaults to `%USERPROFILE%/.sidecrab` on Windows, or `~/.sidecrab`.
pub fn sidecrab_dir() -> PathBuf {
    if let Ok(val) = std::env::var("SIDECRAB_HOME") {
        if !val.trim().is_empty() {
            return PathBuf::from(val);
        }
    }
    user_home_dir().join(".sidecrab")
}

/// Returns the path to `%USERPROFILE%/.sidecrab/state.json`.
pub fn state_path() -> PathBuf {
    sidecrab_dir().join("state.json")
}

/// Returns the path to `%USERPROFILE%/.sidecrab/config.json`.
pub fn config_path() -> PathBuf {
    sidecrab_dir().join("config.json")
}

/// Returns the path to `%USERPROFILE%/.sidecrab/sessions.d`.
pub fn sessions_dir() -> PathBuf {
    sidecrab_dir().join("sessions.d")
}

/// Returns the root Claude directory `%USERPROFILE%/.claude`.
/// Respects `CLAUDE_HOME` if set.
pub fn claude_dir() -> PathBuf {
    if let Ok(val) = std::env::var("CLAUDE_HOME") {
        if !val.trim().is_empty() {
            return PathBuf::from(val);
        }
    }
    user_home_dir().join(".claude")
}

/// Returns the path to `%USERPROFILE%/.claude/settings.json`.
/// Respects `CLAUDE_SETTINGS_PATH` if set.
pub fn claude_settings_path() -> PathBuf {
    if let Ok(val) = std::env::var("CLAUDE_SETTINGS_PATH") {
        if !val.trim().is_empty() {
            return PathBuf::from(val);
        }
    }
    claude_dir().join("settings.json")
}

/// Returns the path to `%USERPROFILE%/.claude/settings.json.bak`.
pub fn claude_settings_backup_path() -> PathBuf {
    let base = claude_settings_path();
    let mut file_name = base.file_name().unwrap_or_default().to_os_string();
    file_name.push(".bak");
    base.with_file_name(file_name)
}

/// Cross-platform helper to locate the user's home directory.
pub fn user_home_dir() -> PathBuf {
    if let Ok(val) = std::env::var("USERPROFILE") {
        if !val.trim().is_empty() {
            return PathBuf::from(val);
        }
    }
    if let Ok(val) = std::env::var("HOME") {
        if !val.trim().is_empty() {
            return PathBuf::from(val);
        }
    }
    if let (Ok(drive), Ok(path)) = (std::env::var("HOMEDRIVE"), std::env::var("HOMEPATH")) {
        let combined = format!("{}{}", drive, path);
        if !combined.trim().is_empty() {
            return PathBuf::from(combined);
        }
    }
    PathBuf::from(".")
}
