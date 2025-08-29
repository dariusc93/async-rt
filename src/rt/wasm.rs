use crate::{Executor, InnerJoinHandle, JoinHandle};
use futures::future::{AbortHandle, Abortable};
use std::future::Future;

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

impl Executor for WasmExecutor {
    fn spawn_blocking<F, R>(&self, f: F) -> JoinHandle<R>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        // Since wasm does not have any blocking operations, we would just
        // execute the function in the given task and return its underlining value, if any
        // TODO: Maybe check to determine if this is a browser and if so use webworker for offloading
        let fut = async {
            let val = f();
            // we yield back to the executor, so it can make some progress before we return the value here and allow other tasks to continue.
            // not exactly needed
            self::yield_now().await;
            val
        };
        self.spawn(fut)
    }
}