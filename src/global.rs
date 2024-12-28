use crate::{rt, Executor, JoinHandle};
use std::future::Future;

/// Executor that switch between [`TokioExecutor`](rt::tokio::TokioExecutor) and [`WasmExecutor`](rt::wasm::WasmExecutor) at compile time.
/// If the target is non-wasm32, [`tokio`] would be used, if it is supported.
/// If the target is wasm32, [`wasm-bindgen-futures`] would be used.
#[derive(Clone, Copy, Debug, PartialOrd, PartialEq, Eq)]
pub struct GlobalExecutor;

impl Executor for GlobalExecutor {
    fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        #[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
        let executor = rt::tokio::TokioExecutor;
        #[cfg(target_arch = "wasm32")]
        let executor = rt::wasm::WasmExecutor;

        executor.spawn(future)
    }
}
