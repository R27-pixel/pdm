// SPDX-FileCopyrightText: 2024 PDM Authors
// SPDX-License-Identifier: AGPL-3.0-or-later

#[derive(Debug, Default, Clone, PartialEq)]
pub struct P2PoolMetrics {
    pub shares_accepted: u64,
    pub shares_rejected: u64,
    pub pool_difficulty: u64,
    pub start_time: u64,
    pub coinbase_total: u64,
}

impl P2PoolMetrics {
    /// Parses a raw Prometheus text response into our P2PoolMetrics struct
    pub fn parse_prometheus(raw_text: &str) -> Self {
        let mut metrics = P2PoolMetrics::default();

        for line in raw_text.lines() {
            let trimmed = line.trim();

            // Skip comments and empty lines
            if trimmed.starts_with('#') || trimmed.is_empty() {
                continue;
            }

            // Split by whitespace
            let mut parts = trimmed.split_whitespace();
            if let (Some(key), Some(value_str)) = (parts.next(), parts.next()) {
                // Try to parse the value as a u64
                let value = value_str.parse::<u64>().unwrap_or(0);

                // Map the Prometheus keys to our struct fields
                match key {
                    "shares_accepted_total" => metrics.shares_accepted = value,
                    "shares_rejected_total" => metrics.shares_rejected = value,
                    "pool_difficulty" => metrics.pool_difficulty = value,
                    "start_time_seconds" => metrics.start_time = value,
                    "coinbase_total" => metrics.coinbase_total = value,
                    _ => {} // Ignore metrics we don't care about
                }
            }
        }

        metrics
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_prometheus_output() {
        let raw_data = "\
# HELP shares_accepted_total Total number of accepted shares
# TYPE shares_accepted_total counter
shares_accepted_total 0

# HELP start_time_seconds Pool start time in Unix timestamp
# TYPE start_time_seconds gauge
start_time_seconds 1771149691

coinbase_output{index=\"0\",address=\"tb1qyazxde6558qj6z3d9np5e6msmrspwpf6k0qggk\"} 2500000000
coinbase_total 2500000000
";

        let metrics = P2PoolMetrics::parse_prometheus(raw_data);

        assert_eq!(metrics.shares_accepted, 0);
        assert_eq!(metrics.start_time, 1771149691);
        assert_eq!(metrics.coinbase_total, 2500000000);
    }
}
