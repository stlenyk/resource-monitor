use crate::send_types::*;

use std::{collections::VecDeque, time::Duration};

use leptos::*;
use plotly::{
    bindings::react,
    color::{Rgb, Rgba},
    common::{AxisSide, Fill, Marker, Title},
    layout::{Axis, Margin},
    Configuration, Layout, Plot, Scatter,
};
use serde::Serialize;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "tauri"])]
    async fn invoke(cmd: &str, args: JsValue) -> JsValue;
}

// Assumes that the number of cpus doesn't change and may panic otherwise.
fn plot_cpu(sys_util_history: &[SystemUtilization], max_history: usize) -> Plot {
    let mut plot = Plot::new();

    let config = Configuration::new().static_plot(true).responsive(true);
    plot.set_configuration(config);
    let layout = Layout::new().auto_size(true);
    plot.set_layout(layout);

    let cpu_history = sys_util_history
        .iter()
        .map(|util| util.cpus.clone())
        .collect::<Vec<_>>();

    if let Some(history_point) = cpu_history.first() {
        let cpu_count = history_point.len();
        let mut traces: Vec<Vec<f32>> = vec![Vec::new(); cpu_count];

        for history_point in &cpu_history {
            for (id, cpu) in history_point.iter().enumerate() {
                traces[id].push(cpu.usage / cpu_count as f32);
            }
        }
        let lower_bound = (max_history - cpu_history.len()).max(0);
        let x = (lower_bound..max_history).collect::<Vec<_>>();
        let stack_group = "stack_group";
        let colors = [
            Rgb::new(74, 85, 162),
            Rgb::new(120, 149, 203),
            Rgb::new(160, 191, 224),
            Rgb::new(197, 223, 248),
            Rgb::new(160, 191, 224),
            Rgb::new(120, 149, 203),
        ];
        for (i, y) in traces.iter().enumerate() {
            let color = colors[i % colors.len()];
            let trace = Scatter::new(x.clone(), y.clone())
                // Line smoothing
                // .line(
                //     plotly::common::Line::new()
                //         .shape(plotly::common::LineShape::Spline)
                //         // between 0.0 and 1.3
                //         .smoothing(1.3),
                // )
                .show_legend(false)
                .stack_group(stack_group)
                .marker(Marker::new().color(color));
            plot.add_trace(trace);
        }
    }
    plot
}

fn plot_generic_many<T: Clone + Serialize + 'static>(
    values: &[Vec<T>],
    max_history: usize,
    colors: &[Rgb],
    fill: Fill,
) -> Plot {
    let mut plot = Plot::new();
    let config = Configuration::new().static_plot(true);
    plot.set_configuration(config);
    let layout = Layout::new().auto_size(true);
    plot.set_layout(layout);

    let lower_bound = (max_history - values[0].len()).max(0);
    let x = (lower_bound..max_history).collect::<Vec<_>>();
    for (i, y) in values.iter().enumerate() {
        let color = colors[i % colors.len()];
        let trace = Scatter::new(x.clone(), y.clone())
            .show_legend(false)
            .marker(Marker::new().color(color).size(1))
            .fill(fill.clone());

        plot.add_trace(trace);
    }

    plot
}

fn plot_mem(sys_util_history: &[SystemUtilization], max_history: usize) -> Plot {
    let plot_values = sys_util_history.iter().map(|util| util.mem).collect();
    let color = Rgb::new(101, 39, 190);
    // plot_generic(plot_values, max_history, color);
    plot_generic_many(&[plot_values], max_history, &[color], Fill::ToZeroY)
}

fn plot_gpu(sys_util_history: &[SystemUtilization], max_history: usize, gpu_id: usize) -> Plot {
    let plot_values = sys_util_history
        .iter()
        .map(|util| util.gpus[gpu_id].usage)
        .collect();
    plot_generic_many(
        &[plot_values],
        max_history,
        &[Rgb::new(120, 149, 203)],
        Fill::ToZeroY,
    )
}

const COLOR_READ: (u8, u8, u8) = (0, 128, 43);
const COLOR_WRITE: (u8, u8, u8) = (120, 149, 203);
const COLOR_READ_HTML: &str = const_format::formatcp!(
    "color: rgb({}, {}, {})",
    COLOR_READ.0,
    COLOR_READ.1,
    COLOR_READ.2
);
const COLOR_WRITE_HTML: &str = const_format::formatcp!(
    "color: rgb({}, {}, {})",
    COLOR_WRITE.0,
    COLOR_WRITE.1,
    COLOR_WRITE.2
);

fn plot_disk(sys_util_history: &[SystemUtilization], max_history: usize) -> Plot {
    let read = sys_util_history
        .iter()
        .map(|util| util.disk.read_bytes)
        .collect();
    let write = sys_util_history
        .iter()
        .map(|util| util.disk.writen_bytes)
        .collect();
    plot_generic_many(
        &[read, write],
        max_history,
        &[
            Rgb::new(COLOR_READ.0, COLOR_READ.1, COLOR_READ.2),
            Rgb::new(COLOR_WRITE.0, COLOR_WRITE.1, COLOR_WRITE.2),
        ],
        Fill::None,
    )
}

const COLOR_DOWN: (u8, u8, u8) = (0, 128, 43);
const COLOR_UP: (u8, u8, u8) = (120, 149, 203);
const COLOR_DOWN_HTML: &str = const_format::formatcp!(
    "color: rgb({}, {}, {})",
    COLOR_DOWN.0,
    COLOR_DOWN.1,
    COLOR_DOWN.2
);
const COLOR_UP_HTML: &str =
    const_format::formatcp!("color: rgb({}, {}, {})", COLOR_UP.0, COLOR_UP.1, COLOR_UP.2);

fn plot_network(sys_util_history: &[SystemUtilization], max_history: usize) -> Plot {
    let down = sys_util_history
        .iter()
        .map(|util| util.network.down)
        .collect();
    let up = sys_util_history
        .iter()
        .map(|util| util.network.up)
        .collect();
    let colors = [
        Rgb::new(COLOR_DOWN.0, COLOR_DOWN.1, COLOR_DOWN.2),
        Rgb::new(COLOR_UP.0, COLOR_UP.1, COLOR_UP.2),
    ];

    plot_generic_many(&[down, up], max_history, &colors, Fill::None)
}

#[component]
fn PlotCpuMini(
    sys_util_history: Signal<Vec<SystemUtilization>>,
    max_history: ReadSignal<usize>,
) -> impl IntoView {
    let div_id = "side-cpu";
    create_effect(move |_| {
        let mut plot = plot_cpu(&sys_util_history.get(), max_history.get());

        let y_ticks = vec![0.0, 20.0, 40.0, 60.0, 80.0, 100.0];
        let y_axis = Axis::new().range(vec![0, 100]).tick_values(y_ticks);
        let x_axis = Axis::new()
            .range(vec![0, max_history.get() - 1])
            .tick_values(vec![]);
        let margin = Margin::new().left(0).right(0).top(0).bottom(0);
        let layout = plot
            .layout()
            .clone()
            .margin(margin)
            .y_axis(y_axis)
            .x_axis(x_axis);
        plot.set_layout(layout);

        spawn_local(async move {
            react(div_id, &plot).await;
        });
    });

    view! { <div class="leftmini" id=div_id></div> }
}

#[component]
fn PlotMemMini(
    sys_util_history: Signal<Vec<SystemUtilization>>,
    max_history: ReadSignal<usize>,
) -> impl IntoView {
    let div_id = "side-mem";
    create_effect(move |_| {
        let max_history = max_history.get();

        let mut plot = plot_mem(&sys_util_history.get(), max_history);

        let max_mem = if let Some(sys_util) = sys_util_history.get().first() {
            sys_util.mem_max
        } else {
            0
        };
        let y_axis = Axis::new().range(vec![0, max_mem]);
        let x_axis = Axis::new()
            .range(vec![0, max_history - 1])
            .tick_values(vec![]);
        let margin = Margin::new().left(0).right(0).top(0).bottom(0);
        let layout = plot
            .layout()
            .clone()
            .margin(margin)
            .y_axis(y_axis)
            .x_axis(x_axis);
        plot.set_layout(layout);

        spawn_local(async move {
            react(div_id, &plot).await;
        });
    });

    view! { <div class="leftmini" id=div_id></div> }
}

#[component]
fn PlotGpusMini(
    sys_util_history: Signal<Vec<SystemUtilization>>,
    max_history: ReadSignal<usize>,
    main_view: WriteSignal<MainView>,
) -> impl IntoView {
    view! {
        <For
            each=move || 0..sys_util_history.get().first().map_or(0, |sys_util| sys_util.gpus.len())
            key=|gpu_id| *gpu_id
            children=move |gpu_id| {
                let div_id = format!("side-gpu-{}", gpu_id);
                {
                    let div_id = div_id.clone();
                    create_effect(move |_| {
                        let max_history = max_history.get();
                        let mut plot = plot_gpu(&sys_util_history.get(), max_history, gpu_id);
                        let y_ticks = vec![0.0, 20.0, 40.0, 60.0, 80.0, 100.0];
                        let y_axis = Axis::new().range(vec![0, 100]).tick_values(y_ticks);
                        let x_axis = Axis::new()
                            .range(vec![0, max_history - 1])
                            .tick_values(vec![]);
                        let margin = Margin::new().left(0).right(0).top(0).bottom(0);
                        let layout = plot
                            .layout()
                            .clone()
                            .margin(margin)
                            .y_axis(y_axis)
                            .x_axis(x_axis);
                        plot.set_layout(layout);
                        let div_id = div_id.clone();
                        spawn_local(async move {
                            react(&div_id, &plot).await;
                        });
                    });
                }
                let gpu_descr = move || {
                    if let Some(last) = sys_util_history.get().last() {
                        let gpu = last.gpus[gpu_id].clone();
                        format!("{}% ({} ℃)", gpu.usage, gpu.temp)
                    } else {
                        String::new()
                    }
                };
                view! {
                    <button on:click=move |_| { main_view.set(MainView::Gpu(gpu_id)) }>
                        <div class="leftmini" id=div_id></div>
                        <div class="rightmini">
                            <div class="rightminititle">{format!("GPU {}", gpu_id)}</div>
                            {gpu_descr}
                        </div>
                    </button>
                }
            }
        />
    }
}

#[component]
fn PlotDiskMini(
    sys_util_history: Signal<Vec<SystemUtilization>>,
    max_history: ReadSignal<usize>,
) -> impl IntoView {
    let div_id = "side-disk";
    create_effect(move |_| {
        let mut plot = plot_disk(&sys_util_history.get(), max_history.get());

        let x_axis = Axis::new()
            .range(vec![0, max_history.get() - 1])
            .tick_values(vec![]);
        let margin = Margin::new().left(0).right(0).top(0).bottom(0);
        let layout = plot.layout().clone().margin(margin).x_axis(x_axis);
        plot.set_layout(layout);

        spawn_local(async move {
            react(div_id, &plot).await;
        });
    });

    view! { <div class="leftmini" id=div_id></div> }
}

#[component]
fn PlotNetworkMini(
    sys_util_history: Signal<Vec<SystemUtilization>>,
    max_history: ReadSignal<usize>,
) -> impl IntoView {
    let div_id = "side-network";
    create_effect(move |_| {
        let mut plot = plot_network(&sys_util_history.get(), max_history.get());

        let x_axis = Axis::new()
            .range(vec![0, max_history.get() - 1])
            .tick_values(vec![]);
        let margin = Margin::new().left(0).right(0).top(0).bottom(0);
        let layout = plot.layout().clone().margin(margin).x_axis(x_axis);
        plot.set_layout(layout);

        spawn_local(async move {
            react(div_id, &plot).await;
        });
    });

    view! { <div class="leftmini" id=div_id></div> }
}

#[component]
fn SidePanel(
    main_view: WriteSignal<MainView>,
    sys_util_history: Signal<Vec<SystemUtilization>>,
    max_history: ReadSignal<usize>,
) -> impl IntoView {
    let cpu_descr = move || {
        let sys_util_history = sys_util_history.get();
        let (usage, freq) = if let Some(sys_util) = sys_util_history.last() {
            let cpus = &sys_util.cpus;
            let usage = cpus.iter().map(|cpu| cpu.usage).sum::<f32>() / cpus.len() as f32;
            let freq_mhz = cpus.iter().map(|cpu| cpu.freq).sum::<u64>() / cpus.len() as u64;
            let freq = freq_mhz as f32 / 1000.0;
            (usage, freq)
        } else {
            (0.0, 0.0)
        };
        format!("{:.0}% {:.2} GHz", usage, freq)
    };

    let mem_descr = move || {
        let sys_util_history = sys_util_history.get();
        if let Some(sys_util) = sys_util_history.last() {
            let gb = 1_073_741_824.0;
            let mem_curr = sys_util.mem as f32 / gb;
            let mem_max = sys_util.mem_max as f32 / gb;
            format!(
                "{:.1}/{:.1} GiB ({:.0}%)",
                mem_curr,
                mem_max,
                mem_curr / mem_max * 100.0
            )
        } else {
            String::new()
        }
    };

    let disk_descr = move || {
        let sys_util_history = sys_util_history.get();
        let (read, write) = if let Some(sys_util) = sys_util_history.last() {
            (sys_util.disk.read_bytes, sys_util.disk.writen_bytes)
        } else {
            (0, 0)
        };
        (print_bytes(read), print_bytes(write))
    };

    let net_descr = move || {
        let sys_util_history = sys_util_history.get();
        let network = sys_util_history
            .last()
            .map_or(Network::default(), |sys_util| sys_util.network.clone());
        (print_bytes(network.down), print_bytes(network.up))
    };

    view! {
        <div>
            <button on:click=move |_| { main_view.set(MainView::Cpu) }>
                <PlotCpuMini sys_util_history=sys_util_history max_history=max_history/>
                <div class="rightmini">
                    <div class="rightminititle">CPU</div>
                    {cpu_descr}
                </div>
            </button>

            <button on:click=move |_| { main_view.set(MainView::Mem) }>
                <PlotMemMini sys_util_history=sys_util_history max_history=max_history/>
                <div class="rightmini">
                    <div class="rightminititle">Memory</div>
                    {mem_descr}
                </div>
            </button>

            <PlotGpusMini sys_util_history=sys_util_history max_history=max_history main_view/>

            <button on:click=move |_| { main_view.set(MainView::Disk) }>
                <PlotDiskMini sys_util_history=sys_util_history max_history=max_history/>
                <div class="rightmini">
                    <div class="rightminititle">Disk</div>
                    {
                        move || {
                            let (read, write) = disk_descr();
                            view! {
                                <table>
                                    <tr>
                                        <td>
                                            <span style=COLOR_READ_HTML>
                                                <b>"R"</b>
                                            </span>
                                        </td>
                                        <td>{read}</td>
                                    </tr>
                                    <tr>
                                        <td>
                                            <span style=COLOR_WRITE_HTML>
                                                <b>"W"</b>
                                            </span>
                                        </td>
                                        <td>{write}</td>
                                    </tr>
                                </table>
                            }
                        }
                    }
                </div>
            </button>

            <button on:click=move |_| { main_view.set(MainView::Network) }>
                <PlotNetworkMini sys_util_history=sys_util_history max_history=max_history/>
                <div class="rightmini">
                    <div class="rightminititle">Network</div>
                    {
                        move || {
                            let (down, up) = net_descr();
                            view! {
                                <span style=COLOR_DOWN_HTML><b>"↓"</b></span>{down}
                                <br/>
                                <span style=COLOR_UP_HTML><b>"↑"</b></span>{up}
                            }
                        }
                    }
                </div>
            </button>

        // <img src="public/rzulta.png" style="width:100%; height:auto"/>
        </div>
    }
}

fn print_bytes(value: u64) -> String {
    let mut value = value as f32;
    let suffixes = ["B", "KiB", "MiB", "GiB", "TiB"];
    let base = 1024.0;
    let mut pow = 0;
    while (value >= base) && pow < suffixes.len() - 1 {
        value /= base;
        pow += 1;
    }
    format!("{:.1} {}", value, suffixes.get(pow).unwrap())
}

fn print_secs(value: u64) -> String {
    let mut value = value;
    let suffixes = ["s", "min", "h"];
    let base = 60;
    let mut pow = 0;
    while (value >= base) && pow < suffixes.len() - 1 {
        value /= base;
        pow += 1;
    }
    format!("{} {}", value, suffixes.get(pow).unwrap())
}

#[component]
fn MainPanel(
    main_view: ReadSignal<MainView>,
    sys_info: ReadSignal<SystemInfo>,
    sys_util_history: ReadSignal<VecDeque<SystemUtilization>>,
    max_history: ReadSignal<usize>,
    history_time: ReadSignal<usize>,
) -> impl IntoView {
    let div_id = "main-view";

    let sys_util_history_sampled: Signal<_> = {
        move || {
            let history_time = history_time.get();
            let sys_util_history = sys_util_history.get();
            let step = history_time.div_ceil(max_history.get());

            if sys_util_history.len() < step {
                // So that there are proper y axes values for long periods such as 24h
                sys_util_history.iter().take(1).cloned().collect()
            } else {
                sys_util_history
                    .iter()
                    .rev()
                    .skip(sys_util_history.len() % step)
                    .take(history_time)
                    .step_by(step)
                    .rev()
                    .cloned()
                    .collect::<Vec<_>>()
            }
        }
    }
    .into();

    create_effect(move |_| {
        let binding = sys_util_history.get();
        let sys_util_history = binding.iter().rev().take(history_time.get());
        let sys_util_history_sampled = sys_util_history_sampled.get();

        let mut title = Title::new("");
        let black = Rgb::new(0, 0, 0);
        let x_axis = Axis::new()
            .range(vec![0, max_history.get() - 1])
            .tick_values(vec![0.0])
            .tick_text(vec![format!("{}", print_secs(history_time.get() as u64))])
            .line_color(black)
            .mirror(true);
        let y_ticks = vec![0.0, 20.0, 40.0, 60.0, 80.0, 100.0];

        let mut y_axis = Axis::new()
            .side(AxisSide::Right)
            .line_color(black)
            .mirror(true);

        let mut plot = match main_view.get() {
            MainView::Cpu => {
                let plot = plot_cpu(&sys_util_history_sampled, max_history.get());

                title = Title::new(&sys_info.get().cpu_brand.to_string());
                let y_ticks_text = y_ticks.iter().map(|x| format!("{:.0}%", x)).collect();
                y_axis = y_axis
                    .range(vec![0, 100])
                    .tick_values(y_ticks)
                    .tick_text(y_ticks_text);

                plot
            }

            MainView::Mem => {
                let plot = plot_mem(&sys_util_history_sampled, max_history.get());

                let mem_max = sys_util_history_sampled
                    .first()
                    .map_or(0, |sys_util| sys_util.mem_max);
                let y_ticks_values: Vec<_> =
                    y_ticks.iter().map(|y| y * mem_max as f64 / 100.0).collect();
                let y_ticks_text = y_ticks_values
                    .iter()
                    .map(|y| print_bytes(*y as u64))
                    .collect();
                y_axis = y_axis
                    .range(vec![0, mem_max])
                    .tick_values(y_ticks_values)
                    .tick_text(y_ticks_text);

                plot
            }

            MainView::Gpu(gpu_id) => {
                let plot = plot_gpu(&sys_util_history_sampled, max_history.get(), gpu_id);

                title = Title::new(&sys_info.get().gpu_names[gpu_id]);
                let y_ticks_text = y_ticks.iter().map(|x| format!("{:.0}%", x)).collect();
                y_axis = y_axis
                    .range(vec![0, 100])
                    .tick_values(y_ticks)
                    .tick_text(y_ticks_text);

                plot
            }

            MainView::Network => {
                let plot = plot_network(&sys_util_history_sampled, max_history.get());
                let max = sys_util_history_sampled
                    .iter()
                    .map(|util| util.network.down.max(util.network.up))
                    .max()
                    .unwrap_or(0);
                let y_ticks_values: Vec<_> =
                    y_ticks.iter().map(|y| y * max as f64 / 100.0).collect();
                let y_ticks_text = y_ticks_values
                    .iter()
                    .map(|y| print_bytes(*y as u64))
                    .collect();
                y_axis = y_axis
                    .range(vec![0, max])
                    .tick_values(y_ticks_values)
                    .tick_text(y_ticks_text);

                let (total_down, total_up) = sys_util_history.fold((0, 0), |(down, up), util| {
                    (down + util.network.down, up + util.network.up)
                });
                title = Title::new(&format!(
                    "Total: {} | {}",
                    print_bytes(total_down),
                    print_bytes(total_up)
                ));
                plot
            }

            MainView::Disk => {
                let plot = plot_disk(&sys_util_history_sampled, max_history.get());
                let max = sys_util_history_sampled
                    .iter()
                    .map(|util| util.network.down.max(util.network.up))
                    .max()
                    .unwrap_or(0);
                let y_ticks_values: Vec<_> =
                    y_ticks.iter().map(|y| y * max as f64 / 100.0).collect();
                let y_ticks_text = y_ticks_values
                    .iter()
                    .map(|y| print_bytes(*y as u64))
                    .collect();
                y_axis = y_axis
                    .range(vec![0, max])
                    .tick_values(y_ticks_values)
                    .tick_text(y_ticks_text);

                let (total_read, total_write) =
                    sys_util_history.fold((0, 0), |(read, write), util| {
                        (read + util.disk.read_bytes, write + util.disk.writen_bytes)
                    });
                title = Title::new(&format!(
                    "Total: {} | {}",
                    print_bytes(total_read),
                    print_bytes(total_write)
                ));

                plot
            }
        };

        let _transparent = Rgba::new(0, 0, 0, 0.0);
        let layout = plot
            .layout()
            .clone()
            // .paper_background_color(transparent)
            .title(title)
            .y_axis(y_axis)
            .x_axis(x_axis);
        plot.set_layout(layout);

        spawn_local(async move {
            react(div_id, &plot).await;
        });
    });

    view! {
        <div class="rightpanel">
            <div style="height:450px">
                <div id=div_id></div>
            </div>
        </div>
    }
}

#[derive(Clone)]
enum MainView {
    Cpu,
    Mem,
    Gpu(usize),
    Disk,
    Network,
}

#[component]
pub fn App() -> impl IntoView {
    let update_interval = Duration::from_millis(1000);

    const TIME_OPTIONS: [u64; 7] = [
        60,
        5 * 60,
        30 * 60,
        3 * 3600,
        6 * 3600,
        12 * 3600,
        24 * 3600,
    ];

    let sys_util_history = RwSignal::new(VecDeque::new());
    let sys_info = RwSignal::new(SystemInfo::default());
    let main_view = RwSignal::new(MainView::Cpu);
    let history_time = RwSignal::new(TIME_OPTIONS[0] as usize);
    let x_axis_points = RwSignal::new(TIME_OPTIONS[0] as usize);
    let get_history_time = move |ev| {
        let value = event_target_value(&ev).parse().unwrap();
        history_time.set(value);
        x_axis_points.set(value.min(TIME_OPTIONS[1] as usize))
    };

    spawn_local(async move {
        let values = invoke("get_sys_info", JsValue::NULL).await;
        let values = serde_wasm_bindgen::from_value(values).unwrap();
        sys_info.set(values);
    });

    let update_sys_util = move || {
        spawn_local(async move {
            let values = invoke("get_stats", JsValue::NULL).await;
            let values: SystemUtilization = serde_wasm_bindgen::from_value(values).unwrap();
            sys_util_history.update(|history| {
                history.push_back(values);
                if history.len() > TIME_OPTIONS[TIME_OPTIONS.len() - 1] as usize {
                    history.pop_front();
                }
            });
        });
    };
    update_sys_util();

    set_interval(update_sys_util, update_interval);

    const X_AXIS_LEN_STATIC: usize = TIME_OPTIONS[0] as usize;
    let sys_util_history_side_panel = {
        move || {
            sys_util_history
                .get()
                .iter()
                .rev()
                .take(X_AXIS_LEN_STATIC)
                .rev()
                .cloned()
                .collect()
        }
    }
    .into();

    let (x_axis_points_static, _) = RwSignal::new(X_AXIS_LEN_STATIC).split();

    view! {
        <main class="container">
            <div>
                <div class="leftpanel">
                    <SidePanel
                        main_view=main_view.write_only()
                        sys_util_history=sys_util_history_side_panel
                        max_history=x_axis_points_static
                    />
                    <div style="margin-top:10px">
                        <b>"Period: "</b>
                        <select on:input=get_history_time>

                            {TIME_OPTIONS
                                .into_iter()
                                .map(|x| view! { <option value=x>{print_secs(x)}</option> })
                                .collect_view()}

                        </select>
                    </div>
                </div>
                <MainPanel
                    main_view=main_view.read_only()
                    sys_util_history=sys_util_history.read_only()
                    max_history=x_axis_points.read_only()
                    sys_info=sys_info.read_only()
                    history_time=history_time.read_only()
                />
            </div>
        </main>
    }
}

#[cfg(test)]
mod tests {
    use super::{print_bytes, print_secs};

    #[test]
    fn print_bytes_test() {
        let test_cases = [
            (0, "0.0 B"),
            (1023, "1023.0 B"),
            (1024, "1.0 KiB"),
            (21_372_137, "20.4 MiB"),
            (2_137_213_721_372_137, "1943.8 TiB"),
        ];
        for (input, expected) in test_cases {
            assert_eq!(expected, print_bytes(input));
        }
    }

    #[test]
    fn print_time_test() {
        let test_cases = [
            (0, "0 s"),
            (59, "59 s"),
            (60, "1 min"),
            (3 * 60, "3 min"),
            (3 * 60 + 1, "3 min"),
            (60, "1 min"),
            (5 * 60, "5 min"),
            (30 * 60, "30 min"),
            (3 * 60 * 60, "3 h"),
            (6 * 60 * 60, "6 h"),
            (12 * 60 * 60, "12 h"),
            (24 * 60 * 60, "24 h"),
        ];
        for (input, expected) in test_cases {
            assert_eq!(expected, print_secs(input));
        }
    }
}
