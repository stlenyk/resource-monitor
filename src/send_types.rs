use std::time::Duration;

use serde::{Deserialize, Serialize};

#[derive(Clone, Default, Serialize, Deserialize, Debug)]
pub struct SystemUtilization {
    pub cpus: Vec<CpuCore>,
    pub mem: u64,
    pub mem_max: u64,
    pub gpus: Vec<Gpu>,
    pub up_time: Duration,
    pub processes: u32,
    pub network_throughput: NewtorkThroughput
}

#[derive(Clone, Default, Serialize, Deserialize, Debug)]
pub struct CpuCore {
    pub usage: f32,
    pub freq: u64,
}

#[derive(Clone, Default, Serialize, Deserialize, Debug)]
pub struct NewtorkThroughput {
    pub download: u32,
    pub upload: u32,
}

impl NewtorkThroughput {
    pub fn new() -> Self {
        Self {
            download: 0,
            upload: 0,
        }
    }
}

#[derive(Clone, Default, Serialize, Deserialize, Debug)]
pub struct Gpu {
    pub usage: u32,
    pub mem: u32,
    pub max_mem: u64,
    pub temp: u32,
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
