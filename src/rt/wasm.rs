use crate::{Executor, InnerJoinHandle, JoinHandle, SendBound};
use futures::future::{AbortHandle, Abortable};
use std::future::Future;

/// Wasm executor
#[derive(Clone, Copy, Debug, PartialOrd, PartialEq, Eq)]
pub struct WasmExecutor;

impl Executor for WasmExecutor {
    fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + SendBound + 'static,
        F::Output: SendBound + 'static,
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
