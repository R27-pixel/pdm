// SPDX-FileCopyrightText: 2024 PDM Authors
// SPDX-License-Identifier: AGPL-3.0-or-later

use reqwest::Client;
use serde_json::{Value, json};

#[derive(Debug, Default, Clone, PartialEq)]
pub struct BitcoinMetrics {
    pub blocks: u64,
    pub headers: u64,
    pub connections: u32,
    pub verification_progress: f64,
    pub chain: String,
}

impl BitcoinMetrics {
    /// Asynchronously fetches and parses metrics from a Bitcoin Core RPC node
    pub async fn fetch(client: &Client, url: &str, user: &str, pass: &str) -> anyhow::Result<Self> {
        //  Fetch Blockchain Info (Blocks, Headers, Sync Progress, Chain)
        let body_chain = json!({
            "jsonrpc": "1.0",
            "id": "pdm",
            "method": "getblockchaininfo",
            "params": []
        });

        let res_chain = client
            .post(url)
            .basic_auth(user, Some(pass))
            .json(&body_chain)
            .send()
            .await?
            .json::<Value>()
            .await?;

        let result = &res_chain["result"];
        let blocks = result["blocks"].as_u64().unwrap_or(0);
        let headers = result["headers"].as_u64().unwrap_or(0);
        let verification_progress = result["verificationprogress"].as_f64().unwrap_or(0.0);
        let chain = result["chain"].as_str().unwrap_or("unknown").to_string();

        //  Fetch Network Info (Peer Connections)
        let body_net = json!({
            "jsonrpc": "1.0",
            "id": "pdm",
            "method": "getnetworkinfo",
            "params": []
        });

        let res_net = client
            .post(url)
            .basic_auth(user, Some(pass))
            .json(&body_net)
            .send()
            .await?
            .json::<Value>()
            .await?;

        let connections = res_net["result"]["connections"].as_u64().unwrap_or(0) as u32;

        Ok(BitcoinMetrics {
            blocks,
            headers,
            connections,
            verification_progress,
            chain,
        })
    }
}
