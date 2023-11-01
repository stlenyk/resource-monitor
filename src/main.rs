mod app;
mod send_types;

use app::*;
use leptos::*;

fn main() {
    mount_to_body(|| {
        view! { <App/> }
    })
}
