use serde::{Deserialize, Serialize};

/// Claude Code lifecycle hook events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HookEvent {
    UserPromptSubmit,
    PreToolUse,
    PostToolUse,
    Notification,
    PermissionRequest,
    Stop,
    SessionStart,
    SessionEnd,
}

impl HookEvent {
    pub fn as_str(&self) -> &'static str {
        match self {
            HookEvent::UserPromptSubmit => "UserPromptSubmit",
            HookEvent::PreToolUse => "PreToolUse",
            HookEvent::PostToolUse => "PostToolUse",
            HookEvent::Notification => "Notification",
            HookEvent::PermissionRequest => "PermissionRequest",
            HookEvent::Stop => "Stop",
            HookEvent::SessionStart => "SessionStart",
            HookEvent::SessionEnd => "SessionEnd",
        }
    }

    pub fn short_name(&self) -> &'static str {
        match self {
            HookEvent::UserPromptSubmit => "prompt",
            HookEvent::PreToolUse => "pre",
            HookEvent::PostToolUse => "post",
            HookEvent::Notification => "notify",
            HookEvent::PermissionRequest => "permreq",
            HookEvent::Stop => "stop",
            HookEvent::SessionStart => "start",
            HookEvent::SessionEnd => "end",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().as_str() {
            "prompt" | "userpromptsubmit" => Some(HookEvent::UserPromptSubmit),
            "pre" | "pretooluse" => Some(HookEvent::PreToolUse),
            "post" | "posttooluse" => Some(HookEvent::PostToolUse),
            "notify" | "notification" => Some(HookEvent::Notification),
            "permreq" | "permissionrequest" => Some(HookEvent::PermissionRequest),
            "stop" => Some(HookEvent::Stop),
            "start" | "sessionstart" => Some(HookEvent::SessionStart),
            "end" | "sessionend" => Some(HookEvent::SessionEnd),
            _ => None,
        }
    }

    pub const ALL: [HookEvent; 8] = [
        HookEvent::UserPromptSubmit,
        HookEvent::PreToolUse,
        HookEvent::PostToolUse,
        HookEvent::Notification,
        HookEvent::PermissionRequest,
        HookEvent::Stop,
        HookEvent::SessionStart,
        HookEvent::SessionEnd,
    ];
}

/// Raw payload fed by Claude Code into hook's stdin.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RawHookPayload {
    #[serde(default)]
    pub hook_event_name: Option<String>,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub prompt: Option<String>,
    #[serde(default)]
    pub session_title: Option<String>,
    #[serde(default)]
    pub tool_name: Option<String>,
    #[serde(default)]
    pub tool_input: Option<serde_json::Value>,
    #[serde(default)]
    pub tool_response: Option<serde_json::Value>,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub notification_type: Option<String>,
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(default)]
    pub transcript_path: Option<String>,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
}

/// Resolves a tool name into a human-readable activity description.
pub fn tool_label(tool_name: &str) -> &'static str {
    match tool_name {
        "Bash" => "Running command",
        "Edit" | "MultiEdit" | "NotebookEdit" => "Editing",
        "Write" => "Writing",
        "Read" => "Reading",
        "Grep" | "Glob" => "Searching",
        "WebFetch" => "Browsing web",
        "WebSearch" => "Searching web",
        "Task" => "Delegating",
        "TodoWrite" => "Planning",
        _ => "Using tool",
    }
}
