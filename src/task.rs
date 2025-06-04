use crate::global::GlobalExecutor;
use crate::{
    AbortableJoinHandle, CommunicationTask, Executor, JoinHandle, UnboundedCommunicationTask,
};
use futures::channel::mpsc::{Receiver, UnboundedReceiver};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

const EXECUTOR: GlobalExecutor = GlobalExecutor;

/// Spawns a new asynchronous task in the background, returning a Future [`JoinHandle`] for it.
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

/// Spawns a new asynchronous task in the background without a handle.
/// Basically the same as [`spawn`].
pub fn dispatch<F>(future: F)
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    EXECUTOR.dispatch(future);
}

/// Spawns a new asynchronous task that accepts messages to the task using [`channels`](futures::channel::mpsc).
/// This function returns a handle that allows sending a message, or if there is no reference to the handle at all
/// (in other words, all handles are dropped), the task would be aborted.
pub fn spawn_coroutine<T, F, Fut>(f: F) -> CommunicationTask<T>
where
    F: FnMut(Receiver<T>) -> Fut,
    Fut: Future<Output = ()> + Send + 'static,
{
    EXECUTOR.spawn_coroutine(f)
}

/// Spawns a new asynchronous task with provided context that accepts messages to the task using [`channels`](futures::channel::mpsc).
/// This function returns a handle that allows sending a message, or if there is no reference to the handle at all
/// (in other words, all handles are dropped), the task would be aborted.
pub fn spawn_coroutine_with_context<T, F, C, Fut>(context: C, f: F) -> CommunicationTask<T>
where
    F: FnMut(C, Receiver<T>) -> Fut,
    Fut: Future<Output = ()> + Send + 'static,
{
    EXECUTOR.spawn_coroutine_with_context(context, f)
}

/// Spawns a new asynchronous task that accepts messages to the task using [`channels`](futures::channel::mpsc).
/// This function returns a handle that allows sending a message, or if there is no reference to the handle at all
/// (in other words, all handles are dropped), the task would be aborted.
pub fn spawn_unbounded_coroutine<T, F, Fut>(f: F) -> UnboundedCommunicationTask<T>
where
    F: FnMut(UnboundedReceiver<T>) -> Fut,
    Fut: Future<Output = ()> + Send + 'static,
{
    EXECUTOR.spawn_unbounded_coroutine(f)
}

/// Spawns a new asynchronous task with provided context that accepts messages to the task using [`channels`](futures::channel::mpsc).
/// This function returns a handle that allows sending a message, or if there is no reference to the handle at all
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

#[derive(Default)]
struct Yield {
    yielded: bool,
}

impl Future for Yield {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if self.yielded {
            return Poll::Ready(());
        }
        self.yielded = true;
        cx.waker().wake_by_ref();
        Poll::Pending
    }
}

/// Yields execution back to the runtime
pub fn yield_now() -> impl Future<Output = ()> {
    Yield::default()
}
