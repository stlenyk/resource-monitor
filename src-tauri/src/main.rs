// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[path = "../../src/send_types.rs"]
mod send_types;
use send_types::{CpuCore, Gpu, NewtorkThroughput, SystemInfo, SystemUtilization};

use std::{
    sync::{Arc, Mutex, MutexGuard, PoisonError},
    thread,
    time::{Duration, Instant},
};

use nvml_wrapper::{enum_wrappers::device::TemperatureSensor, Nvml};
use raw_cpuid::CpuId;
use sysinfo::{CpuExt, CpuRefreshKind, System, SystemExt};

struct SystemMonitor {
    nvml: Option<Nvml>,
    sys: System,
    sys_info: SystemInfo,
    network: Arc<Mutex<NewtorkThroughput>>,
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
        sudo::escalate_if_needed().unwrap();
        let interface = pcap::Device::lookup().unwrap().unwrap();
        let mut cap = pcap::Capture::from_device(interface)
            .unwrap()
            .promisc(true)
            .snaplen(5000)
            .open()
            .unwrap();
        let network = Arc::new(Mutex::new(NewtorkThroughput::new()));
        let network_thread = network.clone();

        thread::spawn(move || {
            let my_mac_addr = mac_address::get_mac_address().unwrap().unwrap().bytes();
            let mut t0 = Instant::now();
            let (mut download, mut upload) = (0, 0);
            loop {
                if let Ok(packet) = cap.next_packet() {
                    if let Some(ethernet_packet) =
                        pnet::packet::ethernet::EthernetPacket::new(packet.data)
                    {
                        let dst_ip = ethernet_packet.get_destination();
                        let dst_ip = [dst_ip.0, dst_ip.1, dst_ip.2, dst_ip.3, dst_ip.4, dst_ip.5];
                        if dst_ip == my_mac_addr {
                            download += packet.header.len;
                        } else {
                            upload += packet.header.len;
                        }
                    }
                }

                // TODO this is hardcoded and doesn't repsond to changes in the frontend
                // TODO 2: account for elapsed >> 1s
                if t0.elapsed() >= Duration::from_millis(1000) {
                    t0 = Instant::now();
                    let mut network = network_thread.lock().unwrap();
                    network.download = download;
                    network.upload = upload;
                    (download, upload) = (0, 0);
                }
            }
        });

        let sys = System::new_all();
        let cpuid = CpuId::new();

        let cpu_brand = sys.cpus().first().map_or("", CpuExt::brand).to_owned();
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
            network,
        }
    }

    fn get_stats(&mut self) -> SystemUtilization {
        // println!("{:?}", self.network);
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

        let network_throughput = self.network.lock().unwrap().clone();
        SystemUtilization {
            cpus,
            mem,
            processes,
            mem_max,
            gpus,
            up_time,
            network_throughput,
        }
    }
}

use clap::Parser;
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct CliArgs {
    #[arg(short, long, default_value_t = false, help = "Start minimized to tray")]
    minimize: bool,
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
    AppHandle, CustomMenuItem, Manager, RunEvent, SystemTray, SystemTrayEvent, SystemTrayMenu,
    SystemTrayMenuItem, WindowEvent,
};

const WINDOW_ID: &str = "main";
const TRAY_QUIT: &str = "quit";
const TRAY_HIDE: &str = "hide";
const TRAY_SHOW: &str = "show";

fn show_window(app: &AppHandle) {
    let window = app.get_window(WINDOW_ID).unwrap();
    window.show().unwrap();
    window.set_focus().unwrap();
    app.tray_handle()
        .get_item(TRAY_HIDE)
        .set_enabled(true)
        .unwrap();
}
fn hide_window(app: &AppHandle) {
    app.get_window(WINDOW_ID).unwrap().hide().unwrap();
    app.tray_handle()
        .get_item(TRAY_HIDE)
        .set_enabled(false)
        .unwrap();
}

fn main() {
    let args = CliArgs::parse();

    let tray_menu = SystemTrayMenu::new()
        .add_item(CustomMenuItem::new(TRAY_QUIT, "Quit"))
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(CustomMenuItem::new(TRAY_HIDE, "Hide"))
        .add_item(CustomMenuItem::new(TRAY_SHOW, "Show"));
    let tray = SystemTray::new().with_menu(tray_menu);

    let builder = if cfg!(not(debug_assertions)) {
        tauri::Builder::default()
            // This plugin breaks `cargo tauri dev` reload
            .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
                show_window(app);
            }))
    } else {
        tauri::Builder::default()
    };

    #[allow(clippy::single_match)]
    builder
        .manage(SystemMonitorState::new())
        .setup(move |app| {
            if args.minimize {
                hide_window(&app.app_handle());
            }
            Ok(())
        })
        .system_tray(tray)
        .on_system_tray_event(|app, event| match event {
            SystemTrayEvent::MenuItemClick { id, .. } => match id.as_str() {
                TRAY_QUIT => std::process::exit(0),
                TRAY_HIDE => hide_window(app),
                TRAY_SHOW => show_window(app),
                _ => {}
            },
            _ => {}
        })
        .on_window_event(|event| {
            if let WindowEvent::CloseRequested { api, .. } = event.event() {
                hide_window(&event.window().app_handle());
                api.prevent_close();
            }
        })
        .invoke_handler(tauri::generate_handler![get_stats, get_sys_info])
        .build(tauri::generate_context!())
        .expect("error while running tauri application")
        .run(|_app, event| {
            if let RunEvent::ExitRequested { api, .. } = event {
                api.prevent_exit();
            }
        });
}
