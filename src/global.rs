use crate::{Executor, ExecutorTimer, JoinHandle};
use std::future::Future;
use std::time::Duration;

/// Executor that switch between [`TokioExecutor`](crate::rt::tokio::TokioExecutor), [`ThreadpoolExecutor`](crate::rt::threadpool::ThreadpoolExecutor) and [`WasmExecutor`](crate::rt::wasm::WasmExecutor) at compile time.
/// * If the target is non-wasm32 and "tokio" feature is enabled, [`tokio`] would be used.
/// * if the target is non-wasm32 and "threadpool" feature is enabled with [`tokio`] feature disabled, [`futures`] [`ThreadPool`](futures::executor::ThreadPool) will be used.
/// * If the target is wasm32, [`wasm-bindgen-futures`] would be used.
#[derive(Clone, Copy, Debug, PartialOrd, PartialEq, Eq)]
pub struct GlobalExecutor;

impl Executor for GlobalExecutor {
    fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        #[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
        {
            crate::rt::tokio::TokioExecutor.spawn(future)
        }
        #[cfg(all(
            feature = "threadpool",
            not(feature = "tokio"),
            not(target_arch = "wasm32")
        ))]
        {
            crate::rt::threadpool::ThreadPoolExecutor.spawn(future)
        }
        #[cfg(target_arch = "wasm32")]
        {
            crate::rt::wasm::WasmExecutor.spawn(future)
        }

        #[cfg(all(
            not(feature = "threadpool"),
            not(feature = "tokio"),
            not(target_arch = "wasm32")
        ))]
        {
            _ = future;
            unreachable!()
        }
    }
}

impl ExecutorTimer for GlobalExecutor {
    fn timeout<F>(
        &self,
        duration: Duration,
        future: F,
    ) -> impl Future<Output = std::io::Result<F::Output>> + Send + 'static
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        #[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
        {
            crate::rt::tokio::TokioExecutor.timeout(duration, future)
        }
        #[cfg(all(
            feature = "threadpool",
            not(feature = "tokio"),
            not(target_arch = "wasm32")
        ))]
        {
            crate::rt::threadpool::ThreadPoolExecutor.timeout(duration, future)
        }
        #[cfg(target_arch = "wasm32")]
        {
            crate::rt::wasm::WasmExecutor.timeout(duration, future)
        }

        #[cfg(all(
            not(feature = "threadpool"),
            not(feature = "tokio"),
            not(target_arch = "wasm32")
        ))]
        {
            _ = future;
            _ = duration;
            unreachable!()
        }
    }

    fn sleep(&self, duration: Duration) -> impl Future<Output = ()> + Send + 'static {
        #[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
        {
            crate::rt::tokio::TokioExecutor.sleep(duration)
        }
        #[cfg(all(
            feature = "threadpool",
            not(feature = "tokio"),
            not(target_arch = "wasm32")
        ))]
        {
            crate::rt::threadpool::ThreadPoolExecutor.sleep(duration)
        }
        #[cfg(target_arch = "wasm32")]
        {
            crate::rt::wasm::WasmExecutor.sleep(duration)
        }

        #[cfg(all(
            not(feature = "threadpool"),
            not(feature = "tokio"),
            not(target_arch = "wasm32")
        ))]
        {
            _ = duration;
            unreachable!()
        }
    }
}
