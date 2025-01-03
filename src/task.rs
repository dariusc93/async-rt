use crate::global::GlobalExecutor;
use crate::{
    AbortableJoinHandle, CommunicationTask, Executor, ExecutorTimer, JoinHandle,
    UnboundedCommunicationTask,
};
use futures::channel::mpsc::{Receiver, UnboundedReceiver};
use std::future::Future;
use std::time::Duration;

const EXECUTOR: GlobalExecutor = GlobalExecutor;

/// Spawns a new asynchronous task in the background, returning an Future [`JoinHandle`] for it.
pub fn spawn<F>(future: F) -> JoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    EXECUTOR.spawn(future)
}

/// Spawns a new asynchronous task in the background, returning an abortable handle that will cancel the task
/// once the handle is dropped.
///
/// Note: This function is used if the task is expected to run until the handle is dropped. It is recommended to use
/// [`spawn`] or [`dispatch`] otherwise.
pub fn spawn_abortable<F>(future: F) -> AbortableJoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    EXECUTOR.spawn_abortable(future)
}

/// Spawns a new asynchronous task in the background without an handle.
/// Basically the same as [`spawn`].
pub fn dispatch<F>(future: F)
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    EXECUTOR.dispatch(future);
}

/// Spawns a new asynchronous task that accepts messages to the task using [`channels`](futures::channel::mpsc).
/// This function returns an handle that allows sending a message or if there is no reference to the handle at all
/// (in other words, all handles are dropped), the task would be aborted.
pub fn spawn_coroutine<T, F, Fut>(f: F) -> CommunicationTask<T>
where
    F: FnMut(Receiver<T>) -> Fut,
    Fut: Future<Output = ()> + Send + 'static,
{
    EXECUTOR.spawn_coroutine(f)
}

/// Spawns a new asynchronous task with provided context, that accepts messages to the task using [`channels`](futures::channel::mpsc).
/// This function returns an handle that allows sending a message or if there is no reference to the handle at all
/// (in other words, all handles are dropped), the task would be aborted.
pub fn spawn_coroutine_with_context<T, F, C, Fut>(context: C, f: F) -> CommunicationTask<T>
where
    F: FnMut(C, Receiver<T>) -> Fut,
    Fut: Future<Output = ()> + Send + 'static,
{
    EXECUTOR.spawn_coroutine_with_context(context, f)
}

/// Spawns a new asynchronous task that accepts messages to the task using [`channels`](futures::channel::mpsc).
/// This function returns an handle that allows sending a message or if there is no reference to the handle at all
/// (in other words, all handles are dropped), the task would be aborted.
pub fn spawn_unbounded_coroutine<T, F, Fut>(f: F) -> UnboundedCommunicationTask<T>
where
    F: FnMut(UnboundedReceiver<T>) -> Fut,
    Fut: Future<Output = ()> + Send + 'static,
{
    EXECUTOR.spawn_unbounded_coroutine(f)
}

/// Spawns a new asynchronous task with provided context, that accepts messages to the task using [`channels`](futures::channel::mpsc).
/// This function returns an handle that allows sending a message or if there is no reference to the handle at all
/// (in other words, all handles are dropped), the task would be aborted.
pub fn spawn_unbounded_coroutine_with_context<T, F, C, Fut>(
    context: C,
    f: F,
) -> UnboundedCommunicationTask<T>
where
    F: FnMut(C, UnboundedReceiver<T>) -> Fut,
    Fut: Future<Output = ()> + Send + 'static,
{
    EXECUTOR.spawn_unbounded_coroutine_with_context(context, f)
}

/// Creates a future that waits until duration has elapsed.
pub fn sleep(duration: Duration) -> impl Future<Output = ()> + Send + 'static {
    EXECUTOR.sleep(duration)
}

/// Creates a future that consumes and runs a future to completion before specific duration has
/// elapsed.
pub fn timeout<F>(
    duration: Duration,
    future: F,
) -> impl Future<Output = std::io::Result<F::Output>> + Send + 'static
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    EXECUTOR.timeout(duration, future)
}
