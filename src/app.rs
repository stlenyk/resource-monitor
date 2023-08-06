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
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "tauri"])]
    async fn invoke(cmd: &str, args: JsValue) -> JsValue;
}

#[derive(Serialize, Deserialize)]
struct GreetArgs<'a> {
    name: &'a str,
}

// Assumes that the number of cpus doesn't change and may panic otherwise.
fn plot_cpu(sys_util_history: &VecDeque<SystemUtilization>, max_history: usize) -> Plot {
    let mut plot = Plot::new();

    let config = Configuration::new().static_plot(true);
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
        .map(|util| util.gpus[gpu_id])
        .collect();
    let color = Rgb::new(120, 149, 203);
    plot_generic(plot_values, max_history, color)
}

#[component]
fn PlotCpuMini(
    cx: Scope,
    sys_util_history: ReadSignal<VecDeque<SystemUtilization>>,
    max_history: usize,
) -> impl IntoView {
    let div_id = "side-cpu";
    create_effect(cx, move |_| {
        let mut plot = plot_cpu(&sys_util_history.get(), max_history);

        let y_ticks = vec![0.0, 20.0, 40.0, 60.0, 80.0, 100.0];
        let y_axis = Axis::new().range(vec![0, 100]).tick_values(y_ticks);
        let x_axis = Axis::new().range(vec![0, max_history - 1]);
        let margin = Margin::new().left(0).right(0).top(0).bottom(0);
        let layout = plot
            .layout()
            .clone()
            .margin(margin)
            .height(100)
            .y_axis(y_axis)
            .x_axis(x_axis);
        plot.set_layout(layout);

        spawn_local(async move {
            react(div_id, &plot).await;
        });
    });

    view! {cx,
        <div id={div_id}></div>
    }
}

#[component]
fn PlotMemMini(
    cx: Scope,
    sys_util_history: ReadSignal<VecDeque<SystemUtilization>>,
    max_history: usize,
) -> impl IntoView {
    let div_id = "side-mem";
    create_effect(cx, move |_| {
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
            .height(100)
            .y_axis(y_axis)
            .x_axis(x_axis);
        plot.set_layout(layout);

        spawn_local(async move {
            react(div_id, &plot).await;
        });
    });

    view! {cx,
        <div id={div_id}></div>
    }
}

#[component]
fn PlotGpusMini(
    cx: Scope,
    sys_util_history: ReadSignal<VecDeque<SystemUtilization>>,
    max_history: usize,
    main_view: WriteSignal<MainView>,
) -> impl IntoView {
    // Why does compilation fail with this?
    // let div_id = format!("side-gpu-{}", gpu_id);
    // ...
    // react(&div_id, &plot).await;
    // ...
    // view! {... id=div_id ...}
    // Why `div_id` can't be reused?
    view! {cx,
        <For
            each=move || 0..sys_util_history
                .get()
                .get(0)
                .map_or(0, |sys_util| sys_util.gpus.len())
            key=|gpu_id| *gpu_id
            view=move |cx, gpu_id| {
                create_effect(cx, move |_| {
                    // let mut plot = plot_gpu(&sys_util_history.get(), max_history, gpu_id);
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
                        .height(100)
                        .y_axis(y_axis)
                        .x_axis(x_axis);
                    plot.set_layout(layout);

                    spawn_local(async move {
                        react(&format!("side-gpu-{}", gpu_id), &plot).await;
                    });
                });
                view! {cx,
                    <button on:click=move |_| {main_view.set(MainView::Gpu(gpu_id))} >
                        <div id=format!("side-gpu-{}", gpu_id)></div>
                    </button>}
            }
        />
    }
}

#[component]
fn SidePanel(
    cx: Scope,
    main_view: WriteSignal<MainView>,
    sys_util_history: ReadSignal<VecDeque<SystemUtilization>>,
    max_history: usize,
) -> impl IntoView {
    view! {cx,
        <div class="left">

            <button on:click=move |_| {main_view.set(MainView::Cpu)}>
                <PlotCpuMini sys_util_history=sys_util_history max_history=max_history/>
            </button>

            <button on:click=move |_| {main_view.set(MainView::Mem)}>
                <PlotMemMini sys_util_history=sys_util_history max_history=max_history/>
            </button>

            <PlotGpusMini sys_util_history=sys_util_history max_history=max_history main_view/>

            // <img src="public/rzulta.png" alt="wrong path" style="width:100%; height:auto"/>
        </div>
    }
}

#[allow(unused)]
fn print_bytes(value: u64) -> String {
    let mut value = value as f32;
    let suffixes = ["B", "KiB", "MiB", "GiB", "TiB"];
    let base = 1024.0;
    let mut pow = 0;
    while value > base {
        value /= base;
        pow += 1;
    }
    format!("{:.1} {}", value, suffixes.get(pow).unwrap_or(&"TB"))
}

#[component]
fn MainPanel(
    cx: Scope,
    main_view: ReadSignal<MainView>,
    sys_util_history: ReadSignal<VecDeque<SystemUtilization>>,
    max_history: usize,
    max_history_time: Duration,
) -> impl IntoView {
    let div_id = "main-view";

    create_effect(cx, move |_| {
        let title = Title::new(&format!("history len: {}", sys_util_history.get().len()));
        let black = Rgb::new(0, 0, 0);
        let x_axis = Axis::new()
            .range(vec![0, max_history - 1])
            .tick_values(vec![0.0])
            .tick_text(vec![format!("{} s", max_history_time.as_secs())])
            .line_color(black)
            .mirror(true);
        let y_ticks = vec![0.0, 20.0, 40.0, 60.0, 80.0, 100.0];

        let mut y_axis = Axis::new()
            .side(AxisSide::Right)
            .line_color(black)
            .mirror(true);

        let mut plot = match main_view.get() {
            MainView::Cpu => {
                let plot = plot_cpu(&sys_util_history.get(), max_history);

                let y_ticks_text = y_ticks.iter().map(|x| format!("{:.0}%", x)).collect();
                y_axis = y_axis
                    .range(vec![0, 100])
                    .tick_values(y_ticks)
                    .tick_text(y_ticks_text);

                plot
            }

            MainView::Mem => {
                let plot = plot_mem(&sys_util_history.get(), max_history);

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
                let plot = plot_gpu(&sys_util_history.get(), max_history, gpu_id);

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

    view! {cx,
        <div class="right" id=div_id></div>
    }
}

#[derive(Clone)]
enum MainView {
    Cpu,
    Mem,
    Gpu(usize),
}

#[component]
pub fn App(cx: Scope) -> impl IntoView {
    let update_interval = Duration::from_millis(500);
    let max_history_time = Duration::from_secs(60);
    let max_history = (max_history_time.as_millis() / update_interval.as_millis()) as usize;

    let (sys_util_history, set_sys_util) = create_signal(cx, VecDeque::with_capacity(max_history));
    let (main_view, set_main_view) = create_signal(cx, MainView::Cpu);

    let update_sys_util = move || {
        spawn_local(async move {
            let values = invoke("get_stats", JsValue::NULL).await;
            let values = serde_wasm_bindgen::from_value(values).unwrap();
            let mut history = sys_util_history.get();
            history.push_back(values);
            if history.len() > max_history {
                history.pop_front();
            }
            set_sys_util.set(history);
        });
    };
    update_sys_util();

    set_interval(update_sys_util, update_interval);

    view! { cx,
        <main class="container">
            <div>
                <p>"Main view: " {move ||
                    match main_view.get() {
                        MainView::Cpu => "cpu".to_owned(),
                        MainView::Mem => "mem".to_owned(),
                        MainView::Gpu(gpu_id) => format!("gpu{}", gpu_id)
                    }
                }</p>
                <SidePanel main_view=set_main_view sys_util_history=sys_util_history max_history=max_history/>
                <MainPanel main_view=main_view sys_util_history=sys_util_history max_history=max_history max_history_time=max_history_time/>
            </div>
        </main>
    }
}
