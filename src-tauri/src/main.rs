// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use shared::*;

use std::{
    process,
    sync::{Mutex, MutexGuard, OnceLock, PoisonError},
    time::{Duration, Instant},
};

use nvml_wrapper::{enum_wrappers::device::TemperatureSensor, Nvml};

struct SystemMonitor {
    nvml: Option<Nvml>,
    sys: sysinfo::System,
    sys_info: SystemInfo,
    networks: sysinfo::Networks,
    last_update: Instant,
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
        let sys = sysinfo::System::new_all();

        let cpu_brand = sys
            .cpus()
            .first()
            .map_or("", sysinfo::Cpu::brand)
            .to_owned();
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

        let (cache_l1, cache_l2, cache_l3) = {
            #[cfg(all(target_arch = "x86_64", not(target_env = "sgx")))]
            {
                let cpuid = raw_cpuid::CpuId::new();
                let l1 = cpuid
                    .get_l1_cache_and_tlb_info()
                    .map(|info| info.dcache_size());
                let (l2, l3) = cpuid
                    .get_l2_l3_cache_and_tlb_info()
                    .map_or((None, None), |info| {
                        (Some(info.l2cache_size()), Some(info.l3cache_size()))
                    });
                (l1, l2, l3)
            }
            #[cfg(not(all(target_arch = "x86_64", not(target_env = "sgx"))))]
            {
                (None, None, None)
            }
        };

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
            networks: sysinfo::Networks::new_with_refreshed_list(),
            last_update: Instant::now(),
        }
    }

    fn get_stats(&mut self) -> SystemUtilization {
        self.sys.refresh_cpu_specifics(
            sysinfo::CpuRefreshKind::new()
                .with_cpu_usage()
                .with_frequency(),
        );
        self.sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
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
        let up_time = Duration::from_secs(sysinfo::System::uptime());

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

        let disk = {
            let (read, written) =
                self.sys
                    .processes()
                    .iter()
                    .fold((0, 0), |(read, write), (_pid, proc)| {
                        (
                            read + proc.disk_usage().read_bytes,
                            write + proc.disk_usage().written_bytes,
                        )
                    });
            Disk {
                read_bytes: read,
                writen_bytes: written,
            }
        };

        let network = {
            self.networks.refresh();
            let (down, up) = self
                .networks
                .iter()
                .fold((0, 0), |(down, up), (_inteface, data)| {
                    (down + data.received(), up + data.transmitted())
                });

            let update_time = Instant::now();
            let duration = update_time.duration_since(self.last_update);
            self.last_update = update_time;
            Network {
                down: (down as f32 / duration.as_secs_f32()) as u64,
                up: (up as f32 / duration.as_secs_f32()) as u64,
            }
        };

        SystemUtilization {
            cpus,
            mem,
            processes,
            mem_max,
            disk,
            gpus,
            up_time,
            network,
        }
    }
}

use clap::{CommandFactory, Parser};
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct CliArgs {
    #[arg(long, default_value_t = false, help = "Start minimized to tray")]
    minimize: bool,
    #[arg(long, help = "Set window width")]
    width: Option<u32>,
    #[arg(long, help = "Set window height")]
    height: Option<u32>,
    #[arg(long, help = "Generate shell completions")]
    completions: Option<clap_complete::Shell>,
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
    menu::{MenuBuilder, MenuItem, MenuItemBuilder},
    AppHandle, Manager, PhysicalSize, RunEvent, Size, WindowEvent, Wry,
};

fn show_window(app: &AppHandle) {
    let window = app.get_webview_window(WINDOW_ID).unwrap();
    window.show().unwrap();
    window.set_focus().unwrap();
    TRAY_HIDE.get().unwrap().set_enabled(true).unwrap();
}
fn hide_window(app: &AppHandle) {
    app.get_webview_window(WINDOW_ID).unwrap().hide().unwrap();
    TRAY_HIDE.get().unwrap().set_enabled(false).unwrap();
}

const WINDOW_ID: &str = "main";
static TRAY_SHOW: OnceLock<MenuItem<Wry>> = OnceLock::new();
static TRAY_HIDE: OnceLock<MenuItem<Wry>> = OnceLock::new();
static TRAY_QUIT: OnceLock<MenuItem<Wry>> = OnceLock::new();
const TRAY_SHOW_ID: &str = "show";
const TRAY_HIDE_ID: &str = "hide";
const TRAY_QUIT_ID: &str = "quit";

fn main() {
    let cli_args = CliArgs::parse();

    if let Some(shell) = cli_args.completions {
        clap_complete::generate(
            shell,
            &mut CliArgs::command(),
            env!("CARGO_PKG_NAME"),
            &mut std::io::stdout(),
        );
        return;
    }

    let builder = if cfg!(not(debug_assertions)) {
        tauri::Builder::default()
            .plugin(tauri_plugin_shell::init())
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
            TRAY_SHOW.get_or_init(|| {
                MenuItemBuilder::with_id(TRAY_SHOW_ID, "Show")
                    .build(app)
                    .unwrap()
            });
            TRAY_HIDE.get_or_init(|| {
                MenuItemBuilder::with_id(TRAY_HIDE_ID, "Hide")
                    .build(app)
                    .unwrap()
            });
            TRAY_QUIT.get_or_init(|| {
                MenuItemBuilder::with_id(TRAY_QUIT_ID, "Quit")
                    .build(app)
                    .unwrap()
            });

            let tray_menu = MenuBuilder::new(app)
                .item(TRAY_HIDE.get().unwrap())
                .item(TRAY_SHOW.get().unwrap())
                .separator()
                .item(TRAY_QUIT.get().unwrap())
                .build()
                .unwrap();
            tauri::tray::TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&tray_menu)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    TRAY_SHOW_ID => show_window(app),
                    TRAY_HIDE_ID => hide_window(app),
                    TRAY_QUIT_ID => process::exit(0),
                    _ => {}
                })
                .build(app)
                .unwrap();

            if cli_args.minimize {
                hide_window(app.app_handle());
            }

            if let Some(width) = cli_args.width {
                let window = app.get_webview_window(WINDOW_ID).unwrap();
                let height = window.inner_size().unwrap().height;
                let size = Size::Physical(PhysicalSize::new(width, height));
                window.set_size(size).unwrap();
            }
            if let Some(height) = cli_args.height {
                let window = app.get_webview_window(WINDOW_ID).unwrap();
                let width = window.inner_size().unwrap().width;
                let size = Size::Physical(PhysicalSize::new(width, height));
                window.set_size(size).unwrap();
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                hide_window(window.app_handle());
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
