// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[path = "../../src/send_types.rs"]
mod send_types;
use send_types::{CpuCore, Gpu, SystemInfo, SystemUtilization};

use std::{
    sync::{Mutex, MutexGuard, PoisonError},
    time::Duration,
};

use nvml_wrapper::{enum_wrappers::device::TemperatureSensor, Nvml};
use raw_cpuid::CpuId;
use sysinfo::{CpuExt, CpuRefreshKind, System, SystemExt};

struct SystemMonitor {
    nvml: Option<Nvml>,
    sys: System,
    sys_info: SystemInfo,
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
        let sys = System::new_all();
        let cpuid = CpuId::new();

        let cpu_brand = sys.cpus().get(0).map_or("", CpuExt::brand).to_owned();
        let cpu_core_count = sys.cpus().len() as u32;
        let max_mem = sys.total_memory();

        let (gpu_count, gpu_names) = if let Ok(nvml) = Nvml::init() {
            let gpu_count = nvml.device_count().unwrap_or(0);
            let gpu_names = (0..gpu_count)
                .map(|gpu_id| {
                    nvml.device_by_index(gpu_id)
                        .map_or("".to_owned(), |device| {
                            device.name().unwrap_or("".to_owned())
                        })
                })
                .collect();
            (gpu_count, gpu_names)
        } else {
            (0, Vec::new())
        };

        let cache_l1 = cpuid
            .get_l1_cache_and_tlb_info()
            .map(|info| info.dcache_size());
        let (cache_l2, cache_l3) = cpuid
            .get_l2_l3_cache_and_tlb_info()
            .map_or((None, None), |info| {
                (Some(info.l2cache_size()), Some(info.l3cache_size()))
            });

        let sys_info = SystemInfo {
            cpu_brand,
            cpu_core_count,
            cache_l1,
            cache_l2,
            cache_l3,
            max_mem,
            gpu_count,
            gpu_names,
        };

        Self {
            sys,
            nvml: Nvml::init().ok(),
            sys_info,
        }
    }

    fn get_stats(&mut self) -> SystemUtilization {
        self.sys
            .refresh_cpu_specifics(CpuRefreshKind::new().with_cpu_usage().with_frequency());
        self.sys.refresh_processes();
        self.sys.refresh_memory();
        let cpus = self
            .sys
            .cpus()
            .iter()
            .map(|cpu| CpuCore {
                usage: cpu.cpu_usage(),
                freq: cpu.frequency(),
            })
            .collect();
        let processes = self.sys.processes().len() as u32;
        let mem = self.sys.used_memory();
        let mem_max = self.sys.total_memory();
        let up_time = Duration::from_secs(self.sys.uptime());

        let gpus = if let Some(nvml) = &self.nvml {
            let mut gpus_util = Vec::new();
            let device_count = nvml.device_count().unwrap_or(0);
            for gpu_idx in 0..device_count {
                let (usage, mem, max_mem, temp) = if let Ok(gpu) = nvml.device_by_index(gpu_idx) {
                    let (util, mem) = gpu
                        .utilization_rates()
                        .map_or((0, 0), |util| (util.gpu, util.memory));
                    let temp = gpu.temperature(TemperatureSensor::Gpu).unwrap_or(0);
                    let max_mem = gpu.memory_info().map_or(0, |mem_info| mem_info.total);
                    (util, mem, max_mem, temp)
                } else {
                    (0, 0, 0, 0)
                };
                let gpu_util = Gpu {
                    usage,
                    mem,
                    max_mem,
                    temp,
                };
                gpus_util.push(gpu_util);
            }
            gpus_util
        } else {
            vec![]
        };

        SystemUtilization {
            cpus,
            mem,
            processes,
            mem_max,
            gpus,
            up_time,
        }
    }
}

#[tauri::command]
fn get_stats(state: tauri::State<SystemMonitorState>) -> SystemUtilization {
    state.get_state().unwrap().get_stats()
}

#[tauri::command]
fn get_sys_info(state: tauri::State<SystemMonitorState>) -> SystemInfo {
    state.get_state().unwrap().sys_info.clone()
}

use tauri::{
    CustomMenuItem, Manager, RunEvent, SystemTray, SystemTrayEvent, SystemTrayMenu,
    SystemTrayMenuItem, WindowEvent,
};

fn main() {
    let quit = CustomMenuItem::new("quit".to_string(), "Quit");
    let hide = CustomMenuItem::new("hide".to_string(), "Hide");
    let show = CustomMenuItem::new("show".to_string(), "Show");
    let tray_menu = SystemTrayMenu::new()
        .add_item(quit)
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(show)
        .add_item(hide);
    let tray = SystemTray::new().with_menu(tray_menu);

    const WINDOW_ID: &str = "main";

    #[allow(clippy::single_match)]
    tauri::Builder::default()
        // Breaks `cargo tauri dev` reload
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            app.get_window(WINDOW_ID).unwrap().show().unwrap();
        }))
        .manage(SystemMonitorState::new())
        .system_tray(tray)
        .on_system_tray_event(|app, event| match event {
            SystemTrayEvent::MenuItemClick { id, .. } => match id.as_str() {
                "quit" => {
                    std::process::exit(0);
                }
                "hide" => {
                    app.get_window(WINDOW_ID).unwrap().hide().unwrap();
                }
                "show" => {
                    app.get_window(WINDOW_ID).unwrap().show().unwrap();
                }
                _ => {}
            },
            _ => {}
        })
        .on_window_event(|event| {
            if let WindowEvent::CloseRequested { api, .. } = event.event() {
                event.window().hide().unwrap();
                api.prevent_close();
            }
        })
        .invoke_handler(tauri::generate_handler![get_stats, get_sys_info])
        .build(tauri::generate_context!())
        .expect("error while running tauri application")
        .run(|_app_handle, event| {
            if let RunEvent::ExitRequested { api, .. } = event {
                api.prevent_exit();
            }
        });
}
