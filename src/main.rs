mod app;

fn main() {
    leptos::mount::mount_to_body(|| {
        leptos::view! { <app::App/> }
    })
}
