// SPDX-FileCopyrightText: 2024 PDM Authors
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct WsMessage {
    topic: String,
    data: serde_json::Value,
}

#[derive(Debug, Default, Clone)]
pub struct ParsedP2PoolState {
    pub miner_connected: bool,
    pub last_share_status: String,
    pub last_block_status: String,
    pub last_submit_time: String,
    pub recent_logs: Vec<String>,
}

impl ParsedP2PoolState {
    pub fn new() -> Self {
        Self {
            miner_connected: false,
            last_share_status: "No shares yet".to_string(),
            last_block_status: "No blocks submitted".to_string(),
            last_submit_time: "-".to_string(),
            recent_logs: Vec::new(),
        }
    }

    pub fn parse_log_line(&mut self, line: &str) {
        let line = line.trim();
        if line.is_empty() {
            return;
        }

        self.recent_logs.push(line.to_string());
        if self.recent_logs.len() > 100 {
            self.recent_logs.remove(0);
        }

        // Lines from the WS are prefixed e.g. "[SHARE EVENT] {json}"
        // Strip the prefix to get the raw JSON payload
        let json_part = if let Some(pos) = line.find('{') {
            &line[pos..]
        } else {
            return;
        };

        if let Ok(msg) = serde_json::from_str::<WsMessage>(json_part) {
            match msg.topic.as_str() {
                "Share" => {
                    // A share arrived — miner must be connected
                    self.miner_connected = true;
                    self.last_share_status = "Share accepted".to_string();
                    self.last_submit_time = msg
                        .data
                        .get("timestamp")
                        .and_then(|v| v.as_u64())
                        .map(|ts| {
                            chrono::DateTime::from_timestamp(ts as i64, 0)
                                .map(|dt| dt.format("%H:%M:%S").to_string())
                                .unwrap_or_else(|| ts.to_string())
                        })
                        .unwrap_or_else(|| chrono::Utc::now().format("%H:%M:%S").to_string());
                    if let Some(height) = msg.data.get("height").and_then(|v| v.as_u64()) {
                        self.last_share_status = format!("Accepted at height {}", height);
                    }
                }
                "Peer" => {
                    // Peer events don't affect miner/share state
                    // but are already in recent_logs for the Logs tab
                }
                "Chain" => {
                    if msg.data.get("block_submitted").is_some() {
                        self.last_block_status = "Block submitted successfully".to_string();
                    }
                }
                _ => {}
            }
        }
    }

    /// Update miner connected state and share stats from /metrics response.
    /// Called on each REST poll as a fallback since the WS only emits
    /// share events (not miner connect/disconnect events).
    pub fn parse_metrics(&mut self, metrics_text: &str) {
        for line in metrics_text.lines() {
            if line.starts_with('#') || line.is_empty() {
                continue;
            }

            // worker_last_share_at{...} <timestamp> — non-zero means a miner
            // has submitted at least one share this session
            if line.starts_with("worker_last_share_at") {
                let val: f64 = line
                    .split_whitespace()
                    .last()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(0.0);
                if val > 0.0 {
                    self.miner_connected = true;
                }
            }

            if line.starts_with("shares_accepted_total") {
                let count: u64 = line
                    .split_whitespace()
                    .last()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(0);
                if count > 0 {
                    self.last_share_status = format!("{} accepted", count);
                }
            }

            if line.starts_with("shares_rejected_total") {
                let count: u64 = line
                    .split_whitespace()
                    .last()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(0);
                if count > 0 {
                    // Only update if we don't have a better status already
                    if self.last_share_status == "No shares yet" {
                        self.last_share_status = format!("{} rejected", count);
                    }
                }
            }
        }
    }
}
