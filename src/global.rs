use crate::{Executor, JoinHandle, SendBound};
use std::future::Future;

/// Executor that switches between [`TokioExecutor`](crate::rt::tokio::TokioExecutor), [`ThreadpoolExecutor`](crate::rt::threadpool::ThreadpoolExecutor) and [`WasmExecutor`](crate::rt::wasm::WasmExecutor) at compile time.
/// * If the target is non-wasm32 and the "tokio" feature is enabled, [`tokio`] would be used.
/// * if the target is non-wasm32 and the "threadpool" feature is enabled with [`tokio`] feature disabled, [`futures`] [`ThreadPool`](futures::executor::ThreadPool) will be used.
/// * If the target is wasm32, [`wasm-bindgen-futures`] would be used.
#[derive(Clone, Copy, Debug, PartialOrd, PartialEq, Eq)]
pub struct GlobalExecutor;

impl Executor for GlobalExecutor {
    #[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
    fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + SendBound + 'static,
        F::Output: SendBound + 'static,
    {
        crate::rt::tokio::TokioExecutor.spawn(future)
    }

    #[cfg(all(feature = "threadpool", not(feature = "tokio")))]
    fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + SendBound + 'static,
        F::Output: SendBound + 'static,
    {
        crate::rt::threadpool::ThreadPoolExecutor.spawn(future)
    }

    #[cfg(target_arch = "wasm32")]
    fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + SendBound + 'static,
        F::Output: SendBound + 'static,
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
        F: Future + SendBound + 'static,
        F::Output: SendBound + 'static,
    {
        unreachable!()
    }
}
