use crate::config::Target;
use chrono::{DateTime, Utc};
#[cfg(not(target_arch = "wasm32"))]
use color_eyre::Result;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
#[cfg(not(target_arch = "wasm32"))]
use std::hash::{Hash, Hasher};
use std::net::IpAddr;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PingResult {
    pub timestamp: DateTime<Utc>,
    pub latency_ms: Option<f64>,
    pub success: bool,
    pub failure_reason: Option<String>,
    pub resolved_ip: Option<IpAddr>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailureLog {
    pub timestamp: DateTime<Utc>,
    pub failure_type: String,
    pub reason: String,
}

#[derive(Debug, Clone)]
pub struct TargetStats {
    pub target: Target,
    pub ping_history: VecDeque<PingResult>,
    pub failure_log: VecDeque<FailureLog>,
    pub ping_stats: Option<Statistics>,
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
            failure_log: VecDeque::with_capacity(history_size),
            ping_stats: None,
        }
    }

    pub fn add_ping_result(&mut self, result: PingResult, max_history: usize) {
        if self.ping_history.len() >= max_history {
            self.ping_history.pop_front();
        }

        // Log failure if ping failed
        if !result.success {
            if let Some(failure_reason) = &result.failure_reason {
                self.add_failure_log("Ping".to_string(), failure_reason.clone(), max_history);
            }
        }

        self.ping_history.push_back(result);
        self.update_ping_stats();
    }

    pub fn add_failure_log(&mut self, failure_type: String, reason: String, max_history: usize) {
        if self.failure_log.len() >= max_history {
            self.failure_log.pop_front();
        }

        let failure_entry = FailureLog {
            timestamp: Utc::now(),
            failure_type,
            reason,
        };

        self.failure_log.push_back(failure_entry);
    }

    fn update_ping_stats(&mut self) {
        let successful_pings: Vec<f64> = self
            .ping_history
            .iter()
            .filter_map(|r| r.latency_ms)
            .collect();

        self.ping_stats = None;
        if !successful_pings.is_empty() {
            self.ping_stats = Some(calculate_statistics(
                &successful_pings,
                self.ping_history.len(),
            ));
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub struct Monitor {
    targets: Vec<TargetStats>,
    history_size: usize,
    debug_logging: bool,
}

#[cfg(not(target_arch = "wasm32"))]
impl Monitor {
    pub fn new(
        targets: Vec<Target>,
        _ping_interval_ms: u64,
        history_size: usize,
        debug_logging: bool,
    ) -> Self {
        let target_stats = targets
            .into_iter()
            .map(|target| TargetStats::new(target, history_size))
            .collect();

        Self {
            targets: target_stats,
            history_size,
            debug_logging,
        }
    }

    pub fn get_targets(&self) -> &[TargetStats] {
        &self.targets
    }

    pub async fn run_ping_cycle(&mut self) -> Result<()> {
        let mut handles = Vec::new();

        for (index, target_stats) in self.targets.iter().enumerate() {
            let address = target_stats.target.address.clone();
            let handle = tokio::spawn(async move { (index, ping_target(&address).await) });
            handles.push(handle);
        }

        for handle in handles {
            if let Ok((index, result)) = handle.await {
                if let Some(target_stats) = self.targets.get_mut(index) {
                    if self.debug_logging {
                        log_ping_result(&target_stats.target.address, &result);
                    }
                    target_stats.add_ping_result(result, self.history_size);
                }
            }
        }

        Ok(())
    }
}

#[cfg(not(target_arch = "wasm32"))]
async fn ping_target(host: &str) -> PingResult {
    let timestamp = Utc::now();

    let addr = match resolve_host_to_ip(host).await {
        Ok(addr) => addr,
        Err(e) => {
            return PingResult {
                timestamp,
                latency_ms: None,
                success: false,
                failure_reason: Some(e),
                resolved_ip: None,
            };
        }
    };

    let config = surge_ping::Config::default();
    let client = match surge_ping::Client::new(&config) {
        Ok(client) => client,
        Err(e) => {
            return PingResult {
                timestamp,
                latency_ms: None,
                success: false,
                failure_reason: Some(format!("Failed to create ping client: {}", e)),
                resolved_ip: Some(addr),
            };
        }
    };

    let identifier = surge_ping::PingIdentifier(unique_identifier(host));
    let mut pinger = client.pinger(addr, identifier).await;

    match pinger.ping(surge_ping::PingSequence(0), &[]).await {
        Ok((_, duration)) => {
            let latency = duration.as_millis() as f64;
            PingResult {
                timestamp,
                latency_ms: Some(latency),
                success: true,
                failure_reason: None,
                resolved_ip: Some(addr),
            }
        }
        Err(e) => PingResult {
            timestamp,
            latency_ms: None,
            success: false,
            failure_reason: Some(format!("Ping failed: {:#?}", e)),
            resolved_ip: Some(addr),
        },
    }
}

#[cfg(not(target_arch = "wasm32"))]
async fn resolve_host_to_ip(host: &str) -> Result<IpAddr, String> {
    if let Ok(addr) = host.parse::<IpAddr>() {
        return Ok(addr);
    }

    let lookup = tokio::net::lookup_host((host, 0))
        .await
        .map_err(|e| format!("DNS lookup failed for {}: {}", host, e))?;

    let mut first_addr: Option<IpAddr> = None;

    for socket_addr in lookup {
        let ip = socket_addr.ip();
        if ip.is_ipv4() {
            return Ok(ip);
        }

        if first_addr.is_none() {
            first_addr = Some(ip);
        }
    }

    first_addr.ok_or_else(|| format!("DNS lookup returned no addresses for {}", host))
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

#[cfg(not(target_arch = "wasm32"))]
fn log_ping_result(target: &str, result: &PingResult) {
    let resolved = result
        .resolved_ip
        .map(|ip| ip.to_string())
        .unwrap_or_else(|| "unresolved".to_string());

    if result.success {
        if let Some(latency) = result.latency_ms {
            eprintln!(
                "[debug] Ping success: target={} ip={} latency={:.2}ms timestamp={}",
                target, resolved, latency, result.timestamp
            );
        } else {
            eprintln!(
                "[debug] Ping reported success without latency: target={} ip={} timestamp={}",
                target, resolved, result.timestamp
            );
        }
    } else {
        let reason = result
            .failure_reason
            .as_deref()
            .unwrap_or("Unknown failure");
        eprintln!(
            "[debug] Ping failure: target={} ip={} reason={} timestamp={}",
            target, resolved, reason, result.timestamp
        );
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn unique_identifier(host: &str) -> u16 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    host.hash(&mut hasher);
    (hasher.finish() & 0xFFFF) as u16
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interpolated_percentiles_and_packet_loss() {
        let stats = calculate_statistics(&[10.0, 20.0, 30.0, 40.0], 5);
        assert_eq!(stats.mean, 25.0);
        assert_eq!(stats.median, 25.0);
        assert_eq!(stats.p25, 17.5);
        assert_eq!(stats.p95, 38.5);
        assert_eq!(stats.success_rate, 80.0);
    }

    #[test]
    fn outage_expires_successful_statistics_and_recovers() {
        let mut target = TargetStats::new(
            Target {
                address: "192.0.2.1".into(),
                name: None,
            },
            2,
        );
        let result = |latency| PingResult {
            timestamp: Utc::now(),
            latency_ms: latency,
            success: latency.is_some(),
            failure_reason: latency.is_none().then(|| "timeout".into()),
            resolved_ip: None,
        };
        target.add_ping_result(result(Some(10.0)), 2);
        target.add_ping_result(result(None), 2);
        assert_eq!(target.ping_stats.as_ref().unwrap().success_rate, 50.0);
        target.add_ping_result(result(None), 2);
        assert!(
            target.ping_stats.is_none(),
            "expired successes must not leave stale stats"
        );
        assert_eq!(target.ping_history.len(), 2);
        target.add_ping_result(result(None), 2);
        assert_eq!(target.failure_log.len(), 2);
        target.add_ping_result(result(Some(20.0)), 2);
        assert_eq!(target.ping_stats.as_ref().unwrap().mean, 20.0);
        assert_eq!(target.ping_stats.as_ref().unwrap().success_rate, 50.0);
    }
}
