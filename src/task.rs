use crate::communication::CommunicationHandle;
use crate::error::TimeoutError;
use crate::global::GlobalExecutor;
use crate::{
    AbortableJoinHandle, CommunicationTask, Executor, ExecutorBlocking, ExecutorTimeout,
    JoinHandle, Scope, ScopeExecutor, UnboundedCommunicationTask,
};
use futures::channel::mpsc::{Receiver, UnboundedReceiver};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

static EXECUTOR: GlobalExecutor = GlobalExecutor;

/// Spawns a new asynchronous task in the background, returning a Future [`JoinHandle`] for it.
pub fn spawn<F>(future: F) -> JoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    EXECUTOR.spawn(future)
}

pub fn spawn_blocking<F, T>(future: F) -> JoinHandle<T>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    EXECUTOR.spawn_blocking(future)
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

/// Spawns a new asynchronous task that must complete within `duration`.
///
/// If it does not, the future is dropped and the task completes with [`TimeoutError`].
pub fn spawn_timeout<F>(
    duration: std::time::Duration,
    future: F,
) -> JoinHandle<Result<F::Output, TimeoutError>>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    EXECUTOR.spawn_timeout(duration, future)
}

/// Spawns a new asynchronous task, returning an abortable handle, that must complete within
/// `duration`.
///
/// If it does not, the future is dropped and the task completes with [`TimeoutError`].
pub fn spawn_abortable_timeout<F>(
    duration: std::time::Duration,
    future: F,
) -> AbortableJoinHandle<Result<F::Output, TimeoutError>>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    EXECUTOR.spawn_abortable_timeout(duration, future)
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

/// Spawns a new asynchronous task that accepts messages to the task.
/// This function returns a handle that allows sending a message, or if there is no reference to the handle at all
/// (in other words, all handles are dropped), the task would be aborted.
pub fn spawn_coroutine<In, Out, F, Fut>(f: F) -> CommunicationTask<In, Out>
where
    F: FnMut(&CommunicationHandle<Out>, In) -> Fut + Send + 'static,
    Fut: Future<Output = ()> + Send + 'static,
    In: Send + 'static,
    Out: Send + 'static,
{
    EXECUTOR.spawn_coroutine(f)
}

/// Spawns a new asynchronous task that accepts messages to the task with a set buffer.
/// This function returns a handle that allows sending a message, or if there is no reference to the handle at all
/// (in other words, all handles are dropped), the task would be aborted.
pub fn spawn_coroutine_with_buffer<In, Out, F, Fut>(
    buffer: usize,
    f: F,
) -> CommunicationTask<In, Out>
where
    F: FnMut(&CommunicationHandle<Out>, In) -> Fut + Send + 'static,
    Fut: Future<Output = ()> + Send + 'static,
    In: Send + 'static,
    Out: Send + 'static,
{
    EXECUTOR.spawn_coroutine_with_buffer(buffer, f)
}

/// Spawns a new asynchronous task that accepts unbounded messages to the task.
/// This function returns a handle that allows sending a message, or if there is no reference to the handle at all
/// (in other words, all handles are dropped), the task would be aborted.
pub fn spawn_unbounded_coroutine<In, Out, F, Fut>(f: F) -> UnboundedCommunicationTask<In, Out>
where
    F: FnMut(&CommunicationHandle<Out>, In) -> Fut + Send + 'static,
    Fut: Future<Output = ()> + Send + 'static,
    In: Send + 'static,
    Out: Send + 'static,
{
    EXECUTOR.spawn_unbounded_coroutine(f)
}

/// Spawns a new asynchronous task with provided context that accepts messages to the task.
/// This function returns a handle that allows sending a message, or if there is no reference to the handle at all
/// (in other words, all handles are dropped), the task would be aborted.
///
/// # Note
/// If state must be borrowed across awaits,
/// use [`spawn_coroutine_with_receiver_and_context`].
pub fn spawn_coroutine_with_context<In, Out, C, F, Fut>(
    context: C,
    f: F,
) -> CommunicationTask<In, Out>
where
    F: FnMut(&CommunicationHandle<Out>, &mut C, In) -> Fut + Send + 'static,
    Fut: Future<Output = ()> + Send + 'static,
    C: Send + 'static,
    In: Send + 'static,
    Out: Send + 'static,
{
    EXECUTOR.spawn_coroutine_with_context(context, f)
}

/// Spawns a new asynchronous task with provided context that accepts messages to the task with a set buffer.
/// This function returns a handle that allows sending a message, or if there is no reference to the handle at all
/// (in other words, all handles are dropped), the task would be aborted.
pub fn spawn_coroutine_with_buffer_and_context<In, Out, C, F, Fut>(
    context: C,
    buffer: usize,
    f: F,
) -> CommunicationTask<In, Out>
where
    F: FnMut(&CommunicationHandle<Out>, &mut C, In) -> Fut + Send + 'static,
    Fut: Future<Output = ()> + Send + 'static,
    C: Send + 'static,
    In: Send + 'static,
    Out: Send + 'static,
{
    EXECUTOR.spawn_coroutine_with_buffer_and_context(context, buffer, f)
}

/// Spawns a new asynchronous task with provided context that accepts unbounded messages to the task.
/// This function returns a handle that allows sending a message, or if there is no reference to the handle at all
/// (in other words, all handles are dropped), the task would be aborted.
pub fn spawn_unbounded_coroutine_with_context<In, Out, C, F, Fut>(
    context: C,
    f: F,
) -> UnboundedCommunicationTask<In, Out>
where
    F: FnMut(&CommunicationHandle<Out>, &mut C, In) -> Fut + Send + 'static,
    Fut: Future<Output = ()> + Send + 'static,
    C: Send + 'static,
    In: Send + 'static,
    Out: Send + 'static,
{
    EXECUTOR.spawn_unbounded_coroutine_with_context(context, f)
}

/// Spawns a new asynchronous task that accepts messages to the task using [`channels`](futures::channel::mpsc).
/// This function returns a handle that allows sending a message, or if there is no reference to the handle at all
/// (in other words, all handles are dropped), the task would be aborted.
pub fn spawn_coroutine_with_receiver<In, Out, F, Fut>(f: F) -> CommunicationTask<In, Out>
where
    F: FnMut(CommunicationHandle<Out>, Receiver<In>) -> Fut + Send + 'static,
    Fut: Future<Output = ()> + Send + 'static,
    In: Send + 'static,
    Out: Send + 'static,
{
    EXECUTOR.spawn_coroutine_with_receiver(f)
}

/// Spawns a new asynchronous task with a set channel buffer that accepts messages to the task using [`channels`](futures::channel::mpsc).
/// This function returns a handle that allows sending a message, or if there is no reference to the handle at all
/// (in other words, all handles are dropped), the task would be aborted.
pub fn spawn_coroutine_with_receiver_and_buffer<In, Out, F, Fut>(
    buffer: usize,
    f: F,
) -> CommunicationTask<In, Out>
where
    F: FnMut(CommunicationHandle<Out>, Receiver<In>) -> Fut + Send + 'static,
    Fut: Future<Output = ()> + Send + 'static,
    In: Send + 'static,
    Out: Send + 'static,
{
    EXECUTOR.spawn_coroutine_with_receiver_and_buffer(buffer, f)
}

/// Spawns a new asynchronous task with provided context that accepts messages to the task using [`channels`](futures::channel::mpsc).
/// This function returns a handle that allows sending a message, or if there is no reference to the handle at all
/// (in other words, all handles are dropped), the task would be aborted.
pub fn spawn_coroutine_with_receiver_and_context<In, Out, F, C, Fut>(
    context: C,
    f: F,
) -> CommunicationTask<In, Out>
where
    F: FnMut(CommunicationHandle<Out>, C, Receiver<In>) -> Fut + Send + 'static,
    Fut: Future<Output = ()> + Send + 'static,
    In: Send + 'static,
    Out: Send + 'static,
{
    EXECUTOR.spawn_coroutine_with_receiver_and_context(context, f)
}

/// Spawns a new asynchronous task with a set channel buffer and provided context that accepts messages to the task using [`channels`](futures::channel::mpsc).
/// This function returns a handle that allows sending a message, or if there is no reference to the handle at all
/// (in other words, all handles are dropped), the task would be aborted.
pub fn spawn_coroutine_with_receiver_buffer_and_context<In, Out, F, C, Fut>(
    context: C,
    buffer: usize,
    f: F,
) -> CommunicationTask<In, Out>
where
    F: FnMut(CommunicationHandle<Out>, C, Receiver<In>) -> Fut + Send + 'static,
    Fut: Future<Output = ()> + Send + 'static,
    In: Send + 'static,
    Out: Send + 'static,
{
    EXECUTOR.spawn_coroutine_with_receiver_buffer_and_context(context, buffer, f)
}

/// Spawns a new asynchronous task that accepts messages to the task using [`channels`](futures::channel::mpsc).
/// This function returns a handle that allows sending a message, or if there is no reference to the handle at all
/// (in other words, all handles are dropped), the task would be aborted.
pub fn spawn_unbounded_coroutine_with_receiver<In, Out, F, Fut>(
    f: F,
) -> UnboundedCommunicationTask<In, Out>
where
    F: FnMut(CommunicationHandle<Out>, UnboundedReceiver<In>) -> Fut + Send + 'static,
    Fut: Future<Output = ()> + Send + 'static,
    In: Send + 'static,
    Out: Send + 'static,
{
    EXECUTOR.spawn_unbounded_coroutine_with_receiver(f)
}

/// Spawns a new asynchronous task with provided context that accepts messages to the task using [`channels`](futures::channel::mpsc).
/// This function returns a handle that allows sending a message, or if there is no reference to the handle at all
/// (in other words, all handles are dropped), the task would be aborted.
pub fn spawn_unbounded_coroutine_with_receiver_and_context<In, Out, F, C, Fut>(
    context: C,
    f: F,
) -> UnboundedCommunicationTask<In, Out>
where
    F: FnMut(CommunicationHandle<Out>, C, UnboundedReceiver<In>) -> Fut + Send + 'static,
    Fut: Future<Output = ()> + Send + 'static,
    In: Send + 'static,
    Out: Send + 'static,
{
    EXECUTOR.spawn_unbounded_coroutine_with_receiver_and_context(context, f)
}

/// Create a structured-concurrency scope in which tasks may be spawned
/// that borrow from the enclosing stack frame.
///
/// This is the async analogue of [`std::thread::scope`].
pub fn scope<'env, F, T>(f: F) -> impl Future<Output = T>
where
    F: for<'scope> AsyncFnOnce(&'scope Scope<'scope, 'env>) -> T,
{
    EXECUTOR.scope(f)
}

/// Run an async closure with a scoped [`Executor`] wrapper that
/// forwards spawns to this executor, waits for all spawned tasks
/// to finish when the closure returns, and aborts any outstanding
/// tasks if the scope future itself is cancelled.
pub fn executor_scope<F, T>(f: F) -> impl Future<Output = T>
where
    F: AsyncFnOnce(&ScopeExecutor<'static, GlobalExecutor>) -> T,
{
    EXECUTOR.executor_scope(f)
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
