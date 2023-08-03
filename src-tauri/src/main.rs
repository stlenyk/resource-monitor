// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[path = "../../src/send_types.rs"]
mod send_types;
use send_types::SystemUtilization;

use std::sync::{Mutex, MutexGuard, PoisonError};

use nvml_wrapper::Nvml;
use sysinfo::{CpuExt, System, SystemExt};

struct SystemMonitor {
    nvml: Option<Nvml>,
    sys: System,
}

struct SystemMonitorState(Mutex<SystemMonitor>);

type SystemMonitorStateResult<'a> =
    Result<MutexGuard<'a, SystemMonitor>, PoisonError<MutexGuard<'a, SystemMonitor>>>;

impl SystemMonitorState {
    fn new() -> Self {
        Self(Mutex::new(SystemMonitor::new()))
    }
    fn get_state(&self) -> SystemMonitorStateResult {
        self.0.lock()
    }
}

impl SystemMonitor {
    fn new() -> Self {
        Self {
            nvml: Nvml::init().ok(),
            sys: System::new_all(),
        }
    }

    fn get_stats(&mut self) -> SystemUtilization {
        self.sys.refresh_all();
        let cpus = self.sys.cpus().iter().map(|cpu| cpu.cpu_usage()).collect();
        let mem = self.sys.used_memory();
        let mem_max = self.sys.total_memory();

        let gpus = if let Some(nvml) = &self.nvml {
            let mut gpus_usage = Vec::new();
            let device_count = nvml.device_count().unwrap();
            for gpu_idx in 0..device_count {
                let gpu = nvml.device_by_index(gpu_idx).unwrap();
                let util = gpu.utilization_rates().unwrap().gpu as f32 / 100_f32;
                gpus_usage.push(util);
            }
            Some(gpus_usage)
        } else {
            None
        };

        SystemUtilization {
            cpus,
            mem,
            mem_max,
            gpus,
        }
    }
}

#[tauri::command]
fn get_stats(state: tauri::State<SystemMonitorState>) -> SystemUtilization {
    state.get_state().unwrap().get_stats()
}

fn main() {
    tauri::Builder::default()
        .manage(SystemMonitorState::new())
        .invoke_handler(tauri::generate_handler![get_stats])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
