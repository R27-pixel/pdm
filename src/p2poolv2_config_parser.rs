// SPDX-FileCopyrightText: 2024 PDM Authors
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use anyhow::{Result, anyhow};
use bitcoin::secp256k1::PublicKey as CompressedPublicKey;
use bitcoin::{Address, Network, address::NetworkChecked};
use config::{Config as ConfigLoader, Environment, File, FileFormat};
use serde::Deserialize;
use std::marker::PhantomData;
use std::path::Path;
use std::str::FromStr;

// UI MODEL
#[derive(Debug, Clone, PartialEq)]
pub struct ConfigEntry {
    pub section: String,
    pub key: String,
    pub value: String,
    pub is_default: bool,
}

// P2POOL SCHEMA
const MAX_POOL_SIGNATURE_LENGTH: usize = 16;

#[derive(Debug, Clone, Default)]
pub struct Raw;
#[derive(Debug, Clone)]
pub struct Parsed;

fn default_hostname() -> String {
    "0.0.0.0".to_string()
}
fn default_stratum_port() -> u16 {
    3333
}
fn default_start_difficulty() -> u64 {
    10000
}
fn default_minimum_difficulty() -> u64 {
    100
}
fn default_zmqpubhashblock() -> String {
    "tcp://127.0.0.1:28332".to_string()
}
fn default_network() -> Network {
    Network::Signet
}
fn default_version_mask() -> i32 {
    0x1fffe000
}
fn default_difficulty_multiplier() -> f64 {
    1.0
}
fn default_listen_address() -> String {
    "/ip4/0.0.0.0/tcp/6884".to_string()
}

#[derive(Debug, Deserialize, Clone)]
pub struct StratumConfig<State = Raw> {
    #[serde(default = "default_hostname")]
    pub hostname: String,
    #[serde(default = "default_stratum_port")]
    pub port: u16,
    #[serde(default = "default_start_difficulty")]
    pub start_difficulty: u64,
    #[serde(default = "default_minimum_difficulty")]
    pub minimum_difficulty: u64,
    pub maximum_difficulty: Option<u64>,
    pub solo_address: Option<String>,
    #[serde(default = "default_zmqpubhashblock")]
    pub zmqpubhashblock: String,
    pub bootstrap_address: Option<String>,
    pub donation_address: Option<String>,
    pub donation: Option<u16>,
    pub fee_address: Option<String>,
    pub fee: Option<u16>,
    #[serde(default = "default_network", deserialize_with = "deserialize_network")]
    pub network: Network,
    #[serde(
        default = "default_version_mask",
        deserialize_with = "deserialize_version_mask"
    )]
    pub version_mask: i32,
    #[serde(default = "default_difficulty_multiplier")]
    pub difficulty_multiplier: f64,
    pub ignore_difficulty: Option<bool>,
    pub pool_signature: Option<String>,
    #[serde(skip)]
    pub(crate) bootstrap_address_parsed: Option<Address<NetworkChecked>>,
    #[serde(skip)]
    pub(crate) donation_address_parsed: Option<Address<NetworkChecked>>,
    #[serde(skip)]
    pub(crate) fee_address_parsed: Option<Address<NetworkChecked>>,
    #[serde(skip, default)]
    _state: PhantomData<State>,
}

impl StratumConfig<Raw> {
    pub fn parse(self) -> Result<StratumConfig<Parsed>> {
        if let Some(sig) = &self.pool_signature {
            if sig.len() > MAX_POOL_SIGNATURE_LENGTH {
                return Err(anyhow!("Pool signature exceeds max length"));
            }
        }

        let bootstrap = if let Some(addr_str) = &self.bootstrap_address {
            let addr =
                Address::from_str(addr_str).map_err(|_| anyhow!("Invalid bootstrap_address"))?;
            let addr = addr
                .require_network(self.network)
                .map_err(|_| anyhow!("Invalid bootstrap_address"))?;
            Some(addr)
        } else {
            None
        };

        let donation = if let Some(addr) = &self.donation_address {
            Some(
                Address::from_str(addr)
                    .map_err(|_| anyhow!("Invalid donation_address"))?
                    .require_network(self.network)
                    .map_err(|_| anyhow!("Invalid donation_address"))?,
            )
        } else {
            None
        };

        if self.donation.is_some() && donation.is_none() {
            return Err(anyhow!("donation_address is required when donation is set"));
        }
        let fee = if let Some(addr) = &self.fee_address {
            Some(
                Address::from_str(addr)
                    .map_err(|_| anyhow!("Invalid fee_address"))?
                    .require_network(self.network)
                    .map_err(|_| anyhow!("Invalid fee_address"))?,
            )
        } else {
            None
        };

        if self.fee.is_some() && fee.is_none() {
            return Err(anyhow!("fee_address is required when fee is set"));
        }

        Ok(StratumConfig {
            hostname: self.hostname,
            port: self.port,
            start_difficulty: self.start_difficulty,
            minimum_difficulty: self.minimum_difficulty,
            maximum_difficulty: self.maximum_difficulty,
            solo_address: self.solo_address,
            zmqpubhashblock: self.zmqpubhashblock,
            bootstrap_address: self.bootstrap_address,
            donation_address: self.donation_address,
            donation: self.donation,
            fee_address: self.fee_address,
            fee: self.fee,
            network: self.network,
            version_mask: self.version_mask,
            difficulty_multiplier: self.difficulty_multiplier,
            ignore_difficulty: self.ignore_difficulty,
            pool_signature: self.pool_signature,
            bootstrap_address_parsed: bootstrap,
            donation_address_parsed: donation,
            fee_address_parsed: fee,
            _state: PhantomData,
        })
    }
}

fn deserialize_network<'de, D>(deserializer: D) -> Result<Network, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: String = serde::Deserialize::deserialize(deserializer)?;
    Network::from_core_arg(&s).map_err(serde::de::Error::custom)
}

fn deserialize_version_mask<'de, D>(deserializer: D) -> Result<i32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: String = serde::Deserialize::deserialize(deserializer)?;
    i32::from_str_radix(&s, 16)
        .map_err(|_| serde::de::Error::custom("version_mask must be hex (e.g. 1fffe000)"))
}

// NETWORK CONFIG

#[derive(Debug, Deserialize, Clone, Default)]
pub struct NetworkConfig {
    #[serde(default = "default_listen_address")]
    pub listen_address: String,
    #[serde(default)]
    pub dial_peers: Vec<String>,
    #[serde(default = "d10")]
    pub max_pending_incoming: u32,
    #[serde(default = "d10")]
    pub max_pending_outgoing: u32,
    #[serde(default = "d50")]
    pub max_established_incoming: u32,
    #[serde(default = "d50")]
    pub max_established_outgoing: u32,
    #[serde(default = "d1_u32")]
    pub max_established_per_peer: u32,
    #[serde(default = "d10")]
    pub max_workbase_per_second: u32,
    #[serde(default = "d10")]
    pub max_userworkbase_per_second: u32,
    #[serde(default = "d100")]
    pub max_miningshare_per_second: u32,
    #[serde(default = "d100")]
    pub max_inventory_per_second: u32,
    #[serde(default = "d100")]
    pub max_transaction_per_second: u32,
    #[serde(default = "d1_u64")]
    pub rate_limit_window_secs: u64,
    #[serde(default = "d1_u64")]
    pub max_requests_per_second: u64,
    #[serde(default = "d60")]
    pub peer_inactivity_timeout_secs: u64,
    #[serde(default = "d30")]
    pub dial_timeout_secs: u64,
}

fn d1_u32() -> u32 {
    1
}
fn d10() -> u32 {
    10
}
fn d50() -> u32 {
    50
}
fn d100() -> u32 {
    100
}
fn d1_u64() -> u64 {
    1
}
fn d60() -> u64 {
    60
}
fn d30() -> u64 {
    30
}

#[derive(Debug, Deserialize, Clone)]
pub struct StoreConfig {
    pub path: String,
    #[serde(default = "d1_u64")]
    pub background_task_frequency_hours: u64,
    #[serde(default = "d7")]
    pub pplns_ttl_days: u64,
}
fn d7() -> u64 {
    7
}

#[derive(Debug, Deserialize, Clone)]
pub struct MinerConfig {
    pub pubkey: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct BitcoinRpcConfig {
    pub url: String,
    pub username: String,
    pub password: String,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct LoggingConfig {
    pub file: Option<String>,
    #[serde(default = "log_level")]
    pub level: String,
    #[serde(default = "stats_dir")]
    pub stats_dir: String,
}
fn log_level() -> String {
    "info".into()
}
fn stats_dir() -> String {
    "./logs/stats".into()
}

#[derive(Debug, Deserialize, Clone)]
pub struct ApiConfig {
    pub hostname: String,
    pub port: u16,
    pub auth_user: Option<String>,
    pub auth_token: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct P2PoolConfig {
    #[serde(default)]
    pub network: NetworkConfig,
    pub store: Option<StoreConfig>,
    pub stratum: Option<StratumConfig<Raw>>,
    pub miner: Option<MinerConfig>,
    pub bitcoinrpc: Option<BitcoinRpcConfig>,
    #[serde(default)]
    pub logging: LoggingConfig,
    pub api: Option<ApiConfig>,
}

// PARSER

pub fn parse_config(path: &Path) -> Result<Vec<ConfigEntry>> {
    let raw_text = if path.exists() {
        std::fs::read_to_string(path).unwrap_or_default()
    } else {
        String::new()
    };

    let env_override_present = std::env::vars().any(|(k, _)| k.starts_with("P2POOL_"));

    // Accept configs with any known section or env var
    let looks_like_p2pool = env_override_present
        || raw_text.contains("[stratum]")
        || raw_text.contains("[store]")
        || raw_text.contains("[network]")
        || raw_text.contains("[logging]")
        || raw_text.contains("[api]")
        || raw_text.contains("[miner]")
        || raw_text.contains("[bitcoinrpc]");

    if !looks_like_p2pool {
        return Err(anyhow!("Invalid P2Pool config: not a p2pool configuration"));
    }

    let mut cfg = ConfigLoader::builder();
    if path.exists() {
        cfg = cfg.add_source(File::from(path).format(FileFormat::Toml));
    }
    cfg = cfg.add_source(Environment::with_prefix("P2POOL").separator("_"));
    let raw = cfg.build()?;

    let p: P2PoolConfig = raw.clone().try_deserialize().map_err(|e| {
        anyhow!(
            "Failed to deserialize config: {}\nFile: {}",
            e,
            path.display()
        )
    })?;

    if let Some(stratum_raw) = &p.stratum {
        stratum_raw.clone().parse()?;
    }

    let network_section_present = raw_text.contains("[network]");
    let stratum_section_present = raw_text.contains("[stratum]");
    let logging_section_present = raw_text.contains("[logging]");

    Ok(flatten(
        &p,
        network_section_present,
        stratum_section_present,
        logging_section_present,
    ))
}

// FLATTENER

fn flatten(
    p: &P2PoolConfig,
    network_section_present: bool,
    stratum_section_present: bool,
    logging_section_present: bool,
) -> Vec<ConfigEntry> {
    let mut e = Vec::new();

    // NETWORK
    let n = &p.network;
    macro_rules! n {
        ($k:expr, $v:expr, $default_val:expr) => {
            push(
                &mut e,
                "network",
                $k,
                $v,
                !network_section_present && $default_val,
            )
        };
    }
    n!(
        "listen_address",
        n.listen_address.clone(),
        n.listen_address.is_empty()
    );
    n!(
        "dial_peers",
        n.dial_peers.join(", "),
        n.dial_peers.is_empty()
    );
    n!(
        "max_pending_incoming",
        n.max_pending_incoming.to_string(),
        n.max_pending_incoming == 10
    );
    n!(
        "max_pending_outgoing",
        n.max_pending_outgoing.to_string(),
        n.max_pending_outgoing == 10
    );
    n!(
        "max_established_incoming",
        n.max_established_incoming.to_string(),
        n.max_established_incoming == 50
    );
    n!(
        "max_established_outgoing",
        n.max_established_outgoing.to_string(),
        n.max_established_outgoing == 50
    );
    n!(
        "max_established_per_peer",
        n.max_established_per_peer.to_string(),
        !network_section_present && n.max_established_per_peer == 1
    );

    n!(
        "max_workbase_per_second",
        n.max_workbase_per_second.to_string(),
        n.max_workbase_per_second == 10
    );
    n!(
        "max_userworkbase_per_second",
        n.max_userworkbase_per_second.to_string(),
        n.max_userworkbase_per_second == 10
    );
    n!(
        "max_miningshare_per_second",
        n.max_miningshare_per_second.to_string(),
        n.max_miningshare_per_second == 100
    );
    n!(
        "max_inventory_per_second",
        n.max_inventory_per_second.to_string(),
        n.max_inventory_per_second == 100
    );
    n!(
        "max_transaction_per_second",
        n.max_transaction_per_second.to_string(),
        n.max_transaction_per_second == 100
    );
    n!(
        "rate_limit_window_secs",
        n.rate_limit_window_secs.to_string(),
        n.rate_limit_window_secs == 1
    );
    n!(
        "max_requests_per_second",
        n.max_requests_per_second.to_string(),
        n.max_requests_per_second == 1
    );
    n!(
        "peer_inactivity_timeout_secs",
        n.peer_inactivity_timeout_secs.to_string(),
        n.peer_inactivity_timeout_secs == 60
    );
    n!(
        "dial_timeout_secs",
        n.dial_timeout_secs.to_string(),
        n.dial_timeout_secs == 30
    );

    // STORE
    if let Some(s) = &p.store {
        macro_rules! s_store {
            ($k:expr, $v:expr, $d:expr) => {
                push(&mut e, "store", $k, $v, $d)
            };
        }
        s_store!("path", s.path.clone(), s.path == "./store.db");
        s_store!(
            "background_task_frequency_hours",
            s.background_task_frequency_hours.to_string(),
            s.background_task_frequency_hours == 1
        );
        s_store!(
            "pplns_ttl_days",
            s.pplns_ttl_days.to_string(),
            s.pplns_ttl_days == 7
        );
    }

    // STRATUM
    if let Some(stratum) = &p.stratum {
        macro_rules! stratum_m {
            ($k:expr, $v:expr, $d:expr) => {
                push(&mut e, "stratum", $k, $v, !stratum_section_present && $d)
            };
        }
        stratum_m!(
            "hostname",
            stratum.hostname.clone(),
            stratum.hostname == "0.0.0.0"
        );
        stratum_m!("port", stratum.port.to_string(), stratum.port == 3333);
        stratum_m!(
            "start_difficulty",
            stratum.start_difficulty.to_string(),
            stratum.start_difficulty == 10000
        );
        stratum_m!(
            "minimum_difficulty",
            stratum.minimum_difficulty.to_string(),
            stratum.minimum_difficulty == 100
        );
        opt(
            &mut e,
            "stratum",
            "maximum_difficulty",
            stratum.maximum_difficulty.map(|v| v.to_string()),
            false,
        );
        opt(
            &mut e,
            "stratum",
            "solo_address",
            stratum.solo_address.clone(),
            false,
        );
        stratum_m!(
            "zmqpubhashblock",
            stratum.zmqpubhashblock.clone(),
            stratum.zmqpubhashblock == "tcp://127.0.0.1:28332"
        );
        opt(
            &mut e,
            "stratum",
            "bootstrap_address",
            stratum.bootstrap_address.clone(),
            false,
        );
        opt(
            &mut e,
            "stratum",
            "donation_address",
            stratum.donation_address.clone(),
            false,
        );
        opt(
            &mut e,
            "stratum",
            "donation",
            stratum.donation.map(|v| {
                let pct = v as f64 / 100.0;
                if pct.fract() == 0.0 {
                    format!("{} bp ({:.0}%)", v, pct)
                } else {
                    format!("{} bp ({:.2}%)", v, pct)
                }
            }),
            false,
        );
        opt(
            &mut e,
            "stratum",
            "fee_address",
            stratum.fee_address.clone(),
            false,
        );
        opt(
            &mut e,
            "stratum",
            "fee",
            stratum.fee.map(|v| {
                let pct = v as f64 / 100.0;
                if pct.fract() == 0.0 {
                    format!("{} bp ({:.0}%)", v, pct)
                } else {
                    format!("{} bp ({:.2}%)", v, pct)
                }
            }),
            false,
        );
        stratum_m!(
            "network",
            format!("{:?}", stratum.network).to_lowercase(),
            stratum.network == Network::Signet
        );
        stratum_m!(
            "version_mask",
            format!("{:08x}", stratum.version_mask),
            stratum.version_mask == 0x1fffe000
        );
        stratum_m!(
            "difficulty_multiplier",
            format!("{:.1}", stratum.difficulty_multiplier),
            stratum.difficulty_multiplier == 1.0
        );
        opt(
            &mut e,
            "stratum",
            "ignore_difficulty",
            stratum.ignore_difficulty.map(|v| v.to_string()),
            false,
        );
        opt(
            &mut e,
            "stratum",
            "pool_signature",
            stratum.pool_signature.clone(),
            false,
        );
    }

    // MINER
    if let Some(m) = &p.miner {
        if CompressedPublicKey::from_str(&m.pubkey).is_err() {
            push(&mut e, "miner", "pubkey", "<invalid pubkey>".into(), false);
        } else {
            push(&mut e, "miner", "pubkey", m.pubkey.clone(), false);
        }
    }

    // BITCOIN RPC
    if let Some(b) = &p.bitcoinrpc {
        macro_rules! b_m {
            ($k:expr, $v:expr, $d:expr) => {
                push(&mut e, "bitcoinrpc", $k, $v, $d)
            };
        }
        b_m!("url", b.url.clone(), b.url == "http://127.0.0.1:38332");
        b_m!("username", b.username.clone(), b.username == "p2pool");
        b_m!(
            "password",
            if b.password.is_empty() {
                "<empty>".into()
            } else {
                "*****".into()
            },
            false
        );
    }

    // LOGGING
    let l = &p.logging;
    macro_rules! l_m {
        ($k:expr, $v:expr, $d:expr) => {
            push(&mut e, "logging", $k, $v, !logging_section_present && $d)
        };
    }
    opt(&mut e, "logging", "file", l.file.clone(), false);
    l_m!("level", l.level.clone(), l.level == "info");
    l_m!(
        "stats_dir",
        l.stats_dir.clone(),
        l.stats_dir == "./logs/stats"
    );

    // API
    if let Some(a) = &p.api {
        macro_rules! a_m {
            ($k:expr, $v:expr, $d:expr) => {
                push(&mut e, "api", $k, $v, $d)
            };
        }
        a_m!("hostname", a.hostname.clone(), a.hostname == "127.0.0.1");
        a_m!("port", a.port.to_string(), false);
        opt(&mut e, "api", "auth_user", a.auth_user.clone(), false);
        opt(
            &mut e,
            "api",
            "auth_token",
            a.auth_token.clone().map(|_| String::from("*****")),
            false,
        );
    }

    e
}

// HELPERS
fn push(e: &mut Vec<ConfigEntry>, s: &str, k: &str, v: String, is_default: bool) {
    e.push(ConfigEntry {
        section: s.into(),
        key: k.into(),
        value: v,
        is_default,
    });
}

fn opt<T: ToString>(e: &mut Vec<ConfigEntry>, s: &str, k: &str, v: Option<T>, is_default: bool) {
    if let Some(v) = v {
        push(e, s, k, v.to_string(), is_default);
    }
}

impl From<StratumConfig<Parsed>> for StratumConfig<Raw> {
    fn from(parsed: StratumConfig<Parsed>) -> Self {
        StratumConfig {
            hostname: parsed.hostname,
            port: parsed.port,
            start_difficulty: parsed.start_difficulty,
            minimum_difficulty: parsed.minimum_difficulty,
            maximum_difficulty: parsed.maximum_difficulty,
            solo_address: parsed.solo_address,
            zmqpubhashblock: parsed.zmqpubhashblock,
            bootstrap_address: parsed.bootstrap_address,
            donation_address: parsed.donation_address,
            donation: parsed.donation,
            fee_address: parsed.fee_address,
            fee: parsed.fee,
            network: parsed.network,
            version_mask: parsed.version_mask,
            difficulty_multiplier: parsed.difficulty_multiplier,
            ignore_difficulty: parsed.ignore_difficulty,
            pool_signature: parsed.pool_signature,
            bootstrap_address_parsed: None,
            donation_address_parsed: None,
            fee_address_parsed: None,
            _state: PhantomData,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    fn write_cfg(txt: &str) -> (std::path::PathBuf, tempfile::TempDir) {
        let dir = tempdir().unwrap();
        let path = dir.path().join("p2pool.toml");
        let mut f = File::create(&path).unwrap();
        f.write_all(txt.as_bytes()).unwrap();
        (path, dir)
    }

    #[test]
    fn loads_and_flattens_full_config() {
        let (path, _dir) = write_cfg(
            r#"
[network]
listen_address = "/ip4/127.0.0.1/tcp/6884"
dial_peers = ["p1", "p2"]

[store]
path = "./store.db"
background_task_frequency_hours = 24
pplns_ttl_days = 7

[stratum]
hostname = "0.0.0.0"
port = 3333
start_difficulty = 10000
minimum_difficulty = 100
maximum_difficulty = 1000000
solo_address = "tb1qyazxde6558qj6z3d9np5e6msmrspwpf6k0qggk"
bootstrap_address = "tb1qyazxde6558qj6z3d9np5e6msmrspwpf6k0qggk"
donation_address = "tb1qw508d6qejxtdg4y5r3zarvary0c5xw7kxpjzsx"
donation = 100
fee_address = "tb1qyazxde6558qj6z3d9np5e6msmrspwpf6k0qggk"
fee = 50
zmqpubhashblock = "tcp://127.0.0.1:28332"
network = "signet"
version_mask = "1fffe000"
difficulty_multiplier = 1.0
ignore_difficulty = true
pool_signature = "TestPool"

[miner]
pubkey = "020202020202020202020202020202020202020202020202020202020202020202"

[bitcoinrpc]
url = "http://127.0.0.1:38332"
username = "user"
password = "pass"

[logging]
file = "./logs/p2pool.log"
level = "debug"
stats_dir = "./logs/stats"

[api]
hostname = "127.0.0.1"
port = 46884
auth_user = "admin"
auth_token = "token"
"#,
        );

        let entries = parse_config(&path).unwrap();

        assert!(entries.iter().any(|x| x.section == "network"
            && x.key == "listen_address"
            && x.value == "/ip4/127.0.0.1/tcp/6884"
            && !x.is_default));
        assert!(
            entries
                .iter()
                .any(|x| x.section == "stratum" && x.key == "hostname" && !x.is_default)
        );
        assert!(
            entries
                .iter()
                .any(|x| x.section == "stratum" && x.key == "donation" && x.value == "100 bp (1%)")
        );
        assert!(
            entries
                .iter()
                .any(|x| x.section == "bitcoinrpc" && x.key == "password" && x.value == "*****")
        );
        assert!(
            entries
                .iter()
                .any(|x| x.section == "api" && x.key == "auth_token" && x.value == "*****")
        );
        assert!(
            entries
                .iter()
                .any(|x| x.section == "stratum" && x.key == "network" && x.value == "signet")
        );
        assert!(
            entries.iter().any(|x| x.section == "stratum"
                && x.key == "version_mask"
                && x.value == "1fffe000")
        );
    }

    #[test]
    fn invalid_address_fails() {
        let (path, _dir) = write_cfg(
            r#"
[stratum]
hostname = "0.0.0.0"
port = 3333
start_difficulty = 10000
minimum_difficulty = 100
bootstrap_address = "invalid"
zmqpubhashblock = "tcp://127.0.0.1:28332"
network = "signet"
version_mask = "1fffe000"
difficulty_multiplier = 1.0

[store]
path = "./store.db"

[bitcoinrpc]
url = "http://127.0.0.1:38332"
username = "p2pool"
password = "p2pool"

[api]
hostname = "127.0.0.1"
port = 46884
"#,
        );

        let err = parse_config(&path).unwrap_err();
        assert!(err.to_string().contains("Invalid bootstrap_address"));
    }

    #[test]
    fn pool_signature_too_long_fails() {
        let (path, _dir) = write_cfg(
            r#"
[stratum]
hostname = "0.0.0.0"
port = 3333
start_difficulty = 10000
minimum_difficulty = 100
bootstrap_address = "tb1qyazxde6558qj6z3d9np5e6msmrspwpf6k0qggk"
zmqpubhashblock = "tcp://127.0.0.1:28332"
network = "signet"
version_mask = "1fffe000"
difficulty_multiplier = 1.0
pool_signature = "ThisIsWayTooLongForASignature"

[store]
path = "./store.db"

[bitcoinrpc]
url = "http://127.0.0.1:38332"
username = "p2pool"
password = "p2pool"

[api]
hostname = "127.0.0.1"
port = 46884
"#,
        );

        let err = parse_config(&path).unwrap_err();
        assert!(
            err.to_string()
                .contains("Pool signature exceeds max length")
        );
    }

    #[test]
    fn env_var_override_works() {
        unsafe { std::env::set_var("P2POOL_STRATUM_PORT", "9999") };

        let (path, _dir) = write_cfg(
            r#"
[stratum]
hostname = "0.0.0.0"
port = 3333
start_difficulty = 10000
minimum_difficulty = 100
bootstrap_address = "tb1qyazxde6558qj6z3d9np5e6msmrspwpf6k0qggk"
zmqpubhashblock = "tcp://127.0.0.1:28332"
network = "signet"
version_mask = "1fffe000"
difficulty_multiplier = 1.0

[store]
path = "./store.db"

[bitcoinrpc]
url = "http://127.0.0.1:38332"
username = "p2pool"
password = "p2pool"

[api]
hostname = "127.0.0.1"
port = 46884
"#,
        );

        let entries = parse_config(&path).unwrap();
        assert!(
            entries
                .iter()
                .any(|x| x.section == "stratum" && x.key == "port" && x.value == "9999")
        );

        unsafe { std::env::remove_var("P2POOL_STRATUM_PORT") };
    }

    #[test]
    fn non_p2pool_file_fails() {
        let (path, _dir) = write_cfg(
            r#"
foo = "bar"
answer = 42
"#,
        );

        let err = parse_config(&path).unwrap_err();
        assert!(err.to_string().contains("Invalid P2Pool config"));
    }

    #[test]
    fn wrong_network_address_is_rejected() {
        // bc1... is a MAINNET address, but network is set to signet
        let (path, _dir) = write_cfg(
            r#"
[stratum]
network = "signet"
bootstrap_address = "bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kygt080"
version_mask = "1fffe000"
zmqpubhashblock = "tcp://127.0.0.1:28332"

[store]
path = "./store.db"

[bitcoinrpc]
url = "http://127.0.0.1:38332"
username = "p2pool"
password = "p2pool"

[api]
hostname = "127.0.0.1"
port = 46884
"#,
        );

        let err = parse_config(&path).unwrap_err();

        assert!(
            err.to_string().contains("Invalid bootstrap_address"),
            "expected wrong-network address to be rejected, got: {err}"
        );
    }

    #[test]
    fn minimal_config_uses_defaults() {
        // ensures Serde defaults + flattening actually work together
        let (path, _dir) = write_cfg(
            r#"
[stratum]
network = "signet"
version_mask = "1fffe000"
zmqpubhashblock = "tcp://127.0.0.1:28332"
"#,
        );

        let entries = parse_config(&path).unwrap();

        assert!(entries.iter().any(|e| e.key == "port" && e.value == "3333"));
        assert!(
            entries
                .iter()
                .any(|e| e.key == "minimum_difficulty" && e.value == "100")
        );
    }

    #[test]
    fn donation_without_address_fails() {
        let (path, _dir) = write_cfg(
            r#"
[stratum]
donation = 100
network = "signet"
version_mask = "1fffe000"
zmqpubhashblock = "tcp://127.0.0.1:28332"
"#,
        );

        let err = parse_config(&path).unwrap_err();
        assert!(err.to_string().contains("donation_address is required"));
    }

    #[test]
    fn fee_without_address_fails() {
        let (path, _dir) = write_cfg(
            r#"
[stratum]
fee = 50
network = "signet"
version_mask = "1fffe000"
zmqpubhashblock = "tcp://127.0.0.1:28332"
"#,
        );

        let err = parse_config(&path).unwrap_err();
        assert!(err.to_string().contains("fee_address is required"));
    }

    #[test]
    fn parsed_to_raw_preserves_all_fields() {
        let parsed = StratumConfig::<Parsed> {
            hostname: "0.0.0.0".into(),
            port: 3333,
            start_difficulty: 10000,
            minimum_difficulty: 100,
            maximum_difficulty: Some(1_000_000),
            solo_address: Some("tb1qyazxde6558qj6z3d9np5e6msmrspwpf6k0qggk".into()),
            zmqpubhashblock: "tcp://127.0.0.1:28332".into(),
            bootstrap_address: Some("tb1qyazxde6558qj6z3d9np5e6msmrspwpf6k0qggk".into()),
            donation_address: Some("tb1qw508d6qejxtdg4y5r3zarvary0c5xw7kxpjzsx".into()),
            donation: Some(100),
            fee_address: Some("tb1qyazxde6558qj6z3d9np5e6msmrspwpf6k0qggk".into()),
            fee: Some(50),
            network: Network::Signet,
            version_mask: 0x1fffe000,
            difficulty_multiplier: 1.0,
            ignore_difficulty: Some(true),
            pool_signature: Some("TestPool".into()),
            bootstrap_address_parsed: None,
            donation_address_parsed: None,
            fee_address_parsed: None,
            _state: PhantomData,
        };

        let raw: StratumConfig<Raw> = parsed.clone().into();

        // Explicit field-by-field assertions
        assert_eq!(raw.hostname, parsed.hostname);
        assert_eq!(raw.port, parsed.port);
        assert_eq!(raw.start_difficulty, parsed.start_difficulty);
        assert_eq!(raw.minimum_difficulty, parsed.minimum_difficulty);
        assert_eq!(raw.maximum_difficulty, parsed.maximum_difficulty);
        assert_eq!(raw.solo_address, parsed.solo_address);
        assert_eq!(raw.zmqpubhashblock, parsed.zmqpubhashblock);
        assert_eq!(raw.bootstrap_address, parsed.bootstrap_address);
        assert_eq!(raw.donation_address, parsed.donation_address);
        assert_eq!(raw.donation, parsed.donation);
        assert_eq!(raw.fee_address, parsed.fee_address);
        assert_eq!(raw.fee, parsed.fee);
        assert_eq!(raw.network, parsed.network);
        assert_eq!(raw.version_mask, parsed.version_mask);
        assert_eq!(raw.difficulty_multiplier, parsed.difficulty_multiplier);
        assert_eq!(raw.ignore_difficulty, parsed.ignore_difficulty);
        assert_eq!(raw.pool_signature, parsed.pool_signature);
    }
}
