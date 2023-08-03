use serde::{Deserialize, Serialize};

pub type UtilCPU = Vec<f32>;
pub type UtilMem = u64;
pub type UtilGPU = Option<Vec<f32>>;

#[derive(Clone, Default, Serialize, Deserialize, Debug)]

pub struct SystemUtilization {
    pub cpus: UtilCPU,
    pub mem: UtilMem,
    pub mem_max: UtilMem,
    pub gpus: UtilGPU,
}
