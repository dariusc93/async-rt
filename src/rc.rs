use std::future::Future;
use std::rc::Rc;
use crate::{Executor, JoinHandle, SendBound};

impl<E> Executor for Rc<E> where E: Executor {
    fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + SendBound + 'static,
        F::Output: SendBound + 'static,
    {
        (**self).spawn(future)
    }
}