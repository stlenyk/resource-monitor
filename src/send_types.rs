use std::time::Duration;

use serde::{Deserialize, Serialize};

#[derive(Clone, Default, Serialize, Deserialize, Debug)]
pub struct SystemUtilization {
    pub cpus: Vec<CpuCore>,
    pub mem: u64,
    pub mem_max: u64,
    pub disk: Disk,
    pub gpus: Vec<Gpu>,
    pub up_time: Duration,
    pub processes: u32,
    pub network: Network,
}

#[derive(Clone, Default, Serialize, Deserialize, Debug)]
pub struct CpuCore {
    pub usage: f32,
    pub freq: u64,
}

#[derive(Clone, Default, Serialize, Deserialize, Debug)]
pub struct Gpu {
    pub usage: u32,
    pub mem: u32,
    pub max_mem: u64,
    pub temp: u32,
}

#[derive(Clone, Default, Serialize, Deserialize, Debug)]
pub struct Disk {
    /// Read bytes
    pub read_bytes: u64,
    /// Written bytes
    pub writen_bytes: u64,
}

#[derive(Clone, Default, Serialize, Deserialize, Debug)]
pub struct Network {
    /// Download speed in bytes per second
    pub down: u64,
    /// Upload speed in bytes per second
    pub up: u64,
}

#[derive(Clone, Default, Serialize, Deserialize, Debug)]
pub struct SystemInfo {
    pub cpu_brand: String,
    pub cpu_core_count: u32,
    /// L1 data cache size in KB
    pub cache_l1: Option<u8>,
    pub cache_l2: Option<u16>,
    pub cache_l3: Option<u16>,
    pub max_mem: u64,
    pub gpu_count: u32,
    pub gpu_names: Vec<String>,
}
