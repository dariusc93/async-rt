use crate::error::TimeoutError;
use crate::{
    AbortableJoinHandle, Executor, ExecutorBlockOn, ExecutorBlocking, ExecutorTimeout, JoinHandle,
};
use std::future::Future;
use std::sync::Arc;

impl<E> Executor for Arc<E>
where
    E: Executor,
{
    fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        (**self).spawn(future)
    }
}

impl<E> ExecutorBlocking for Arc<E>
where
    E: ExecutorBlocking,
{
    fn spawn_blocking<F, T>(&self, future: F) -> JoinHandle<T>
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + 'static,
    {
        (**self).spawn_blocking(future)
    }
}

impl<E> ExecutorTimeout for Arc<E>
where
    E: ExecutorTimeout,
{
    fn spawn_timeout<F>(
        &self,
        duration: std::time::Duration,
        f: F,
    ) -> JoinHandle<Result<F::Output, TimeoutError>>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        (**self).spawn_timeout(duration, f)
    }

    fn spawn_abortable_timeout<F>(
        &self,
        duration: std::time::Duration,
        f: F,
    ) -> AbortableJoinHandle<Result<F::Output, TimeoutError>>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        (**self).spawn_abortable_timeout(duration, f)
    }
}

impl<E> ExecutorBlockOn for Arc<E>
where
    E: ExecutorBlockOn,
{
    fn block_on<F: Future>(&self, f: F) -> F::Output {
        (**self).block_on(f)
    }
}
