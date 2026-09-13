use common::{
    sessions_dir, sidecrab_dir, state_path, tool_label, HookEvent, RawHookPayload, StateJson,
};
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::host::detect_host_terminal;

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn truncate_str(s: &str, max_len: usize) -> String {
    let clean = s.replace(['\r', '\n'], " ");
    let trimmed = clean.trim();
    if trimmed.chars().count() > max_len {
        let truncated: String = trimmed.chars().take(max_len).collect();
        format!("{}…", truncated)
    } else {
        trimmed.to_string()
    }
}

/// Dispatches a Claude Code hook event, updating sessions.d and atomically writing state.json.
pub fn handle_hook_event(event: HookEvent, payload: &RawHookPayload) {
    let sid = payload
        .session_id
        .clone()
        .unwrap_or_default()
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
        .collect::<String>();

    let sessions_path = sessions_dir();
    let _ = fs::create_dir_all(&sessions_path);

    // Multi-session tracking in sessions.d/
    if !sid.is_empty() {
        let session_file_raw = sessions_path.join(&sid);
        let session_file_json = sessions_path.join(format!("{}.json", sid));
        if event == HookEvent::SessionStart {
            let session_info = serde_json::json!({
                "session_id": sid,
                "created_at": current_timestamp(),
                "cwd": payload.cwd,
                "source": payload.source,
            });
            let info_str = session_info.to_string();
            let _ = fs::write(&session_file_raw, &info_str);
            let _ = fs::write(&session_file_json, &info_str);
        } else if event == HookEvent::SessionEnd {
            let _ = fs::remove_file(&session_file_raw);
            let _ = fs::remove_file(&session_file_json);
        }
    }

    // Read previous state.json to preserve persistent context if needed
    let state_file = state_path();
    let prev_state = if let Ok(data) = fs::read_to_string(&state_file) {
        serde_json::from_str::<StateJson>(data.trim_start_matches('\u{feff}').trim()).unwrap_or_default()
    } else {
        StateJson::default()
    };

    let prev_sid = prev_state.session_id();

    // If SessionStart is fired by a secondary session while primary session is actively working,
    // do not stomp the primary session's active state!
    if event == HookEvent::SessionStart
        && !prev_sid.is_empty()
        && !sid.is_empty()
        && prev_sid != sid
        && prev_state.state != "idle"
    {
        return;
    }

    // If SessionEnd is fired by a session that is NOT the active owner of state.json,
    // do not stomp the active session's state!
    if event == HookEvent::SessionEnd
        && !prev_sid.is_empty()
        && !sid.is_empty()
        && prev_sid != sid
    {
        return;
    }

    let now = current_timestamp();
    let (host_name, host_pid, host_hwnd) = unsafe { detect_host_terminal() };

    let project_name = payload
        .cwd
        .as_ref()
        .and_then(|p| Path::new(p).file_name())
        .and_then(|f| f.to_str())
        .map(|s| s.to_string());

    let mut new_state = StateJson {
        version: Some(1),
        state: "idle".to_string(),
        mood: Some("neutral".to_string()),
        label: Some("Idle".to_string()),
        tool: None,
        prompt: prev_state.prompt.clone(),
        prompt_preview: prev_state.prompt_preview.clone(),
        project: project_name.or(prev_state.project),
        cwd: payload.cwd.clone().or(prev_state.cwd),
        session_id_camel: Some(sid.clone()),
        session_id_snake: Some(sid.clone()),
        active_session_id: Some(sid),
        transcript: payload.transcript_path.clone().or(prev_state.transcript),
        host: host_name.or(prev_state.host),
        host_hwnd: if host_hwnd != 0 {
            Some(host_hwnd)
        } else {
            prev_state.host_hwnd
        },
        host_pid: if host_pid != 0 {
            Some(host_pid)
        } else {
            prev_state.host_pid
        },
        started_at_camel: prev_state.started_at_camel,
        started_at_snake: prev_state.started_at_snake,
        ts: Some(now),
        timestamp: Some(now),
    };

    match event {
        HookEvent::UserPromptSubmit => {
            new_state.state = "thinking".to_string();
            new_state.mood = Some("thinking".to_string());
            new_state.label = Some("Thinking…".to_string());
            if let Some(ref p) = payload.prompt {
                let preview = truncate_str(p, 120);
                new_state.prompt = Some(preview.clone());
                new_state.prompt_preview = Some(preview);
            }
            new_state.started_at_camel = Some(now);
            new_state.started_at_snake = Some(now);
        }

        HookEvent::PreToolUse => {
            let tname = payload.tool_name.clone().unwrap_or_else(|| "tool".to_string());
            let label = tool_label(&tname);
            new_state.state = "tool".to_string();
            new_state.mood = Some("working".to_string());
            new_state.label = Some(label.to_string());
            new_state.tool = Some(tname);
            if new_state.started_at_camel.is_none() || new_state.started_at_camel == Some(0) {
                new_state.started_at_camel = Some(now);
                new_state.started_at_snake = Some(now);
            }
        }

        HookEvent::PostToolUse => {
            new_state.state = "thinking".to_string();
            new_state.mood = Some("thinking".to_string());
            new_state.label = Some("Thinking…".to_string());
        }

        HookEvent::PermissionRequest => {
            new_state.state = "permission".to_string();
            new_state.mood = Some("alert".to_string());
            new_state.label = Some("Awaiting permission".to_string());
            new_state.started_at_camel = Some(0);
            new_state.started_at_snake = Some(0);
        }

        HookEvent::Notification => {
            let notif_type = payload.notification_type.as_deref().unwrap_or("");
            let msg = payload.message.as_deref().unwrap_or("").to_lowercase();

            if notif_type.contains("permission") || msg.contains("permission") {
                new_state.state = "permission".to_string();
                new_state.mood = Some("alert".to_string());
                new_state.label = Some("Awaiting permission".to_string());
                new_state.started_at_camel = Some(0);
                new_state.started_at_snake = Some(0);
            } else {
                // Non-permission notifications do not override active thinking/working
                if prev_state.state != "tool" && prev_state.state != "thinking" {
                    new_state.state = "alert".to_string();
                    new_state.mood = Some("alert".to_string());
                    new_state.label = Some("Notification".to_string());
                } else {
                    new_state.state = prev_state.state;
                    new_state.mood = prev_state.mood;
                    new_state.label = prev_state.label;
                    new_state.tool = prev_state.tool;
                }
            }
        }

        HookEvent::Stop => {
            new_state.state = "done".to_string();
            new_state.mood = Some("happy".to_string());
            new_state.label = Some("Done".to_string());
            new_state.started_at_camel = Some(0);
            new_state.started_at_snake = Some(0);
        }

        HookEvent::SessionStart => {
            new_state.state = "idle".to_string();
            new_state.mood = Some("neutral".to_string());
            new_state.label = Some("Ready".to_string());
            new_state.started_at_camel = Some(0);
            new_state.started_at_snake = Some(0);
        }

        HookEvent::SessionEnd => {
            new_state.state = "idle".to_string();
            new_state.mood = Some("neutral".to_string());
            new_state.label = Some("Idle".to_string());
            new_state.started_at_camel = Some(0);
            new_state.started_at_snake = Some(0);
        }
    }

    // Atomic write to %USERPROFILE%/.sidecrab/state.<pid>.tmp then rename
    let s_dir = sidecrab_dir();
    let _ = fs::create_dir_all(&s_dir);
    let tmp_file = s_dir.join(format!("state.{}.tmp", std::process::id()));

    if let Ok(json_str) = serde_json::to_string_pretty(&new_state) {
        if fs::write(&tmp_file, json_str).is_ok() {
            let _ = fs::rename(&tmp_file, &state_file);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hook_event_state_lifecycle() {
        let temp_dir = std::env::temp_dir().join(format!("sidecrab_test_sess_{}", std::process::id()));
        let _ = fs::create_dir_all(&temp_dir);
        std::env::set_var("SIDECRAB_HOME", &temp_dir);

        let sid = "session-test-uuid-42".to_string();

        // 1. SessionStart
        let start_payload = RawHookPayload {
            session_id: Some(sid.clone()),
            hook_event_name: Some("SessionStart".to_string()),
            cwd: Some("A:\\CODE\\test".to_string()),
            ..Default::default()
        };
        handle_hook_event(HookEvent::SessionStart, &start_payload);

        let sess_file = sessions_dir().join(format!("{}.json", sid));
        assert!(sess_file.exists(), "Session marker in sessions.d must exist");

        let state_data = fs::read_to_string(state_path()).expect("read state.json");
        let state: StateJson = serde_json::from_str(&state_data).expect("parse state.json");
        assert_eq!(state.state, "idle");
        assert_eq!(state.session_id(), sid);

        // 2. UserPromptSubmit
        let prompt_payload = RawHookPayload {
            session_id: Some(sid.clone()),
            prompt: Some("Create a desktop mascot in pure Rust".to_string()),
            ..Default::default()
        };
        handle_hook_event(HookEvent::UserPromptSubmit, &prompt_payload);

        let state_data = fs::read_to_string(state_path()).unwrap();
        let state: StateJson = serde_json::from_str(&state_data).unwrap();
        assert_eq!(state.state, "thinking");
        assert_eq!(state.mood, Some("thinking".to_string()));
        assert!(state.started_at() > 0);
        assert!(state.prompt_text().contains("Create a desktop mascot"));

        // 3. PreToolUse
        let pre_payload = RawHookPayload {
            session_id: Some(sid.clone()),
            tool_name: Some("Bash".to_string()),
            ..Default::default()
        };
        handle_hook_event(HookEvent::PreToolUse, &pre_payload);

        let state_data = fs::read_to_string(state_path()).unwrap();
        let state: StateJson = serde_json::from_str(&state_data).unwrap();
        assert_eq!(state.state, "tool");
        assert_eq!(state.mood, Some("working".to_string()));
        assert_eq!(state.label, Some("Running command".to_string()));
        assert_eq!(state.tool, Some("Bash".to_string()));

        // 4. PermissionRequest
        let perm_payload = RawHookPayload {
            session_id: Some(sid.clone()),
            tool_name: Some("Bash".to_string()),
            ..Default::default()
        };
        handle_hook_event(HookEvent::PermissionRequest, &perm_payload);

        let state_data = fs::read_to_string(state_path()).unwrap();
        let state: StateJson = serde_json::from_str(&state_data).unwrap();
        assert_eq!(state.state, "permission");
        assert_eq!(state.mood, Some("alert".to_string()));
        assert_eq!(state.label, Some("Awaiting permission".to_string()));
        assert_eq!(state.started_at(), 0);

        // 5. Stop
        let stop_payload = RawHookPayload {
            session_id: Some(sid.clone()),
            ..Default::default()
        };
        handle_hook_event(HookEvent::Stop, &stop_payload);

        let state_data = fs::read_to_string(state_path()).unwrap();
        let state: StateJson = serde_json::from_str(&state_data).unwrap();
        assert_eq!(state.state, "done");
        assert_eq!(state.mood, Some("happy".to_string()));
        assert_eq!(state.label, Some("Done".to_string()));

        // 6. SessionEnd
        let end_payload = RawHookPayload {
            session_id: Some(sid.clone()),
            ..Default::default()
        };
        handle_hook_event(HookEvent::SessionEnd, &end_payload);

        assert!(!sess_file.exists(), "Session marker should be removed on SessionEnd");
        let state_data = fs::read_to_string(state_path()).unwrap();
        let state: StateJson = serde_json::from_str(&state_data).unwrap();
        assert_eq!(state.state, "idle");

        // Cleanup
        std::env::remove_var("SIDECRAB_HOME");
        let _ = fs::remove_dir_all(&temp_dir);
    }
}

