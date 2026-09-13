pub mod palette;
pub mod paths;
pub mod protocol;
pub mod state;

pub use palette::*;
pub use paths::*;
pub use protocol::*;
pub use state::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_palette_bgra_values() {
        assert_eq!(PALETTE_BGRA[0], [0, 0, 0, 0]); // Transparent
        assert_eq!(PALETTE_BGRA[1], [0, 0, 0, 255]); // Black
        assert_eq!(PALETTE_BGRA[6], [87, 119, 217, 255]); // Coral (#d97757 in BGRA)
        assert_eq!(COLOR_BUBBLE_BGRA, [220, 231, 242, 255]);
        assert_eq!(COLOR_TYPING_DOTS_BGRA, [116, 107, 107, 255]);
        assert_eq!(COLOR_HEART_BGRA, [119, 59, 255, 255]);

        assert_eq!(hat_color_bgra('k'), Some([34, 28, 29, 255]));
        assert_eq!(hat_color_bgra('w'), Some([233, 239, 242, 255]));
        assert_eq!(hat_color_bgra('r'), Some([45, 55, 200, 255]));
        assert_eq!(hat_color_bgra('?'), None);
    }

    #[test]
    fn test_window_sizes_and_scaling() {
        let s = WindowSize::Small;
        assert_eq!(s.scale(), 2);
        assert_eq!(s.width(), 102);
        assert_eq!(s.height(), 96);

        let m = WindowSize::Medium;
        assert_eq!(m.scale(), 3);
        assert_eq!(m.width(), 153);
        assert_eq!(m.height(), 144);

        let l = WindowSize::Large;
        assert_eq!(l.scale(), 4);
        assert_eq!(l.width(), 204);
        assert_eq!(l.height(), 192);

        assert_eq!(WindowSize::from_scale(2), WindowSize::Small);
        assert_eq!(WindowSize::from_scale(3), WindowSize::Medium);
        assert_eq!(WindowSize::from_scale(4), WindowSize::Large);
    }

    #[test]
    fn test_state_json_serialization() {
        let json_str = r#"{
            "version": 1,
            "state": "tool",
            "mood": "working",
            "label": "Running command",
            "tool": "Bash",
            "prompt": "cargo test",
            "sessionId": "test-session-123",
            "host": "WindowsTerminal.exe",
            "host_hwnd": 12345,
            "host_pid": 6789,
            "startedAt": 1740000000,
            "ts": 1740000005
        }"#;

        let state: StateJson = serde_json::from_str(json_str).expect("deserialize state");
        assert_eq!(state.mascot_state(), MascotState::Tool);
        assert_eq!(state.mascot_mood(), MascotMood::Working);
        assert_eq!(state.session_id(), "test-session-123");
        assert_eq!(state.prompt_text(), "cargo test");
        assert_eq!(state.host_hwnd, Some(12345));
        assert_eq!(state.host_pid, Some(6789));
        assert_eq!(state.started_at(), 1740000000);
        assert_eq!(state.timestamp(), 1740000005);
    }

    #[test]
    fn test_hook_event_parsing() {
        assert_eq!(HookEvent::from_str("prompt"), Some(HookEvent::UserPromptSubmit));
        assert_eq!(HookEvent::from_str("UserPromptSubmit"), Some(HookEvent::UserPromptSubmit));
        assert_eq!(HookEvent::from_str("pre"), Some(HookEvent::PreToolUse));
        assert_eq!(HookEvent::from_str("post"), Some(HookEvent::PostToolUse));
        assert_eq!(HookEvent::from_str("notify"), Some(HookEvent::Notification));
        assert_eq!(HookEvent::from_str("permreq"), Some(HookEvent::PermissionRequest));
        assert_eq!(HookEvent::from_str("stop"), Some(HookEvent::Stop));
        assert_eq!(HookEvent::from_str("start"), Some(HookEvent::SessionStart));
        assert_eq!(HookEvent::from_str("end"), Some(HookEvent::SessionEnd));
        assert_eq!(HookEvent::from_str("unknown"), None);
    }

    #[test]
    fn test_tool_label_mappings() {
        assert_eq!(tool_label("Bash"), "Running command");
        assert_eq!(tool_label("Edit"), "Editing");
        assert_eq!(tool_label("Write"), "Writing");
        assert_eq!(tool_label("Read"), "Reading");
        assert_eq!(tool_label("Grep"), "Searching");
        assert_eq!(tool_label("Glob"), "Searching");
        assert_eq!(tool_label("WebFetch"), "Browsing web");
        assert_eq!(tool_label("Task"), "Delegating");
        assert_eq!(tool_label("mcp__test"), "Using tool");
    }

    #[test]
    fn test_hat_types() {
        assert_eq!(HatType::from_str("none"), HatType::None);
        assert_eq!(HatType::from_str("top"), HatType::Top);
        assert_eq!(HatType::from_str("tophat"), HatType::Top);
        assert_eq!(HatType::from_str("chef"), HatType::Chef);
        assert_eq!(HatType::from_str("fedora"), HatType::Fedora);
        assert_eq!(HatType::from_str("heli"), HatType::Helicopter);
    }
}

