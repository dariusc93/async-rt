use crate::{Executor, ExecutorBlocking, ExecutorTimeout, JoinHandle};
use std::future::Future;

/// Executor that selects an available runtime backend at compile time.
///
/// * On non-Wasm targets with the `tokio` feature enabled, it uses
///   `TokioExecutor`.
/// * On non-Wasm targets with the `threadpool` feature enabled and the `tokio`
///   feature disabled, it uses `ThreadPoolExecutor`.
/// * On Wasm targets, it uses `WasmExecutor`, backed by
///   `wasm-bindgen-futures`.
#[derive(Clone, Copy, Debug, PartialOrd, PartialEq, Eq)]
pub struct GlobalExecutor;

impl Executor for GlobalExecutor {
    #[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
    fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        crate::rt::tokio::TokioExecutor.spawn(future)
    }

    #[cfg(all(
        feature = "threadpool",
        not(any(feature = "tokio", target_arch = "wasm32"))
    ))]
    fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        crate::rt::threadpool::ThreadPoolExecutor.spawn(future)
    }

    #[cfg(target_arch = "wasm32")]
    fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        crate::rt::wasm::WasmExecutor.spawn(future)
    }

    #[cfg(all(
        not(feature = "threadpool"),
        not(feature = "tokio"),
        not(target_arch = "wasm32")
    ))]
    fn spawn<F>(&self, _: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        unreachable!()
    }
}

impl ExecutorBlocking for GlobalExecutor {
    #[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
    fn spawn_blocking<F, R>(&self, f: F) -> JoinHandle<R>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        crate::rt::tokio::TokioExecutor.spawn_blocking(f)
    }

    #[cfg(all(
        feature = "threadpool",
        not(any(feature = "tokio", target_arch = "wasm32"))
    ))]
    fn spawn_blocking<F, R>(&self, f: F) -> JoinHandle<R>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        crate::rt::threadpool::ThreadPoolExecutor.spawn_blocking(f)
    }

    #[cfg(target_arch = "wasm32")]
    fn spawn_blocking<F, R>(&self, f: F) -> JoinHandle<R>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        crate::rt::wasm::WasmExecutor.spawn_blocking(f)
    }

    #[cfg(all(
        not(feature = "threadpool"),
        not(feature = "tokio"),
        not(target_arch = "wasm32")
    ))]
    fn spawn_blocking<F, R>(&self, _: F) -> JoinHandle<R>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        unimplemented!()
    }
}

impl ExecutorTimeout for GlobalExecutor {}
