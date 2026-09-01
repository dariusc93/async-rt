mod executor;

use std::fmt::Debug;
use async_rt::{ExecutorBlockOn, ExecutorTimeout, JoinHandle};
use crate::{Executor, ExecutorBlocking};
pub use crate::global::executor::DefaultExecutor;

pub struct ConfiguredExecutor<E = DefaultExecutor> {
    executor: E,
}

impl<E: Default> Default for ConfiguredExecutor<E> {
    fn default() -> Self {
        Self { executor: E::default() }
    }
}

impl<E: Clone> Clone for ConfiguredExecutor<E> {
    fn clone(&self) -> Self {
        Self { executor: self.executor.clone() }
    }
}

impl<E: Debug> Debug for ConfiguredExecutor<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConfiguredExecutor")
            .field("executor", &self.executor)
            .finish()
    }
}

impl<E> ConfiguredExecutor<E> {
    pub fn new(executor: E) -> Self {
        Self { executor }
    }

    pub fn executor(&self) -> &E {
        &self.executor
    }

    pub fn into_executor(self) -> E {
        self.executor
    }
}

impl<E: Executor> Executor for ConfiguredExecutor<E> {
    fn runtime_type(&self) -> Option<&'static str> {
        self.executor.runtime_type()
    }

    fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static
    {
        self.executor.spawn(future)
    }
}

impl<E: ExecutorTimeout> ExecutorTimeout for ConfiguredExecutor<E> {}
impl<E: ExecutorBlocking> ExecutorBlocking for ConfiguredExecutor<E> {
    fn spawn_blocking<F, R>(&self, f: F) -> JoinHandle<R>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static
    {
        self.executor.spawn_blocking(f)
    }
}

impl<E: ExecutorBlockOn> ExecutorBlockOn for ConfiguredExecutor<E> {
    fn block_on<F: Future>(&self, future: F) -> F::Output {
        self.executor.block_on(future)
    }
}