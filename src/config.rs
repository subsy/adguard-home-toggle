use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[derive(Deserialize)]
pub struct Config {
    pub server_url: String,
    pub username: String,
    pub password: String,
}

impl Config {
    pub fn load() -> Result<Self, String> {
        let path = Self::path();
        let contents = fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read config at {}: {e}", path.display()))?;
        toml::from_str(&contents)
            .map_err(|e| format!("Failed to parse config: {e}"))
    }

    pub fn path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("~/.config"))
            .join("adguard-home-toggle")
            .join("config.toml")
    }
}
