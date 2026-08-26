/// Executor that selects an available runtime backend at compile time.
///
/// * On non-Wasm targets with the `tokio` or `compio` feature enabled, it uses
///   `TokioExecutor` or `CompioExecutor`.
/// * On non-Wasm targets with the `threadpool` feature enabled and the `tokio` or `compio`
///   feature disabled, it uses `ThreadPoolExecutor`.
/// * On Wasm targets, it uses `WasmExecutor`, backed by
///   `wasm-bindgen-futures`.
#[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
pub type GlobalExecutor = crate::rt::tokio::TokioExecutor;

#[cfg(all(
    feature = "compio",
    not(any(feature = "tokio", target_arch = "wasm32"))
))]
pub type GlobalExecutor = crate::rt::compio::CompioExecutor;

#[cfg(all(
    feature = "threadpool",
    not(any(feature = "tokio", feature = "compio", target_arch = "wasm32"))
))]
pub type GlobalExecutor = crate::rt::threadpool::ThreadPoolExecutor;

#[cfg(target_arch = "wasm32")]
pub type GlobalExecutor = crate::rt::wasm::WasmExecutor;

#[cfg(all(
    not(feature = "threadpool"),
    not(feature = "tokio"),
    not(feature = "compio"),
    not(target_arch = "wasm32")
))]
pub type GlobalExecutor = crate::rt::dummy::DummyExecutor;
