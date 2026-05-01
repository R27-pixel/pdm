// SPDX-FileCopyrightText: 2024 PDM Authors
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message};

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct WsPeerEvent {
    pub peer_id: String,
    pub status: String,
}

#[derive(Debug, Deserialize)]
pub struct WsShareEvent {
    pub height: u64,
    pub miner_address: String,
}

#[derive(Debug, Deserialize)]
pub struct WsMessage {
    pub topic: String,
    pub data: serde_json::Value,
}

pub async fn connect_p2pool_websocket(
    base_url: &str,
    token: Option<&str>,
    tx: mpsc::Sender<String>,
) {
    let ws_base = format!("{}/ws", base_url.replace("http", "ws"));
    let ws_url = match token {
        Some(t) => format!("{}?token={}", ws_base, t),
        None => ws_base,
    };

    loop {
        match connect_async(&ws_url).await {
            Ok((ws_stream, _)) => {
                let (mut write, mut read) = ws_stream.split();

                // Subscribe to all available topics immediately after connecting
                for topic in &["shares", "peers"] {
                    let msg = format!(r#"{{"action":"subscribe","topic":"{}"}}"#, topic);
                    if write.send(Message::Text(msg)).await.is_err() {
                        break;
                    }
                }

                while let Some(message) = read.next().await {
                    match message {
                        Ok(msg) => {
                            if msg.is_text() {
                                if let Ok(text) = msg.to_text() {
                                    let formatted = if let Ok(parsed) =
                                        serde_json::from_str::<WsMessage>(text)
                                    {
                                        match parsed.topic.as_str() {
                                            "Peer" => format!("[PEER EVENT] {}", text),
                                            "Share" => format!("[SHARE EVENT] {}", text),
                                            "Chain" => format!("[CHAIN EVENT] {}", text),
                                            _ => format!("[WS] {}", text),
                                        }
                                    } else {
                                        format!("[RAW] {}", text)
                                    };

                                    if tx.send(formatted).await.is_err() {
                                        // Receiver dropped — app is shutting down
                                        return;
                                    }
                                }
                            }
                        }
                        Err(_) => break, // Silent disconnect, reconnect below
                    }
                }
            }
            Err(_) => {
                // Node is down — wait and retry silently
            }
        }

        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    }
}
