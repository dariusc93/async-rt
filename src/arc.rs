use crate::{Executor, JoinHandle, SendBound};
use std::future::Future;
use std::sync::Arc;

impl<E> Executor for Arc<E>
where
    E: Executor,
{
    fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + SendBound + 'static,
        F::Output: SendBound + 'static,
    {
        (**self).spawn(future)
    }
}
