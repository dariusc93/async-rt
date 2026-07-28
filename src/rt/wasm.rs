use crate::{
    CompletionGuard, Executor, ExecutorBlocking, ExecutorTimeout, InnerJoinHandle, JoinHandle,
    abortable_result,
};
use futures::future::AbortHandle;
use pollable_map::optional::Optional;
use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

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
        let future = abortable_result(future, abort_registration);
        let (tx, rx) = futures::channel::oneshot::channel();
        let finished = Arc::new(AtomicBool::new(false));
        let completion = CompletionGuard::new(finished.clone());
        let fut = async move {
            let _completion = completion;
            let val = future.await;
            _ = tx.send(val);
        };

        wasm_bindgen_futures::spawn_local(fut);
        let inner = InnerJoinHandle::CustomHandle {
            inner: Optional::new(rx),
            handle: abort_handle,
            finished,
        };
        JoinHandle { inner }
    }
}

impl ExecutorBlocking for WasmExecutor {
    fn spawn_blocking<F, R>(&self, f: F) -> JoinHandle<R>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        let fut = async move {
            let _handle = wasm_thread::spawn(f);
            _handle.join_async().await.expect("shouldn't panic")
        };
        self.spawn(fut)
    }
}

impl ExecutorTimeout for WasmExecutor {}
