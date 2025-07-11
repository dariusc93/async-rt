use crate::{Executor, JoinHandle, SendBound};
use either::Either;
use std::future::Future;

impl<L, R> Executor for Either<L, R>
where
    L: Executor,
    R: Executor,
{
    fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + SendBound + 'static,
        F::Output: SendBound + 'static,
    {
        match self {
            Either::Left(l) => l.spawn(future),
            Either::Right(r) => r.spawn(future),
        }
    }
}
