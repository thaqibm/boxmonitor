use crate::config::Target;
use chrono::{DateTime, Utc};
use color_eyre::Result;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PingResult {
    pub timestamp: DateTime<Utc>,
    pub latency_ms: Option<f64>,
    pub success: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshResult {
    pub timestamp: DateTime<Utc>,
    pub connection_time_ms: Option<f64>,
    pub success: bool,
}

#[derive(Debug, Clone)]
pub struct TargetStats {
    pub target: Target,
    pub ping_history: VecDeque<PingResult>,
    pub ssh_history: VecDeque<SshResult>,
    pub ping_stats: Option<Statistics>,
    pub ssh_stats: Option<Statistics>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Statistics {
    pub mean: f64,
    pub median: f64,
    pub min: f64,
    pub max: f64,
    pub p25: f64,
    pub p75: f64,
    pub p90: f64,
    pub p95: f64,
    pub p99: f64,
    pub success_rate: f64,
    pub total_count: usize,
}

impl TargetStats {
    pub fn new(target: Target, history_size: usize) -> Self {
        Self {
            target,
            ping_history: VecDeque::with_capacity(history_size),
            ssh_history: VecDeque::with_capacity(history_size),
            ping_stats: None,
            ssh_stats: None,
        }
    }

    pub fn add_ping_result(&mut self, result: PingResult, max_history: usize) {
        if self.ping_history.len() >= max_history {
            self.ping_history.pop_front();
        }
        self.ping_history.push_back(result);
        self.update_ping_stats();
    }

    pub fn add_ssh_result(&mut self, result: SshResult, max_history: usize) {
        if self.ssh_history.len() >= max_history {
            self.ssh_history.pop_front();
        }
        self.ssh_history.push_back(result);
        self.update_ssh_stats();
    }

    fn update_ping_stats(&mut self) {
        let successful_pings: Vec<f64> = self
            .ping_history
            .iter()
            .filter_map(|r| r.latency_ms)
            .collect();

        if !successful_pings.is_empty() {
            self.ping_stats = Some(calculate_statistics(&successful_pings, self.ping_history.len()));
        }
    }

    fn update_ssh_stats(&mut self) {
        let successful_ssh: Vec<f64> = self
            .ssh_history
            .iter()
            .filter_map(|r| r.connection_time_ms)
            .collect();

        if !successful_ssh.is_empty() {
            self.ssh_stats = Some(calculate_statistics(&successful_ssh, self.ssh_history.len()));
        }
    }
}

pub struct Monitor {
    targets: Vec<TargetStats>,
    ping_interval: Duration,
    ssh_timeout: Duration,
    history_size: usize,
}

impl Monitor {
    pub fn new(
        targets: Vec<Target>,
        ping_interval_ms: u64,
        ssh_timeout_ms: u64,
        history_size: usize,
    ) -> Self {
        let target_stats = targets
            .into_iter()
            .map(|target| TargetStats::new(target, history_size))
            .collect();

        Self {
            targets: target_stats,
            ping_interval: Duration::from_millis(ping_interval_ms),
            ssh_timeout: Duration::from_millis(ssh_timeout_ms),
            history_size,
        }
    }

    pub fn get_targets(&self) -> &[TargetStats] {
        &self.targets
    }

    pub async fn run_ping_cycle(&mut self) -> Result<()> {
        let mut handles = Vec::new();

        for (index, target_stats) in self.targets.iter().enumerate() {
            let ip = target_stats.target.ip.clone();
            let handle = tokio::spawn(async move {
                (index, ping_target(&ip).await)
            });
            handles.push(handle);
        }

        for handle in handles {
            if let Ok((index, result)) = handle.await {
                if let Some(target_stats) = self.targets.get_mut(index) {
                    target_stats.add_ping_result(result, self.history_size);
                }
            }
        }

        Ok(())
    }

    pub async fn run_ssh_cycle(&mut self) -> Result<()> {
        let mut handles = Vec::new();

        for (index, target_stats) in self.targets.iter().enumerate() {
            if target_stats.target.ssh_port.is_some() && target_stats.target.ssh_user.is_some() {
                let ip = target_stats.target.ip.clone();
                let port = target_stats.target.ssh_port.unwrap_or(22);
                let user = target_stats.target.ssh_user.clone().unwrap();
                let timeout = self.ssh_timeout;

                let handle = tokio::spawn(async move {
                    (index, ssh_test(&ip, port, &user, timeout).await)
                });
                handles.push(handle);
            }
        }

        for handle in handles {
            if let Ok((index, result)) = handle.await {
                if let Some(target_stats) = self.targets.get_mut(index) {
                    target_stats.add_ssh_result(result, self.history_size);
                }
            }
        }

        Ok(())
    }

    pub async fn start_monitoring(&mut self) -> Result<()> {
        let mut ping_interval = tokio::time::interval(self.ping_interval);
        let mut ssh_interval = tokio::time::interval(self.ping_interval * 5);

        loop {
            tokio::select! {
                _ = ping_interval.tick() => {
                    if let Err(e) = self.run_ping_cycle().await {
                        eprintln!("Ping cycle error: {}", e);
                    }
                }
                _ = ssh_interval.tick() => {
                    if let Err(e) = self.run_ssh_cycle().await {
                        eprintln!("SSH cycle error: {}", e);
                    }
                }
            }
        }
    }
}

async fn ping_target(ip: &str) -> PingResult {
    let timestamp = Utc::now();
    
    let addr = match ip.parse::<std::net::IpAddr>() {
        Ok(addr) => addr,
        Err(_) => {
            return PingResult {
                timestamp,
                latency_ms: None,
                success: false,
            };
        }
    };

    let config = surge_ping::Config::default();
    let client = match surge_ping::Client::new(&config) {
        Ok(client) => client,
        Err(_) => {
            return PingResult {
                timestamp,
                latency_ms: None,
                success: false,
            };
        }
    };

    let mut pinger = client.pinger(addr, surge_ping::PingIdentifier(0)).await;
    let start = Instant::now();
    
    match pinger.ping(surge_ping::PingSequence(0), &[]).await {
        Ok(_) => {
            let latency = start.elapsed().as_millis() as f64;
            PingResult {
                timestamp,
                latency_ms: Some(latency),
                success: true,
            }
        }
        Err(_) => PingResult {
            timestamp,
            latency_ms: None,
            success: false,
        },
    }
}

async fn ssh_test(ip: &str, port: u16, _user: &str, timeout: Duration) -> SshResult {
    let start = Instant::now();
    let timestamp = Utc::now();

    let result = tokio::time::timeout(timeout, async {
        let tcp = std::net::TcpStream::connect(format!("{}:{}", ip, port));
        match tcp {
            Ok(stream) => {
                let mut session = ssh2::Session::new().unwrap();
                session.set_tcp_stream(stream);
                match session.handshake() {
                    Ok(_) => true,
                    Err(_) => false,
                }
            }
            Err(_) => false,
        }
    }).await;

    match result {
        Ok(true) => {
            let connection_time = start.elapsed().as_millis() as f64;
            SshResult {
                timestamp,
                connection_time_ms: Some(connection_time),
                success: true,
            }
        }
        _ => SshResult {
            timestamp,
            connection_time_ms: None,
            success: false,
        },
    }
}

fn calculate_statistics(values: &[f64], total_count: usize) -> Statistics {
    let mut sorted_values = values.to_vec();
    sorted_values.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let median = percentile(&sorted_values, 50.0);
    let min = *sorted_values.first().unwrap_or(&0.0);
    let max = *sorted_values.last().unwrap_or(&0.0);
    let success_rate = (values.len() as f64 / total_count as f64) * 100.0;

    Statistics {
        mean,
        median,
        min,
        max,
        p25: percentile(&sorted_values, 25.0),
        p75: percentile(&sorted_values, 75.0),
        p90: percentile(&sorted_values, 90.0),
        p95: percentile(&sorted_values, 95.0),
        p99: percentile(&sorted_values, 99.0),
        success_rate,
        total_count,
    }
}

fn percentile(sorted_values: &[f64], p: f64) -> f64 {
    if sorted_values.is_empty() {
        return 0.0;
    }
    
    if sorted_values.len() == 1 {
        return sorted_values[0];
    }

    let index = (p / 100.0) * (sorted_values.len() - 1) as f64;
    let lower = index.floor() as usize;
    let upper = index.ceil() as usize;

    if lower == upper {
        sorted_values[lower]
    } else {
        let weight = index - lower as f64;
        sorted_values[lower] * (1.0 - weight) + sorted_values[upper] * weight
    }
}
