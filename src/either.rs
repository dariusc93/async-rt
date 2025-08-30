use crate::{Executor, ExecutorBlocking, JoinHandle};
use either::Either;
use std::future::Future;

impl<L, R> Executor for Either<L, R>
where
    L: Executor,
    R: Executor,
{
    fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        match self {
            Either::Left(l) => l.spawn(future),
            Either::Right(r) => r.spawn(future),
        }
    }
}

impl<L, R> ExecutorBlocking for Either<L, R> where L: ExecutorBlocking, R: ExecutorBlocking {
    fn spawn_blocking<F, T>(&self, f: F) -> JoinHandle<T>
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + 'static,
    {
        match self {
            Either::Left(l) => l.spawn_blocking(f),
            Either::Right(r) => r.spawn_blocking(f),
        }
    }
}