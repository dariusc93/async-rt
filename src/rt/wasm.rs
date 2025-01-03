use crate::{Executor, ExecutorTimer, InnerJoinHandle, JoinHandle};
use futures::future::{AbortHandle, Abortable};
use std::future::Future;
use std::time::Duration;

/// Wasm executor
#[derive(Clone, Copy, Debug, PartialOrd, PartialEq, Eq)]
pub struct WasmExecutor;

impl Executor for WasmExecutor {
    fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        let (abort_handle, abort_registration) = AbortHandle::new_pair();
        let future = Abortable::new(future, abort_registration);
        let (tx, rx) = futures::channel::oneshot::channel();
        let fut = async {
            let val = future.await;
            _ = tx.send(val);
        };

        wasm_bindgen_futures::spawn_local(fut);
        let inner = InnerJoinHandle::CustomHandle {
            inner: Some(rx),
            handle: abort_handle,
        };
        JoinHandle { inner }
    }
}

impl ExecutorTimer for WasmExecutor {
    fn sleep(&self, duration: Duration) -> impl Future<Output = ()> + Send + 'static {
        futures_timer::Delay::new(duration)
    }

    fn timeout<F>(
        &self,
        duration: Duration,
        future: F,
    ) -> impl Future<Output = std::io::Result<F::Output>> + Send + 'static
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        futures_timeout::TimeoutExt::timeout(future, duration)
    }
}
