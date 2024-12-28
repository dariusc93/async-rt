use std::future::Future;
use futures::channel::mpsc::Receiver;
use crate::global::GlobalExecutor;
use crate::{AbortableJoinHandle, CommunicationTask, Executor, JoinHandle};

const EXECUTOR: GlobalExecutor = GlobalExecutor;


/// Spawns a new asynchronous task in the background, returning an Future [`JoinHandle`] for it.
pub fn spawn<F>(future: F) -> JoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static {
    EXECUTOR.spawn(future)
}

/// Spawns a new asynchronous task in the background, returning an abortable handle that will cancel the task
/// once the handle is dropped.
///
/// Note: This function is used if the task is expected to run until the handle is dropped. It is recommended to use
/// [`spawn`] or [`dispatch`] otherwise.
fn spawn_abortable<F>(future: F) -> AbortableJoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    EXECUTOR.spawn_abortable(future)
}

/// Spawns a new asynchronous task in the background without an handle.
/// Basically the same as [`spawn`].
fn dispatch<F>(future: F)
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    EXECUTOR.dispatch(future);
}

/// Spawns a new asynchronous task that accepts messages to the task using [`channels`](futures::channel::mpsc).
/// This function returns an handle that allows sending a message or if there is no reference to the handle at all
/// (in other words, all handles are dropped), the task would be aborted.
fn spawn_coroutine<T, F, Fut>(f: F) -> CommunicationTask<T>
where
    F: FnMut(Receiver<T>) -> Fut,
    Fut: Future<Output = ()> + Send + 'static,
{
    EXECUTOR.spawn_coroutine(f)
}