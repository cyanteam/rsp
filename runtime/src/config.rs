use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RspConfig {
    #[serde(default)]
    pub database: DatabaseConfig,
    #[serde(default)]
    pub dependencies: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DatabaseConfig {
    #[serde(default)]
    pub default: Option<String>,
    #[serde(default)]
    pub connections: HashMap<String, ConnectionConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionConfig {
    pub driver: String,
    pub url: String,
    #[serde(default = "default_max_connections")]
    pub max_connections: u32,
}

fn default_max_connections() -> u32 {
    5
}

impl RspConfig {
    pub fn load(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let config_path = path.join("rsp.toml");

        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)?;
            let config: RspConfig = toml::from_str(&content)?;
            return Ok(config);
        }

        let json_path = path.join("rsp.json");
        if json_path.exists() {
            let content = std::fs::read_to_string(&json_path)?;
            let config: RspConfig = serde_json::from_str(&content)?;
            return Ok(config);
        }

        Ok(RspConfig::default())
    }
}
