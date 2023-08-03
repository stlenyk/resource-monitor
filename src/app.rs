use std::{collections::VecDeque, time::Duration, vec};

use leptos::*;
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

#[component]
/// NB: assumes that the number of cpus doesn't change. May panic otherwise.
fn PlotCpu(
    cx: Scope,
    sys_util_history: ReadSignal<VecDeque<SystemUtilization>>,
    max_history: usize,
    max_time: Duration,
) -> impl IntoView {
    let id = "my-div-id";

    create_effect(cx, move |_| {
        let mut plot = plotly::Plot::new();

        let config = plotly::configuration::Configuration::new().static_plot(true);
        plot.set_configuration(config);

        let title =
            plotly::common::Title::new(&format!("history len: {}", sys_util_history.get().len()));

        let black = plotly::color::Rgb::new(0, 0, 0);
        let y_ticks = vec![0.0, 20.0, 40.0, 60.0, 80.0, 100.0];
        let y_ticks_text = y_ticks.iter().map(|x| format!("{:.0}%", x)).collect();
        let y_axis = plotly::layout::Axis::new()
            .range(vec![0, 100])
            .tick_values(y_ticks)
            .tick_text(y_ticks_text)
            .side(plotly::common::AxisSide::Right)
            .line_color(black)
            .mirror(true);
        let x_axis = plotly::layout::Axis::new()
            .range(vec![0, max_history - 1])
            .tick_values(vec![0.0])
            .tick_text(vec![format!("{} s", max_time.as_secs())])
            .line_color(black)
            .mirror(true);
        let _transparent = plotly::color::Rgba::new(0, 0, 0, 0.0);
        let layout = plotly::layout::Layout::new()
            // .paper_background_color(transparent)
            .auto_size(true)
            .title(title)
            .y_axis(y_axis)
            .x_axis(x_axis);
        plot.set_layout(layout);

        let cpu_history = sys_util_history
            .get()
            .iter()
            .map(|util| util.cpus.clone())
            .collect::<Vec<_>>();

        if let Some(history_point) = cpu_history.get(0) {
            let cpu_count = history_point.len();
            let mut traces: Vec<Vec<f32>> = vec![Vec::new(); cpu_count];

            for history_point in &cpu_history {
                for (id, &cpu) in history_point.iter().enumerate() {
                    traces[id].push(cpu / cpu_count as f32);
                }
            }
            let lower_bound = (max_history - cpu_history.len()).max(0);
            let x = (lower_bound..max_history).collect::<Vec<_>>();
            let stack_group = "stack_group";
            for y in traces {
                let trace = plotly::Scatter::new(x.clone(), y)
                    .stack_group(stack_group)
                    .show_legend(false);
                plot.add_trace(trace);
            }
        }
        spawn_local(async move {
            plotly::bindings::react(id, &plot).await;
        });
    });

    view! {cx,
        <div id={id}></div>
    }
}

use crate::send_types::*;

#[component]
fn PlotCpuMini(cx: Scope) -> impl IntoView {
    view! {cx,
        "CPUMini placeholder"
    }
}

#[component]
fn PlotMemMini(cx: Scope) -> impl IntoView {
    view! {cx,
        "MemoryMini placeholder"
    }
}

#[component]
fn SidePanel(cx: Scope) -> impl IntoView {
    view! {cx,
        <PlotCpuMini/>
        <br/>
        <PlotMemMini/>
    }
}

#[component]
fn SysInfo(cx: Scope) -> impl IntoView {
    let update_interval = Duration::from_secs(1);
    let max_history_time = Duration::from_secs(60);
    let max_history = (max_history_time.as_millis() / update_interval.as_millis()) as usize;
    let (sys_util, set_sys_util) = create_signal(cx, VecDeque::with_capacity(max_history));

    let update_sys_util = move || {
        spawn_local(async move {
            let values = invoke("get_stats", JsValue::NULL).await;
            let values = serde_wasm_bindgen::from_value(values).unwrap();
            let mut history = sys_util.get();
            history.push_back(values);
            if history.len() > max_history {
                history.pop_front();
            }
            set_sys_util.set(history);
        });
    };

    set_interval(update_sys_util, update_interval);

    view! { cx,
        <SidePanel/>
        <PlotCpu sys_util_history=sys_util max_history=max_history max_time=max_history_time/>
    }
}

#[component]
pub fn App(cx: Scope) -> impl IntoView {
    view! { cx,
        <main class="container">
            <SysInfo/>
        </main>
    }
}
