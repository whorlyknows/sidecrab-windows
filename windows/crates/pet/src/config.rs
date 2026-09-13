use common::{config_path, sidecrab_dir, PetConfig};
use std::fs;

/// Loads pet configuration from disk, creating default if not found.
pub fn load_config() -> PetConfig {
    let path = config_path();
    if let Ok(data) = fs::read_to_string(&path) {
        if let Ok(cfg) = serde_json::from_str::<PetConfig>(&data) {
            return cfg;
        }
    }
    let default = PetConfig::default();
    save_config(&default);
    default
}

/// Saves pet configuration to disk atomically.
pub fn save_config(cfg: &PetConfig) {
    let dir = sidecrab_dir();
    let _ = fs::create_dir_all(&dir);
    let path = config_path();
    let tmp_path = dir.join(format!("config.json.{}.tmp", std::process::id()));

    if let Ok(json) = serde_json::to_string_pretty(cfg) {
        if fs::write(&tmp_path, json).is_ok() {
            let _ = fs::rename(&tmp_path, &path);
        }
    }
}
