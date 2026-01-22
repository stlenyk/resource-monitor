//! Wrapper module to expose plotly bindings for static analysis
//! This allows rust-analyzer and clippy to see the APIs even when not targeting WASM

#[cfg(target_family = "wasm")]
pub use plotly::bindings::react;

#[cfg(not(target_family = "wasm"))]
/// Stub implementation for static analysis (rust-analyzer, clippy)
/// The real implementation is only available when targeting WASM
pub async fn react(_id: &str, _plot: &plotly::Plot) {
    panic!("plotly::bindings::react is only available when targeting WASM");
}
