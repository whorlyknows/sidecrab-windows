use common::{claude_dir, claude_settings_path, HookEvent};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

fn resolve_settings_path(custom: Option<&str>) -> PathBuf {
    if let Some(p) = custom {
        if !p.trim().is_empty() {
            return PathBuf::from(p);
        }
    }
    claude_settings_path()
}

fn resolve_backup_path(settings_path: &Path) -> PathBuf {
    let mut f = settings_path.file_name().unwrap_or_default().to_os_string();
    f.push(".bak");
    settings_path.with_file_name(f)
}

fn hook_exe_path() -> String {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(s) = exe.to_str() {
            return s.to_string();
        }
    }
    "sidecrab-hook.exe".to_string()
}

fn is_sidecrab_entry(val: &Value) -> bool {
    let s = val.to_string().to_lowercase();
    s.contains("sidecrab-hook") || s.contains("sidecrab_hook") || s.contains("clawd-pet-hook")
}

/// Idempotently installs Sidecrab hooks into Claude settings.json with a pristine backup.
pub fn install_hooks(custom_path: Option<&str>) -> Result<(), String> {
    let settings_path = resolve_settings_path(custom_path);
    let backup_path = resolve_backup_path(&settings_path);

    if let Some(parent) = settings_path.parent() {
        let _ = fs::create_dir_all(parent);
    } else {
        let _ = fs::create_dir_all(claude_dir());
    }

    // 1. Create pristine backup if settings.json exists and backup does not exist yet
    if settings_path.exists() && !backup_path.exists() {
        if let Err(e) = fs::copy(&settings_path, &backup_path) {
            return Err(format!("Failed to create backup at {:?}: {}", backup_path, e));
        }
    }

    // 2. Read existing settings or create empty object
    let mut root: Value = if settings_path.exists() {
        let content = fs::read_to_string(&settings_path)
            .map_err(|e| format!("Failed to read {:?}: {}", settings_path, e))?;
        serde_json::from_str(&content).unwrap_or_else(|_| json!({}))
    } else {
        json!({})
    };

    if !root.is_object() {
        root = json!({});
    }

    let hooks_obj = root
        .as_object_mut()
        .unwrap()
        .entry("hooks")
        .or_insert_with(|| json!({}));

    if !hooks_obj.is_object() {
        *hooks_obj = json!({});
    }
    let hooks_map = hooks_obj.as_object_mut().unwrap();

    let exe = hook_exe_path();

    // 3. Register each of the 8 Claude Code hook events
    for ev in HookEvent::ALL {
        let ev_name = ev.as_str();
        let short_name = ev.short_name();

        let needs_matcher = matches!(
            ev,
            HookEvent::PreToolUse | HookEvent::PostToolUse | HookEvent::PermissionRequest
        );

        let hook_entry = if needs_matcher {
            json!({
                "matcher": "*",
                "hooks": [{
                    "type": "command",
                    "command": exe,
                    "args": [short_name]
                }]
            })
        } else {
            json!({
                "hooks": [{
                    "type": "command",
                    "command": exe,
                    "args": [short_name]
                }]
            })
        };

        let ev_array = hooks_map
            .entry(ev_name)
            .or_insert_with(|| json!([]));

        if !ev_array.is_array() {
            *ev_array = json!([]);
        }

        let arr = ev_array.as_array_mut().unwrap();
        // Remove previous sidecrab entries to ensure idempotency
        arr.retain(|item| !is_sidecrab_entry(item));
        arr.push(hook_entry);
    }

    // 4. Atomic write via temporary file
    let tmp_file = settings_path.with_extension(format!("tmp.{}", std::process::id()));
    let json_output = serde_json::to_string_pretty(&root)
        .map_err(|e| format!("Serialization error: {}", e))?;

    fs::write(&tmp_file, json_output)
        .map_err(|e| format!("Failed writing {:?}: {}", tmp_file, e))?;

    fs::rename(&tmp_file, &settings_path)
        .map_err(|e| format!("Failed renaming to {:?}: {}", settings_path, e))?;

    Ok(())
}

/// Cleanly removes only Sidecrab hooks from settings.json, leaving all user config intact.
pub fn uninstall_hooks(custom_path: Option<&str>) -> Result<(), String> {
    let settings_path = resolve_settings_path(custom_path);
    if !settings_path.exists() {
        return Ok(());
    }

    let content = fs::read_to_string(&settings_path)
        .map_err(|e| format!("Failed to read {:?}: {}", settings_path, e))?;
    let mut root: Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed parsing JSON: {}", e))?;

    if let Some(hooks_obj) = root.get_mut("hooks").and_then(|h| h.as_object_mut()) {
        let mut keys_to_remove = Vec::new();

        for (event_key, event_val) in hooks_obj.iter_mut() {
            if let Some(arr) = event_val.as_array_mut() {
                arr.retain(|item| !is_sidecrab_entry(item));
                if arr.is_empty() {
                    keys_to_remove.push(event_key.clone());
                }
            }
        }

        for k in keys_to_remove {
            hooks_obj.remove(&k);
        }
    }

    // Atomic write via temporary file
    let tmp_file = settings_path.with_extension(format!("tmp.{}", std::process::id()));
    let json_output = serde_json::to_string_pretty(&root)
        .map_err(|e| format!("Serialization error: {}", e))?;

    fs::write(&tmp_file, json_output)
        .map_err(|e| format!("Failed writing {:?}: {}", tmp_file, e))?;

    fs::rename(&tmp_file, &settings_path)
        .map_err(|e| format!("Failed renaming to {:?}: {}", settings_path, e))?;

    Ok(())
}

/// Checks whether Sidecrab hooks are currently installed.
pub fn is_installed(custom_path: Option<&str>) -> bool {
    let settings_path = resolve_settings_path(custom_path);
    if !settings_path.exists() {
        return false;
    }

    if let Ok(content) = fs::read_to_string(&settings_path) {
        if let Ok(root) = serde_json::from_str::<Value>(&content) {
            if let Some(hooks_obj) = root.get("hooks").and_then(|h| h.as_object()) {
                for (_, val) in hooks_obj {
                    if let Some(arr) = val.as_array() {
                        for item in arr {
                            if is_sidecrab_entry(item) {
                                return true;
                            }
                        }
                    }
                }
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_installer_lifecycle_and_idempotency() {
        let temp_dir = std::env::temp_dir().join(format!("sidecrab_test_inst_{}", std::process::id()));
        let _ = fs::create_dir_all(&temp_dir);
        let settings_file = temp_dir.join("settings.json");
        let backup_file = temp_dir.join("settings.json.bak");

        // Initial settings with pre-existing user configuration
        let initial_json = r#"{
            "theme": "dark",
            "hooks": {
                "UserPromptSubmit": [
                    { "type": "command", "command": "other-tool.exe", "args": [] }
                ]
            }
        }"#;
        fs::write(&settings_file, initial_json).expect("write initial settings");

        let custom_path = settings_file.to_str().unwrap();

        // 1. First install
        install_hooks(Some(custom_path)).expect("install hooks 1");
        assert!(is_installed(Some(custom_path)));
        assert!(backup_file.exists(), "Backup file must be created on first install");

        let backup_content = fs::read_to_string(&backup_file).expect("read backup");
        assert!(backup_content.contains("other-tool.exe"));
        assert!(!backup_content.contains("sidecrab-hook"), "Backup must be pristine");

        // 2. Second install (idempotency check)
        install_hooks(Some(custom_path)).expect("install hooks 2");
        let installed_content = fs::read_to_string(&settings_file).expect("read installed");
        let val: Value = serde_json::from_str(&installed_content).expect("parse json");
        let user_prompt_array = val["hooks"]["UserPromptSubmit"].as_array().unwrap();
        
        // Should have 1 other-tool + 1 sidecrab-hook, NOT duplicated!
        assert_eq!(user_prompt_array.len(), 2, "Entries must not be duplicated");

        // 3. Uninstall
        uninstall_hooks(Some(custom_path)).expect("uninstall hooks");
        assert!(!is_installed(Some(custom_path)), "Hooks should be removed");

        let uninstalled_content = fs::read_to_string(&settings_file).expect("read uninstalled");
        let val_uninst: Value = serde_json::from_str(&uninstalled_content).expect("parse json");
        assert_eq!(val_uninst["theme"], "dark", "User settings must be preserved");
        assert_eq!(val_uninst["hooks"]["UserPromptSubmit"].as_array().unwrap().len(), 1, "Other hooks must be preserved");

        // Cleanup
        let _ = fs::remove_dir_all(&temp_dir);
    }
}

