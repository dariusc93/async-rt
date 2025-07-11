use std::future::Future;
use std::rc::Rc;
use crate::{Executor, JoinHandle};

impl<E> Executor for Rc<E> where E: Executor {
    fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        (**self).spawn(future)
    }
}