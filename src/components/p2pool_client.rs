// SPDX-FileCopyrightText: 2024 PDM Authors
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::config::load_api_config;
use reqwest::Client;
use serde::Deserialize;
use std::time::Duration;

const REQUEST_TIMEOUT_SECONDS: u64 = 10;

#[derive(Debug, Clone)]
pub struct P2PoolClient {
    client: Client,
    base_url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChainInfo {
    pub genesis_blockhash: Option<String>,
    pub chain_tip_height: Option<u64>,
    pub total_work: String,
    pub chain_tip_blockhash: Option<String>,
    pub top_candidate_height: Option<u64>,
    pub top_candidate_blockhash: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PeerInfo {
    pub peer_id: String,
    pub status: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SharesResponse {
    pub from_height: u64,
    pub to_height: u64,
    pub shares: Vec<ShareInfo>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ShareInfo {
    pub blockhash: String,
    pub prev_blockhash: String,
    pub height: u64,
    pub miner_address: String,
    pub timestamp: u64,
    pub bits: String,
    pub uncles: Vec<UncleInfo>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UncleInfo {
    pub blockhash: String,
    pub prev_blockhash: String,
    pub miner_address: String,
    pub timestamp: u64,
    pub height: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ShareDetail {
    pub blockhash: String,
    pub height: Option<u64>,
    pub status: String,
    pub parent: String,
    pub uncles: Vec<String>,
    pub miner_address: String,
    pub merkle_root: String,
    pub bits: String,
    pub time: String,
}

impl P2PoolClient {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECONDS))
            .build()
            .expect("failed to build reqwest client");

        let base_url = load_api_config()
            .expect("failed to load API config")
            .base_url;

        Self { client, base_url }
    }

    pub fn with_base_url(base_url: impl Into<String>) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECONDS))
            .build()
            .expect("failed to build reqwest client");

        Self {
            client,
            base_url: base_url.into(),
        }
    }

    pub async fn fetch_health(&self, basic_auth: Option<&str>) -> Result<String, reqwest::Error> {
        let url = format!("{}/health", self.base_url);

        let mut request = self.client.get(url);
        if let Some(auth) = basic_auth {
            request = request.header("Authorization", auth);
        }

        let response = request.send().await?;
        let text = response.text().await?;

        Ok(text)
    }

    pub async fn fetch_metrics(&self, basic_auth: Option<&str>) -> Result<String, reqwest::Error> {
        let url = format!("{}/metrics", self.base_url);

        let mut request = self.client.get(url);
        if let Some(auth) = basic_auth {
            request = request.header("Authorization", auth);
        }

        let response = request.send().await?;
        let text = response.text().await?;

        Ok(text)
    }

    pub async fn fetch_chain_info(
        &self,
        basic_auth: Option<&str>,
    ) -> Result<ChainInfo, reqwest::Error> {
        let url = format!("{}/chain_info", self.base_url);

        let mut request = self.client.get(url);
        if let Some(auth) = basic_auth {
            request = request.header("Authorization", auth);
        }

        let response = request.send().await?;
        let data = response.json::<ChainInfo>().await?;

        Ok(data)
    }

    pub async fn fetch_peers(
        &self,
        basic_auth: Option<&str>,
    ) -> Result<Vec<PeerInfo>, reqwest::Error> {
        let url = format!("{}/peers", self.base_url);

        let mut request = self.client.get(url);
        if let Some(auth) = basic_auth {
            request = request.header("Authorization", auth);
        }

        let response = request.send().await?;
        let data = response.json::<Vec<PeerInfo>>().await?;

        Ok(data)
    }

    pub async fn fetch_shares(
        &self,
        to: Option<u64>,
        num: Option<u64>,
        basic_auth: Option<&str>,
    ) -> Result<SharesResponse, reqwest::Error> {
        let url = format!("{}/shares", self.base_url);

        let mut request = self.client.get(url);

        if let Some(to) = to {
            request = request.query(&[("to", to)]);
        }

        if let Some(num) = num {
            request = request.query(&[("num", num)]);
        }

        if let Some(auth) = basic_auth {
            request = request.header("Authorization", auth);
        }

        let response = request.send().await?;
        let data = response.json::<SharesResponse>().await?;

        Ok(data)
    }

    pub async fn fetch_candidates(
        &self,
        to: Option<u64>,
        num: Option<u64>,
    ) -> Result<SharesResponse, reqwest::Error> {
        let url = format!("{}/candidates", self.base_url);

        let mut request = self.client.get(url);

        if let Some(to) = to {
            request = request.query(&[("to", to)]);
        }

        if let Some(num) = num {
            request = request.query(&[("num", num)]);
        }

        let response = request.send().await?;
        let data = response.json::<SharesResponse>().await?;

        Ok(data)
    }

    pub async fn fetch_share_by_height(
        &self,
        height: u64,
    ) -> Result<Vec<ShareDetail>, reqwest::Error> {
        let url = format!("{}/share", self.base_url);

        let response = self
            .client
            .get(url)
            .query(&[("height", height)])
            .send()
            .await?;

        let data = response.json::<Vec<ShareDetail>>().await?;

        Ok(data)
    }

    pub async fn fetch_pplns_shares(
        &self,
        limit: Option<u64>,
    ) -> Result<serde_json::Value, reqwest::Error> {
        let url = format!("{}/pplns_shares", self.base_url);

        let mut request = self.client.get(url);

        if let Some(limit) = limit {
            request = request.query(&[("limit", limit)]);
        }

        let response = request.send().await?;
        let data = response.json::<serde_json::Value>().await?;

        Ok(data)
    }
}

impl Default for P2PoolClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = P2PoolClient::new();

        let expected = load_api_config().unwrap().base_url;
        assert_eq!(client.base_url, expected);
    }

    #[test]
    fn test_custom_base_url() {
        let client = P2PoolClient::with_base_url("http://localhost:9999");

        assert_eq!(client.base_url, "http://localhost:9999");
    }

    #[test]
    fn test_chain_info_model() {
        let model = ChainInfo {
            genesis_blockhash: Some("genesis".to_string()),
            chain_tip_height: Some(100),
            total_work: "abcd1234".to_string(),
            chain_tip_blockhash: Some("tiphash".to_string()),
            top_candidate_height: Some(101),
            top_candidate_blockhash: Some("candidate".to_string()),
        };

        assert_eq!(model.chain_tip_height, Some(100));
        assert_eq!(model.top_candidate_height, Some(101));
    }

    #[test]
    fn test_peer_model() {
        let peer = PeerInfo {
            peer_id: "12D3KooWExample".to_string(),
            status: "Active".to_string(),
        };

        assert_eq!(peer.peer_id, "12D3KooWExample");
        assert_eq!(peer.status, "Active");
    }
}
