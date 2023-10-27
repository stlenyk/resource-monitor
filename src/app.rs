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
fn plot_cpu(sys_util_history: &VecDeque<SystemUtilization>, max_history: usize) -> Plot {
    let mut plot = Plot::new();

    let config = Configuration::new().static_plot(true).responsive(true);
    plot.set_configuration(config);
    let layout = Layout::new().auto_size(true);
    plot.set_layout(layout);

    let cpu_history = sys_util_history
        .iter()
        .map(|util| util.cpus.clone())
        .collect::<Vec<_>>();

    if let Some(history_point) = cpu_history.get(0) {
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

fn plot_generic<T: Clone + Serialize + 'static>(
    values: Vec<T>,
    max_history: usize,
    color: Rgb,
) -> Plot {
    let mut plot = Plot::new();
    let config = Configuration::new().static_plot(true);
    plot.set_configuration(config);
    let layout = Layout::new().auto_size(true);
    plot.set_layout(layout);

    let lower_bound = (max_history - values.len()).max(0);
    let x = (lower_bound..max_history).collect::<Vec<_>>();
    let trace = Scatter::new(x, values)
        .show_legend(false)
        .marker(Marker::new().color(color).size(1))
        .fill(Fill::ToZeroY);

    plot.add_trace(trace);

    plot
}

fn plot_mem(sys_util_history: &VecDeque<SystemUtilization>, max_history: usize) -> Plot {
    let plot_values = sys_util_history.iter().map(|util| util.mem).collect();
    let color = Rgb::new(101, 39, 190);
    plot_generic(plot_values, max_history, color)
}

fn plot_gpu(
    sys_util_history: &VecDeque<SystemUtilization>,
    max_history: usize,
    gpu_id: usize,
) -> Plot {
    let plot_values = sys_util_history
        .iter()
        .map(|util| util.gpus[gpu_id].usage)
        .collect();
    let color = Rgb::new(120, 149, 203);
    plot_generic(plot_values, max_history, color)
}

#[component]
fn PlotCpuMini(
    sys_util_history: ReadSignal<VecDeque<SystemUtilization>>,
    max_history: ReadSignal<usize>,
) -> impl IntoView {
    let div_id = "side-cpu";
    create_effect(move |_| {
        let mut plot = plot_cpu(&sys_util_history.get(), max_history.get());

        let y_ticks = vec![0.0, 20.0, 40.0, 60.0, 80.0, 100.0];
        let y_axis = Axis::new().range(vec![0, 100]).tick_values(y_ticks);
        let x_axis = Axis::new().range(vec![0, max_history.get() - 1]);
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

    view! {
        <div class="leftmini" id={div_id}></div>
    }
}

#[component]
fn PlotMemMini(
    sys_util_history: ReadSignal<VecDeque<SystemUtilization>>,
    max_history: usize,
) -> impl IntoView {
    let div_id = "side-mem";
    create_effect(move |_| {
        let mut plot = plot_mem(&sys_util_history.get(), max_history);

        let max_mem = if let Some(sys_util) = sys_util_history.get().get(0) {
            sys_util.mem_max
        } else {
            0
        };
        let y_axis = Axis::new().range(vec![0, max_mem]);
        let x_axis = Axis::new().range(vec![0, max_history - 1]);
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

    view! {
        <div class="leftmini" id={div_id}></div>
    }
}

#[component]
fn PlotGpusMini(
    sys_util_history: ReadSignal<VecDeque<SystemUtilization>>,
    max_history: usize,
    main_view: WriteSignal<MainView>,
) -> impl IntoView {
    view! {
        <For
            each=move || 0..sys_util_history
                .get()
                .get(0)
                .map_or(0, |sys_util| sys_util.gpus.len())
            key=|gpu_id| *gpu_id
            children=move |gpu_id| {
                // TODO is there a way to pass around `let div_id = format!("side-gpu-{}", gpu_id)` that doesn't require 2 clone()s:
                // let div_id = format!("side-gpu-{}", gpu_id);
                // let div_id1 = div_id.clone();
                // create_effect(move |_| {
                //     ...
                //     let div_id2 = div_id1.clone();
                //     spawn_local(async move {
                //         react(&div_id2, &plot).await;
                //     });
                // });
                create_effect(move |_| {
                    let mut plot = plot_gpu(&sys_util_history.get(), max_history, gpu_id);

                    let y_ticks = vec![0.0, 20.0, 40.0, 60.0, 80.0, 100.0];
                    let y_axis = Axis::new()
                        .range(vec![0, 100])
                        .tick_values(y_ticks);
                    let x_axis = Axis::new()
                        .range(vec![0, max_history - 1])
                        .tick_values(vec![0.0]);
                    let margin = Margin::new().left(0).right(0).top(0).bottom(0);
                    let layout = plot
                        .layout()
                        .clone()
                        .margin(margin)
                        .y_axis(y_axis)
                        .x_axis(x_axis);
                    plot.set_layout(layout);

                    spawn_local(async move {
                        react(&format!("side-gpu-{}", gpu_id), &plot).await;
                    });
                });

                let gpu_descr = move || {
                    if let Some(last) = sys_util_history.get().back() {
                        let gpu = last.gpus[gpu_id].clone();
                        format!("{}% ({} ℃)", gpu.usage, gpu.temp)
                    } else {
                        String::new()
                    }
                };
                view! {
                    <button on:click=move |_| {main_view.set(MainView::Gpu(gpu_id))} >
                        <div class="leftmini" id=format!("side-gpu-{}", gpu_id)></div>
                        <div class="rightmini">
                            <div class="rightminititle">{format!("GPU {}", gpu_id)}</div>
                            <br/>
                            {gpu_descr}
                        </div>
                    </button>
                }
            }
        />
    }
}

#[component]
fn SidePanel(
    main_view: WriteSignal<MainView>,
    sys_util_history: ReadSignal<VecDeque<SystemUtilization>>,
    max_history: ReadSignal<usize>,
) -> impl IntoView {
    let cpu_descr = move || {
        let sys_util_history = sys_util_history.get();
        let (usage, freq) = if let Some(sys_util) = sys_util_history.back() {
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
        if let Some(sys_util) = sys_util_history.back() {
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

    view! {
        <div class="leftpanel">

            <button on:click=move |_| {main_view.set(MainView::Cpu)}>
                <PlotCpuMini sys_util_history=sys_util_history max_history=max_history/>
                <div class="rightmini">
                    <div class="rightminititle">CPU</div>
                    <br/>
                    {cpu_descr}
                </div>
            </button>

            <button on:click=move |_| {main_view.set(MainView::Mem)}>
                <PlotMemMini sys_util_history=sys_util_history max_history=max_history.get()/>
                <div class="rightmini">
                <div class="rightminititle">Memory</div>
                <br/>
                {mem_descr}
                </div>
            </button>

            <PlotGpusMini sys_util_history=sys_util_history max_history=max_history.get() main_view/>

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
    format!("{:.1} {}", value, suffixes.get(pow).unwrap_or(&"TiB"))
}

#[component]
fn MainPanel(
    main_view: ReadSignal<MainView>,
    sys_info: ReadSignal<SystemInfo>,
    sys_util_history: ReadSignal<VecDeque<SystemUtilization>>,
    max_history: ReadSignal<usize>,
    max_history_time: Duration,
) -> impl IntoView {
    let div_id = "main-view";
    create_effect(move |_| {
        let mut title = Title::new("");
        let black = Rgb::new(0, 0, 0);
        let x_axis = Axis::new()
            .range(vec![0, max_history.get() - 1])
            .tick_values(vec![0.0])
            .tick_text(vec![format!("{} s", max_history.get())])
            .line_color(black)
            .mirror(true);
        let y_ticks = vec![0.0, 20.0, 40.0, 60.0, 80.0, 100.0];

        let mut y_axis = Axis::new()
            .side(AxisSide::Right)
            .line_color(black)
            .mirror(true);

        let mut plot = match main_view.get() {
            MainView::Cpu => {
                let plot = plot_cpu(&sys_util_history.get(), max_history.get());

                title = Title::new(&sys_info.get().cpu_brand.to_string());
                let y_ticks_text = y_ticks.iter().map(|x| format!("{:.0}%", x)).collect();
                y_axis = y_axis
                    .range(vec![0, 100])
                    .tick_values(y_ticks)
                    .tick_text(y_ticks_text);

                plot
            }

            MainView::Mem => {
                let plot = plot_mem(&sys_util_history.get(), max_history.get());

                let mem_max = sys_util_history
                    .get()
                    .get(0)
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
                let plot = plot_gpu(&sys_util_history.get(), max_history.get(), gpu_id);

                title = Title::new(&sys_info.get().gpu_names[gpu_id]);
                let y_ticks_text = y_ticks.iter().map(|x| format!("{:.0}%", x)).collect();
                y_axis = y_axis
                    .range(vec![0, 100])
                    .tick_values(y_ticks)
                    .tick_text(y_ticks_text);

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
                <div id=div_id/>
            </div>
        </div>
        <h2>{max_history}</h2>
    }
}

#[derive(Clone)]
enum MainView {
    Cpu,
    Mem,
    Gpu(usize),
}

#[component]
pub fn App() -> impl IntoView {
    let update_interval = Duration::from_millis(1000);
    let max_history_time = Duration::from_secs(60);

    let sys_util_history = RwSignal::new(VecDeque::new());
    let sys_util_history_to_show = RwSignal::new(VecDeque::new());
    let sys_info = RwSignal::new(SystemInfo::default());
    let main_view = RwSignal::new(MainView::Cpu);
    let history_time = RwSignal::new(60);
    let get_history_time = move |ev| {
        let value = event_target_value(&ev).parse().unwrap();
        history_time.set(value);
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
            let mut history = sys_util_history.get();
            history.push_back(values);
            const SEC_24H: usize = 24 * 60 * 60;
            if history.len() > SEC_24H {
                history.pop_front();
            }
            sys_util_history.set(history.clone());
            let history_to_show = history
                .iter()
                .rev()
                .cloned()
                // .take(history_time.get().as_secs() as usize)
                .collect();
            sys_util_history_to_show.set(history_to_show);
        });
    };
    update_sys_util();

    set_interval(update_sys_util, update_interval);

    view! {
        <main class="container">
            <div>
                <SidePanel main_view=main_view.write_only() sys_util_history=sys_util_history_to_show.read_only() max_history=history_time.read_only()/>
                <MainPanel main_view=main_view.read_only() sys_util_history=sys_util_history_to_show.read_only() max_history=history_time.read_only() max_history_time=max_history_time sys_info=sys_info.read_only()/>

                <b>"Period: "</b>
                <select on:input=get_history_time>
                    <option value=60>"1 min"</option>
                    <option value=5*60>"5 min"</option>
                    <option value=30*60>"30 min"</option>
                    <option value=3*60*60>"3 h"</option>
                    <option value=6*60*60>"6 h"</option>
                    <option value=12*60*60>"12 h"</option>
                    <option value=24*60*60>"24 h"</option>
                </select>
                <h2>{history_time}</h2>
            </div>
        </main>
    }
}

#[cfg(test)]
mod tests {
    use super::print_bytes;

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
}
