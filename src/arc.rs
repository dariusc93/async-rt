use crate::{Executor, JoinHandle};
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
