use std::time::Duration;

use derive_more::{Add, Div, DivAssign, Sum};
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

impl std::ops::Add for SystemUtilization {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            cpus: self
                .cpus
                .into_iter()
                .zip(rhs.cpus)
                .map(|(a, b)| a + b)
                .collect(),
            mem: self.mem + rhs.mem,
            mem_max: self.mem_max + rhs.mem_max,
            disk: self.disk + rhs.disk,
            gpus: self
                .gpus
                .into_iter()
                .zip(rhs.gpus)
                .map(|(a, b)| a + b)
                .collect(),
            up_time: self.up_time + rhs.up_time,
            processes: self.processes + rhs.processes,
            network: self.network + rhs.network,
        }
    }
}
impl std::iter::Sum for SystemUtilization {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self::default(), |a, b| a + b)
    }
}

#[derive(Clone, Default, Serialize, Deserialize, Debug, Add, Div, DivAssign, Sum)]
pub struct CpuCore {
    pub usage: f32,
    pub freq: u64,
}

#[derive(Clone, Default, Serialize, Deserialize, Debug, Add, Div, DivAssign, Sum)]
pub struct Gpu {
    pub usage: u32,
    pub mem: u32,
    pub max_mem: u64,
    pub temp: u32,
}

#[derive(Clone, Default, Serialize, Deserialize, Debug, Add, Div, DivAssign, Sum)]
pub struct Disk {
    /// Read bytes
    pub read_bytes: u64,
    /// Written bytes
    pub writen_bytes: u64,
}

#[derive(Clone, Default, Serialize, Deserialize, Debug, Add, Div, DivAssign, Sum)]
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
