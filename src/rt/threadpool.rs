use std::fmt::{Debug, Formatter};
use std::future::Future;
use futures::future::{AbortHandle, Abortable};
use crate::{Executor, InnerJoinHandle, JoinHandle};

#[derive(Clone)]
pub struct ThreadPoolExecutor {
    pool: futures::executor::ThreadPool,
}

impl Debug for ThreadPoolExecutor {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ThreadPoolExecutor").finish()
    }
}

impl Default for ThreadPoolExecutor {
    fn default() -> Self {
        Self {
            pool: futures::executor::ThreadPool::new().unwrap(),
        }
    }
}

impl Executor for ThreadPoolExecutor {
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
            let _ = tx.send(val);
        };

        self.pool.spawn_ok(fut);
        let inner = InnerJoinHandle::CustomHandle {
            inner: Some(rx),
            handle: abort_handle,
        };

        JoinHandle { inner }
    }
}