// SPDX-FileCopyrightText: 2024 PDM Authors
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use anyhow::Result;
use config::{Config, File, FileFormat};
use std::{collections::HashSet, path::Path};

#[derive(Debug, Clone)]
pub struct CoreConfig {
    datadir: String,
    txindex: bool,
    prune: u32,
    blocksonly: bool,
    dbcache: u32,
    maxmempool: String,
    pid: String,
}

#[derive(Debug, Clone)]
pub struct Network {
    testnet: bool,
    regtest: bool,
    signet: bool,
    listen: bool,
    bind: String,
    port: u32,
    maxconnections: u32,
    proxy: String,
    onion: String,
    upnp: bool,
}

#[derive(Debug, Clone)]
pub struct RPC {
    server: bool,
    rpcuser: String,
    rpcpassword: String,
    rpcauth: String,
    rpcport: u32,
    rpcbind: String,
    rpcallowip: String,
    rpcthreads: u32,
}

#[derive(Debug, Clone)]
pub struct Wallet {
    disablewallet: bool,
    fallbackfee: String,
    discardfee: String,
    mintxfee: String,
    paytxfee: String,
}

#[derive(Debug, Clone)]
pub struct Debug {
    debug: String,
    logips: bool,
    shrinkdebugfile: bool,
}

#[derive(Debug, Clone)]
pub struct Mining {
    blockmaxweight: u32,
    minrelaytxfee: String,
}

#[derive(Debug, Clone)]
pub struct ZMQ {
    zmqpubhashblock: String,
    zmqpubhashtx: String,
    zmqpubrawblock: String,
    zmqpubrawtx: String,
}

#[derive(Debug, Clone)]
pub struct BitcoinConfig {
    core: CoreConfig,
    network: Network,
    rpc: RPC,
    wallet: Wallet,
    debug: Debug,
    mining: Mining,
    zmq: ZMQ,
}

/// Parse bitcoin.conf file
pub fn parse_config(path: &Path) -> Result<Vec<ConfigEntry>> {
    //let schema_list = get_default_schema();
    let mut entries = Vec::new();
    let mut found_keys = std::collections::HashSet::new();
    let mut builder = Config::builder();

    if path.exists() {
        builder = builder.add_source(File::from(path).format(FileFormat::Ini));
    }

    let config = match builder.build() {
        Ok(cfg) => cfg,
        Err(_) => {
            for schema in schema_list {
                entries.push(ConfigEntry {
                    key: schema.key.clone(),
                    value: schema.default.clone(),
                    schema: Some(schema),
                    enabled: false,
                });
            }
            return Ok(entries);
        }
    };

    let mut config_keys = HashSet::new();

    let sections = vec!["", "main", "test", "signet", "regtest"];

    for section in &sections {
        if let Ok(table) = if section.is_empty() {
            config.get_table("")
        } else {
            config.get_table(section)
        } {
            for key in table.keys() {
                let actual_key = if key.contains('.') {
                    key.split('.').next_back().unwrap_or(key).to_string()
                } else {
                    key.clone()
                };
                config_keys.insert(actual_key);
            }
        }
    }

    for schema in &schema_list {
        let key = &schema.key;
        let mut value = schema.default.clone();
        let mut enabled = false;

        for section in &sections {
            let lookup_key = if section.is_empty() {
                key.clone()
            } else {
                format!("{}.{}", section, key)
            };

            if let Ok(val) = config.get_string(&lookup_key) {
                value = val;
                enabled = true;
                found_keys.insert(key.clone());
                break;
            }

            if let Ok(val) = config.get_bool(&lookup_key) {
                value = if val {
                    "1".to_string()
                } else {
                    "0".to_string()
                };
                enabled = true;
                found_keys.insert(key.clone());
                break;
            }

            if let Ok(val) = config.get_int(&lookup_key) {
                value = val.to_string();
                enabled = true;
                found_keys.insert(key.clone());
                break;
            }

            if let Ok(val) = config.get_float(&lookup_key) {
                value = val.to_string();
                enabled = true;
                found_keys.insert(key.clone());
                break;
            }
        }

        entries.push(ConfigEntry {
            key: key.clone(),
            value,
            schema: Some(schema.clone()),
            enabled,
        });
    }

    for config_key in &config_keys {
        if !found_keys.contains(config_key) {
            let value = config
                .get_string(config_key)
                .or_else(|_| {
                    config
                        .get_bool(config_key)
                        .map(|b| if b { "1".to_string() } else { "0".to_string() })
                })
                .or_else(|_| config.get_int(config_key).map(|i| i.to_string()))
                .or_else(|_| config.get_float(config_key).map(|f| f.to_string()))
                .unwrap_or_else(|_| "".to_string());

            entries.push(ConfigEntry {
                key: config_key.clone(),
                value,
                schema: None,
                enabled: true,
            });
        }
    }

    Ok(entries)
}
