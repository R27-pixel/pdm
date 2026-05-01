// SPDX-FileCopyrightText: 2024 PDM Authors
//
// SPDX-License-Identifier: AGPL-3.0-or-later

#[derive(Debug, Default, Clone)]
pub struct ParsedP2PoolState {
    /// Total accepted shares (from metrics shares_accepted_total)
    pub shares_accepted: u64,
    /// Total rejected shares (from metrics shares_rejected_total)
    pub shares_rejected: u64,
    /// Current pool difficulty (from metrics pool_difficulty)
    pub pool_difficulty: f64,
    /// Best share difficulty this session (from metrics best_share)
    pub best_share: f64,
    /// Best share ever (from metrics best_share_ever)
    pub best_share_ever: f64,

    /// Per-worker stats parsed from worker_* metric lines.
    /// Keyed by worker address.
    pub workers: Vec<WorkerStat>,

    /// Human-readable status of the last share event (from WS or metrics)
    pub last_share_status: String,
    /// Timestamp string of last share (from WS Share event)
    pub last_submit_time: String,

    /// Recent WS event log lines for the Logs tab
    pub recent_logs: Vec<String>,
}

#[derive(Debug, Default, Clone)]
pub struct WorkerStat {
    pub address: String,
    /// worker_shares_valid_total
    pub shares_valid: u64,
    /// worker_last_share_at (Unix timestamp)
    pub last_share_at: u64,
    /// worker_best_share
    pub best_share: f64,
}

impl ParsedP2PoolState {
    pub fn new() -> Self {
        Self {
            shares_accepted: 0,
            shares_rejected: 0,
            pool_difficulty: 0.0,
            best_share: 0.0,
            best_share_ever: 0.0,
            workers: Vec::new(),
            last_share_status: "No shares yet".to_string(),
            last_submit_time: "-".to_string(),
            recent_logs: Vec::new(),
        }
    }

    /// Parse Prometheus text from GET /metrics.
    /// Called on every REST poll tick.
    pub fn parse_metrics(&mut self, text: &str) {
        // Rebuild worker list from scratch on each poll
        let mut workers: std::collections::HashMap<String, WorkerStat> =
            std::collections::HashMap::new();

        for line in text.lines() {
            if line.starts_with('#') || line.is_empty() {
                continue;
            }

            // shares_accepted_total <N>
            if line.starts_with("shares_accepted_total") {
                self.shares_accepted = last_word_as(line).unwrap_or(0);
                if self.shares_accepted > 0 && self.last_share_status == "No shares yet" {
                    self.last_share_status = format!("{} accepted", self.shares_accepted);
                }
                continue;
            }

            // shares_rejected_total <N>
            if line.starts_with("shares_rejected_total") {
                self.shares_rejected = last_word_as(line).unwrap_or(0);
                continue;
            }

            // pool_difficulty <F>
            if line.starts_with("pool_difficulty") {
                self.pool_difficulty = last_word_as(line).unwrap_or(0.0);
                continue;
            }

            // best_share <F>  (space avoids matching best_share_ever)
            if line.starts_with("best_share ") {
                self.best_share = last_word_as(line).unwrap_or(0.0);
                continue;
            }

            // best_share_ever <F>
            if line.starts_with("best_share_ever") {
                self.best_share_ever = last_word_as(line).unwrap_or(0.0);
                continue;
            }

            // worker_shares_valid_total{address="tb1q..."} <N>
            if line.starts_with("worker_shares_valid_total{") {
                if let Some((addr, val)) = parse_worker_line(line) {
                    workers.entry(addr).or_default().shares_valid = val.parse().unwrap_or(0);
                }
                continue;
            }

            // worker_last_share_at{address="tb1q..."} <unix_ts>
            if line.starts_with("worker_last_share_at{") {
                if let Some((addr, val)) = parse_worker_line(line) {
                    let ts: u64 = val.parse().unwrap_or(0);
                    let w = workers.entry(addr).or_default();
                    w.last_share_at = ts;
                    // Use the most recent worker timestamp as last_submit_time
                    if ts > 0 && self.last_submit_time == "-" {
                        self.last_submit_time = chrono::DateTime::from_timestamp(ts as i64, 0)
                            .map(|dt| dt.format("%H:%M:%S UTC").to_string())
                            .unwrap_or_else(|| ts.to_string());
                    }
                }
                continue;
            }

            // worker_best_share{address="tb1q..."} <F>
            if line.starts_with("worker_best_share{") {
                if let Some((addr, val)) = parse_worker_line(line) {
                    workers.entry(addr).or_default().best_share = val.parse().unwrap_or(0.0);
                }
                continue;
            }
        }

        // Flush worker address into struct and collect
        self.workers = workers
            .into_iter()
            .map(|(addr, mut w)| {
                w.address = addr;
                w
            })
            .collect();

        // Sort by most recent activity
        self.workers
            .sort_by(|a, b| b.last_share_at.cmp(&a.last_share_at));
    }

    /// Feed a WS event line forwarded by connect_p2pool_websocket.
    /// Format: "[SHARE EVENT] {\"topic\":\"Share\",\"data\":{...}}"
    ///         "[PEER EVENT]  {\"topic\":\"Peer\",\"data\":{...}}"
    pub fn parse_ws_event(&mut self, line: &str) {
        let line = line.trim();
        if line.is_empty() {
            return;
        }

        self.recent_logs.push(line.to_string());
        if self.recent_logs.len() > 200 {
            self.recent_logs.drain(..self.recent_logs.len() - 200);
        }

        let pos = match line.find('{') {
            Some(p) => p,
            None => return,
        };

        let v: serde_json::Value = match serde_json::from_str(&line[pos..]) {
            Ok(v) => v,
            Err(_) => return,
        };

        match v.get("topic").and_then(|t| t.as_str()) {
            Some("Share") => {
                if let Some(data) = v.get("data") {
                    let height = data.get("height").and_then(|h| h.as_u64());
                    let ts = data.get("timestamp").and_then(|t| t.as_u64());

                    self.last_share_status = match height {
                        Some(h) => format!("Accepted at height {h}"),
                        None => "Share accepted".to_string(),
                    };

                    if let Some(t) = ts {
                        self.last_submit_time = chrono::DateTime::from_timestamp(t as i64, 0)
                            .map(|dt| dt.format("%H:%M:%S UTC").to_string())
                            .unwrap_or_else(|| t.to_string());
                    }
                }
            }
            Some("Peer") => {}

            _ => {}
        }
    }

    /// True if any worker has submitted at least one share.
    /// This is the closest approximation to "miner active" the API exposes.
    pub fn any_worker_active(&self) -> bool {
        self.workers
            .iter()
            .any(|w| w.shares_valid > 0 || w.last_share_at > 0)
    }
}

fn last_word_as<T: std::str::FromStr>(line: &str) -> Option<T> {
    line.split_whitespace().last()?.parse().ok()
}

/// Parse `metric_name{address="tb1q..."} value` → (address, value_str)
fn parse_worker_line(line: &str) -> Option<(String, String)> {
    let addr_start = line.find("address=\"")? + "address=\"".len();
    let addr_end = line[addr_start..].find('"')? + addr_start;
    let addr = line[addr_start..addr_end].to_string();
    let value = line.split_whitespace().last()?.to_string();
    Some((addr, value))
}

#[cfg(test)]
mod tests {
    use super::*;

    const METRICS: &str = r#"
shares_accepted_total 7
shares_rejected_total 2
pool_difficulty 10000
best_share 99999
best_share_ever 199999
worker_shares_valid_total{address="tb1qabc"} 7
worker_last_share_at{address="tb1qabc"} 1777530500
worker_best_share{address="tb1qabc"} 99999
"#;

    #[test]
    fn parses_pool_counters() {
        let mut s = ParsedP2PoolState::new();
        s.parse_metrics(METRICS);
        assert_eq!(s.shares_accepted, 7);
        assert_eq!(s.shares_rejected, 2);
        assert_eq!(s.pool_difficulty, 10000.0);
        assert_eq!(s.best_share, 99999.0);
        assert_eq!(s.best_share_ever, 199999.0);
    }

    #[test]
    fn parses_worker_stats() {
        let mut s = ParsedP2PoolState::new();
        s.parse_metrics(METRICS);
        assert_eq!(s.workers.len(), 1);
        assert_eq!(s.workers[0].address, "tb1qabc");
        assert_eq!(s.workers[0].shares_valid, 7);
        assert_eq!(s.workers[0].last_share_at, 1777530500);
        assert_eq!(s.workers[0].best_share, 99999.0);
    }

    #[test]
    fn any_worker_active_true_when_shares_submitted() {
        let mut s = ParsedP2PoolState::new();
        s.parse_metrics(METRICS);
        assert!(s.any_worker_active());
    }

    #[test]
    fn any_worker_active_false_when_no_workers() {
        let s = ParsedP2PoolState::new();
        assert!(!s.any_worker_active());
    }

    #[test]
    fn ws_share_event_updates_status() {
        let mut s = ParsedP2PoolState::new();
        s.parse_ws_event(r#"[SHARE EVENT] {"topic":"Share","data":{"height":42,"timestamp":1777530500,"blockhash":"abc","prev_blockhash":"def","miner_address":"tb1q","bits":"1d00ffff","uncles":[]}}"#);
        assert_eq!(s.last_share_status, "Accepted at height 42");
        assert_ne!(s.last_submit_time, "-");
    }

    #[test]
    fn ws_status_not_overwritten_by_metrics() {
        let mut s = ParsedP2PoolState::new();
        s.last_share_status = "Accepted at height 99".to_string();
        s.parse_metrics("shares_accepted_total 3\n");
        assert_eq!(s.last_share_status, "Accepted at height 99");
    }
}
