use serde::{Deserialize, Serialize};

/// Visual animation states for the Sidecrab mascot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MascotState {
    Idle,
    Thinking,
    Tool,
    Working,
    Permission,
    Alert,
    Done,
    Celebrate,
    Sleep,
    Wakeup,
}

impl MascotState {
    pub fn as_str(&self) -> &'static str {
        match self {
            MascotState::Idle => "idle",
            MascotState::Thinking => "thinking",
            MascotState::Tool => "tool",
            MascotState::Working => "working",
            MascotState::Permission => "permission",
            MascotState::Alert => "alert",
            MascotState::Done => "done",
            MascotState::Celebrate => "celebrate",
            MascotState::Sleep => "sleep",
            MascotState::Wakeup => "wakeup",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().trim() {
            "thinking" => MascotState::Thinking,
            "tool" => MascotState::Tool,
            "working" => MascotState::Working,
            "permission" => MascotState::Permission,
            "alert" => MascotState::Alert,
            "done" => MascotState::Done,
            "celebrate" => MascotState::Celebrate,
            "sleep" => MascotState::Sleep,
            "wakeup" => MascotState::Wakeup,
            _ => MascotState::Idle,
        }
    }
}

impl Default for MascotState {
    fn default() -> Self {
        MascotState::Idle
    }
}

/// Mood state indicator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MascotMood {
    Neutral,
    Happy,
    Focused,
    Excited,
    Curious,
    Alert,
    Working,
    Thinking,
    Sleepy,
}

impl MascotMood {
    pub fn as_str(&self) -> &'static str {
        match self {
            MascotMood::Neutral => "neutral",
            MascotMood::Happy => "happy",
            MascotMood::Focused => "focused",
            MascotMood::Excited => "excited",
            MascotMood::Curious => "curious",
            MascotMood::Alert => "alert",
            MascotMood::Working => "working",
            MascotMood::Thinking => "thinking",
            MascotMood::Sleepy => "sleepy",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().trim() {
            "happy" => MascotMood::Happy,
            "focused" => MascotMood::Focused,
            "excited" => MascotMood::Excited,
            "curious" => MascotMood::Curious,
            "alert" => MascotMood::Alert,
            "working" => MascotMood::Working,
            "thinking" => MascotMood::Thinking,
            "sleepy" => MascotMood::Sleepy,
            _ => MascotMood::Neutral,
        }
    }
}

impl Default for MascotMood {
    fn default() -> Self {
        MascotMood::Neutral
    }
}

/// Available hat accessories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HatType {
    None,
    Top,
    Chef,
    Fedora,
    Helicopter,
}

impl HatType {
    pub fn as_str(&self) -> &'static str {
        match self {
            HatType::None => "none",
            HatType::Top => "top",
            HatType::Chef => "chef",
            HatType::Fedora => "fedora",
            HatType::Helicopter => "heli",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().trim() {
            "top" | "tophat" | "top_hat" => HatType::Top,
            "chef" | "chefhat" | "chef_hat" => HatType::Chef,
            "fedora" => HatType::Fedora,
            "heli" | "helicopter" => HatType::Helicopter,
            _ => HatType::None,
        }
    }
}

impl Default for HatType {
    fn default() -> Self {
        HatType::None
    }
}

/// Window dimensions and integer nearest-neighbor scaling.
/// Logical canvas is 51x48 (1.0625:1 aspect ratio).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WindowSize {
    Small,  // 2x: 102x96
    Medium, // 3x: 153x144
    Large,  // 4x: 204x192
}

impl WindowSize {
    pub const BASE_WIDTH: i32 = 51;
    pub const BASE_HEIGHT: i32 = 48;
    pub const CRAB_Y: i32 = 12;

    pub fn scale(&self) -> u32 {
        match self {
            WindowSize::Small => 2,
            WindowSize::Medium => 3,
            WindowSize::Large => 4,
        }
    }

    pub fn width(&self) -> i32 {
        Self::BASE_WIDTH * (self.scale() as i32)
    }

    pub fn height(&self) -> i32 {
        Self::BASE_HEIGHT * (self.scale() as i32)
    }

    pub fn from_scale(scale: u32) -> Self {
        match scale {
            2 => WindowSize::Small,
            4 => WindowSize::Large,
            _ => WindowSize::Medium,
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().trim() {
            "small" | "s" => WindowSize::Small,
            "large" | "l" => WindowSize::Large,
            _ => WindowSize::Medium,
        }
    }
}

impl Default for WindowSize {
    fn default() -> Self {
        WindowSize::Medium
    }
}

/// Persistent mascot configuration stored in `%USERPROFILE%/.sidecrab/config.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PetConfig {
    pub x: i32,
    pub y: i32,
    pub size: WindowSize,
    pub hat: HatType,
    pub wander_enabled: bool,
    pub autostart: bool,
}

impl Default for PetConfig {
    fn default() -> Self {
        Self {
            x: 100,
            y: 100,
            size: WindowSize::Medium,
            hat: HatType::None,
            wander_enabled: true,
            autostart: false,
        }
    }
}

/// JSON payload stored in `%USERPROFILE%/.sidecrab/state.json`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StateJson {
    #[serde(default)]
    pub version: Option<u32>,
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub mood: Option<String>,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub tool: Option<String>,
    #[serde(default)]
    pub prompt: Option<String>,
    #[serde(default)]
    pub prompt_preview: Option<String>,
    #[serde(default)]
    pub project: Option<String>,
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(default, rename = "sessionId")]
    pub session_id_camel: Option<String>,
    #[serde(default, rename = "session_id")]
    pub session_id_snake: Option<String>,
    #[serde(default, rename = "active_session_id")]
    pub active_session_id: Option<String>,
    #[serde(default)]
    pub transcript: Option<String>,
    #[serde(default)]
    pub host: Option<String>,
    #[serde(default)]
    pub host_hwnd: Option<u64>,
    #[serde(default)]
    pub host_pid: Option<u32>,
    #[serde(default, rename = "startedAt")]
    pub started_at_camel: Option<u64>,
    #[serde(default, rename = "started_at")]
    pub started_at_snake: Option<u64>,
    #[serde(default)]
    pub ts: Option<u64>,
    #[serde(default)]
    pub timestamp: Option<u64>,
}

impl StateJson {
    pub fn session_id(&self) -> &str {
        if let Some(ref id) = self.session_id_camel {
            if !id.is_empty() {
                return id;
            }
        }
        if let Some(ref id) = self.session_id_snake {
            if !id.is_empty() {
                return id;
            }
        }
        if let Some(ref id) = self.active_session_id {
            if !id.is_empty() {
                return id;
            }
        }
        ""
    }

    pub fn prompt_text(&self) -> &str {
        if let Some(ref p) = self.prompt {
            return p;
        }
        if let Some(ref p) = self.prompt_preview {
            return p;
        }
        ""
    }

    pub fn mascot_state(&self) -> MascotState {
        MascotState::from_str(&self.state)
    }

    pub fn mascot_mood(&self) -> MascotMood {
        if let Some(ref m) = self.mood {
            MascotMood::from_str(m)
        } else {
            MascotMood::Neutral
        }
    }

    pub fn started_at(&self) -> u64 {
        self.started_at_camel
            .or(self.started_at_snake)
            .unwrap_or(0)
    }

    pub fn timestamp(&self) -> u64 {
        self.ts
            .or(self.timestamp)
            .unwrap_or(0)
    }
}
