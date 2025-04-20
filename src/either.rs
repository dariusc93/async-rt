use crate::{Executor, JoinHandle};
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
