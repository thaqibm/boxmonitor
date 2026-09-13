#[cfg(not(target_arch = "wasm32"))]
use color_eyre::Result;
use serde::{Deserialize, Serialize};
#[cfg(not(target_arch = "wasm32"))]
use std::fs;
#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub targets: Vec<Target>,
    pub ping_interval_ms: u64,
    pub history_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Target {
    #[serde(alias = "ip", alias = "host", alias = "domain")]
    pub address: String,
    pub name: Option<String>,
}

#[cfg(not(target_arch = "wasm32"))]
impl Default for Config {
    fn default() -> Self {
        Self {
            targets: vec![
                Target {
                    address: "8.8.8.8".to_string(),
                    name: Some("Google DNS".to_string()),
                },
                Target {
                    address: "1.1.1.1".to_string(),
                    name: Some("Cloudflare DNS".to_string()),
                },
            ],
            ping_interval_ms: 1000,
            history_size: 100,
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn get_config_dir() -> Result<PathBuf> {
    let home =
        dirs::home_dir().ok_or_else(|| color_eyre::eyre::eyre!("Could not find home directory"))?;
    let config_dir = home.join(".config").join("box");
    Ok(config_dir)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load_config() -> Result<Config> {
    let config_dir = get_config_dir()?;
    let config_file = config_dir.join(".iplist");

    if !config_file.exists() {
        let default_config = Config::default();
        save_config(&default_config)?;
        return Ok(default_config);
    }

    let content = fs::read_to_string(&config_file)?;
    let config: Config = serde_json::from_str(&content)?;
    Ok(config)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn save_config(config: &Config) -> Result<()> {
    let config_dir = get_config_dir()?;
    fs::create_dir_all(&config_dir)?;

    let config_file = config_dir.join(".iplist");
    let content = serde_json::to_string_pretty(config)?;
    fs::write(config_file, content)?;
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load_targets_from_simple_list() -> Result<Vec<Target>> {
    let config_dir = get_config_dir()?;
    let iplist_file = config_dir.join(".iplist");

    if !iplist_file.exists() {
        return Ok(Config::default().targets);
    }

    let content = fs::read_to_string(&iplist_file)?;

    if content.trim().starts_with('{') {
        let config: Config = serde_json::from_str(&content)?;
        return Ok(config.targets);
    }

    let targets = content
        .lines()
        .filter(|line| !line.trim().is_empty() && !line.trim().starts_with('#'))
        .map(|line| {
            let parts: Vec<&str> = line.split_whitespace().collect();
            let address = parts[0].to_string();
            let name = if parts.len() > 1 {
                Some(parts[1..].join(" "))
            } else {
                None
            };

            Target { address, name }
        })
        .collect();

    Ok(targets)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn parse_targets_from_args(ip_list: Option<String>) -> Result<Vec<Target>> {
    let mut targets = Vec::new();

    if let Some(ips) = ip_list {
        for ip in ips.split(',') {
            let address = ip.trim().to_string();
            if !address.is_empty() {
                targets.push(Target {
                    address,
                    name: None,
                });
            }
        }
    }

    Ok(targets)
}
