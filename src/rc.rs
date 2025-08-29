use crate::{Executor, ExecutorBlocking, JoinHandle};
use std::future::Future;
use std::rc::Rc;

impl<E> Executor for Rc<E>
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

impl<E> ExecutorBlocking for Rc<E> where E: ExecutorBlocking {
    fn spawn_blocking<F, R>(&self, f: F) -> JoinHandle<R>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        (**self).spawn_blocking(f)
    }
}