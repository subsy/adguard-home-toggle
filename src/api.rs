use crate::config::Config;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct Status {
    pub protection_enabled: bool,
    #[serde(default)]
    pub protection_disabled_duration: u64,
}

#[derive(Serialize)]
struct ProtectionRequest {
    enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    duration: Option<u64>,
}

pub struct AdGuardClient {
    client: Client,
    base_url: String,
    username: String,
    password: String,
}

impl AdGuardClient {
    pub fn new(config: &Config) -> Self {
        Self {
            client: Client::new(),
            base_url: config.server_url.trim_end_matches('/').to_string(),
            username: config.username.clone(),
            password: config.password.clone(),
        }
    }

    pub fn get_status(&self) -> Result<Status, String> {
        let resp = self.client
            .get(format!("{}/control/status", self.base_url))
            .basic_auth(&self.username, Some(&self.password))
            .send()
            .map_err(|e| format!("Request failed: {e}"))?;

        if !resp.status().is_success() {
            return Err(format!("API returned {}", resp.status()));
        }

        resp.json::<Status>().map_err(|e| format!("Failed to parse response: {e}"))
    }

    pub fn set_protection(&self, enabled: bool) -> Result<(), String> {
        let body = ProtectionRequest { enabled, duration: None };
        self.post_protection(&body)
    }

    pub fn snooze(&self, duration_ms: u64) -> Result<(), String> {
        let body = ProtectionRequest {
            enabled: false,
            duration: Some(duration_ms),
        };
        self.post_protection(&body)
    }

    fn post_protection(&self, body: &ProtectionRequest) -> Result<(), String> {
        let resp = self.client
            .post(format!("{}/control/protection", self.base_url))
            .basic_auth(&self.username, Some(&self.password))
            .json(body)
            .send()
            .map_err(|e| format!("Request failed: {e}"))?;

        if !resp.status().is_success() {
            return Err(format!("API returned {}", resp.status()));
        }
        Ok(())
    }
}
