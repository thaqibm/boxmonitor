use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use color_eyre::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub targets: Vec<Target>,
    pub ping_interval_ms: u64,
    pub ssh_timeout_ms: u64,
    pub history_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Target {
    pub ip: String,
    pub name: Option<String>,
    pub ssh_port: Option<u16>,
    pub ssh_user: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            targets: vec![
                Target {
                    ip: "8.8.8.8".to_string(),
                    name: Some("Google DNS".to_string()),
                    ssh_port: None,
                    ssh_user: None,
                },
                Target {
                    ip: "1.1.1.1".to_string(),
                    name: Some("Cloudflare DNS".to_string()),
                    ssh_port: None,
                    ssh_user: None,
                },
            ],
            ping_interval_ms: 1000,
            ssh_timeout_ms: 5000,
            history_size: 100,
        }
    }
}

pub fn get_config_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| color_eyre::eyre::eyre!("Could not find home directory"))?;
    let config_dir = home.join(".config").join("box");
    Ok(config_dir)
}

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

pub fn save_config(config: &Config) -> Result<()> {
    let config_dir = get_config_dir()?;
    fs::create_dir_all(&config_dir)?;
    
    let config_file = config_dir.join(".iplist");
    let content = serde_json::to_string_pretty(config)?;
    fs::write(config_file, content)?;
    Ok(())
}

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
            let ip = parts[0].to_string();
            let name = if parts.len() > 1 {
                Some(parts[1..].join(" "))
            } else {
                None
            };
            
            Target {
                ip,
                name,
                ssh_port: None,
                ssh_user: None,
            }
        })
        .collect();
    
    Ok(targets)
}

pub fn parse_targets_from_args(ip_list: Option<String>, ssh_list: Option<String>) -> Result<Vec<Target>> {
    let mut targets = Vec::new();
    
    if let Some(ips) = ip_list {
        for ip in ips.split(',') {
            let ip = ip.trim().to_string();
            if !ip.is_empty() {
                targets.push(Target {
                    ip,
                    name: None,
                    ssh_port: None,
                    ssh_user: None,
                });
            }
        }
    }
    
    if let Some(ssh_targets) = ssh_list {
        for ssh_target in ssh_targets.split(',') {
            let ssh_target = ssh_target.trim();
            if !ssh_target.is_empty() {
                let (user, ip_port) = if let Some(pos) = ssh_target.find('@') {
                    (&ssh_target[..pos], &ssh_target[pos + 1..])
                } else {
                    return Err(color_eyre::eyre::eyre!("Invalid SSH format: {}. Expected USER@ip[:port]", ssh_target));
                };
                
                let (ip, port) = if let Some(pos) = ip_port.find(':') {
                    let ip = &ip_port[..pos];
                    let port_str = &ip_port[pos + 1..];
                    let port = port_str.parse::<u16>()
                        .map_err(|_| color_eyre::eyre::eyre!("Invalid port number: {}", port_str))?;
                    (ip.to_string(), Some(port))
                } else {
                    (ip_port.to_string(), Some(22))
                };
                
                targets.push(Target {
                    ip,
                    name: Some(format!("{}@{}", user, ssh_target)),
                    ssh_port: port,
                    ssh_user: Some(user.to_string()),
                });
            }
        }
    }
    
    Ok(targets)
}