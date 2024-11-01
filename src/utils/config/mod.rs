use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Read;

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub server: ServerConfig,
    pub raft: RaftConfig,
    pub storage: StorageConfig
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ServerConfig {
    pub raft_port: u16,
    pub outbound_port: u16
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RaftConfig {
    pub tick_millis_duration: u64,
    pub leader_seen_timeout: u64,
    pub candidate_election_timeout: u64,
    pub candidate_election_timeout_rand: u64,
    pub leader_idle_timeout: u64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct StorageConfig {
    pub max_memtable_size: usize,
    pub data_path: String
}

pub fn load_config() -> Result<Config, Box<dyn std::error::Error>> {
    let mut file = File::open("config.yaml")?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    let config: Config = serde_yaml::from_str(&content)?;
    Ok(config)
}

lazy_static! {
    pub static ref CONFIG: Config = load_config().expect("Failed to load config");
}
