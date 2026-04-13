//! Structured concurrency scopes for async tasks.
//!
//! A [`Scope`] lets you spawn futures that borrow from the enclosing stack
//! frame, analogous to [`std::thread::scope`]. Unlike thread scope, the
//! tasks are driven cooperatively by the scope's own future, they make
//! progress whenever the scope is polled so no runtime-level `spawn` is
//! involved and no `unsafe` is needed to extend lifetimes.
//!
//! The [`scope`] function is the entry point. Inside the async closure,
//! call [`Scope::spawn`] to enqueue a task; the returned [`ScopedJoinHandle`]
//! resolves with the task's output. Any tasks still running when the user
//! closure's future completes are drained before `scope` returns, so every
//! borrow is released before the stack frame goes away.

use crate::{
    AbortableJoinHandle, CommunicationTask, Executor, InnerJoinHandle, JoinHandle,
    UnboundedCommunicationTask,
};
use futures::channel::mpsc::{Receiver, UnboundedReceiver};
use futures::channel::oneshot;
use futures::future::{AbortHandle, Abortable, BoxFuture};
use futures::stream::FuturesUnordered;
use futures::{FutureExt, StreamExt};
use parking_lot::Mutex;
use core::future::{Future, poll_fn};
use core::marker::PhantomData;
use core::pin::Pin;
use core::task::{Context, Poll};

/// A scope within which tasks can be spawned that borrow from the enclosing
/// stack frame.
pub struct Scope<'env> {
    tasks: Mutex<FuturesUnordered<BoxFuture<'env, ()>>>,
    _env: PhantomData<&'env mut &'env ()>,
}

impl<'env> Scope<'env> {
    fn new() -> Self {
        Self {
            tasks: Mutex::new(FuturesUnordered::new()),
            _env: PhantomData,
        }
    }

    /// Spawn a task into this scope.
    ///
    /// The future may borrow any data that outlives its lifetime `'env`. The task will
    /// be polled cooperatively alongside the scope's user closure and any
    /// other spawned tasks.
    pub fn spawn<Fut>(&self, fut: Fut) -> ScopedJoinHandle<Fut::Output>
    where
        Fut: Future + Send + 'env,
        Fut::Output: Send + 'env,
    {
        let (tx, rx) = oneshot::channel();
        let wrapped: BoxFuture<'env, ()> = async move {
            let output = fut.await;
            // If the receiver was dropped, the caller doesn't care about
            // the output so we will discard it.
            let _ = tx.send(output);
        }
        .boxed();

        self.tasks.lock().push(wrapped);

        ScopedJoinHandle { rx }
    }

    /// Spawn a task into this scope and return an [`AbortableJoinHandle`].
    ///
    /// This mirrors [`Executor::spawn_abortable`] for cooperatively
    /// scheduled tasks. The returned handle aborts the task when all
    /// references to it have been dropped.
    pub fn spawn_abortable<Fut>(&self, fut: Fut) -> AbortableJoinHandle<Fut::Output>
    where
        Fut: Future + Send + 'env,
        Fut::Output: Send + 'env,
    {
        let (abort_handle, abort_reg) = AbortHandle::new_pair();
        let abortable = Abortable::new(fut, abort_reg);
        let (tx, rx) = oneshot::channel();

        let wrapped: BoxFuture<'env, ()> = async move {
            let val = abortable.await;
            let _ = tx.send(val);
        }
        .boxed();
        self.tasks.lock().push(wrapped);

        let join = JoinHandle {
            inner: InnerJoinHandle::CustomHandle {
                inner: Some(rx),
                handle: abort_handle,
            },
        };
        AbortableJoinHandle::from(join)
    }

    /// Spawn a task into this scope without keeping a handle to it.
    ///
    /// Equivalent to [`Executor::dispatch`] for scoped tasks.
    pub fn dispatch<Fut>(&self, fut: Fut)
    where
        Fut: Future + Send + 'env,
        Fut::Output: Send + 'env,
    {
        let _ = self.spawn(fut);
    }

    /// Spawn a message-driven coroutine into this scope.
    ///
    /// Equivalent to [`Executor::spawn_coroutine`] for scoped tasks.
    pub fn spawn_coroutine<T, F, Fut>(&self, f: F) -> CommunicationTask<T>
    where
        F: FnMut(Receiver<T>) -> Fut,
        Fut: Future<Output = ()> + Send + 'env,
    {
        self.spawn_coroutine_with_buffer(1, f)
    }

    /// Like [`Scope::spawn_coroutine`] but with a configurable channel
    /// buffer.
    pub fn spawn_coroutine_with_buffer<T, F, Fut>(
        &self,
        buffer: usize,
        mut f: F,
    ) -> CommunicationTask<T>
    where
        F: FnMut(Receiver<T>) -> Fut,
        Fut: Future<Output = ()> + Send + 'env,
    {
        let (tx, rx) = futures::channel::mpsc::channel(buffer);
        let task_handle = self.spawn_abortable(f(rx));
        CommunicationTask::new(task_handle, tx)
    }

    /// Like [`Scope::spawn_coroutine`] but passes a caller-provided
    /// context into the coroutine alongside the message receiver.
    pub fn spawn_coroutine_with_context<T, F, C, Fut>(
        &self,
        context: C,
        f: F,
    ) -> CommunicationTask<T>
    where
        F: FnMut(C, Receiver<T>) -> Fut,
        Fut: Future<Output = ()> + Send + 'env,
    {
        self.spawn_coroutine_with_buffer_and_context(context, 1, f)
    }

    /// Like [`Scope::spawn_coroutine_with_context`] but with a
    /// configurable channel buffer.
    pub fn spawn_coroutine_with_buffer_and_context<T, F, C, Fut>(
        &self,
        context: C,
        buffer: usize,
        mut f: F,
    ) -> CommunicationTask<T>
    where
        F: FnMut(C, Receiver<T>) -> Fut,
        Fut: Future<Output = ()> + Send + 'env,
    {
        let (tx, rx) = futures::channel::mpsc::channel(buffer);
        let task_handle = self.spawn_abortable(f(context, rx));
        CommunicationTask::new(task_handle, tx)
    }

    /// Spawn an unbounded message-driven coroutine into this scope.
    ///
    /// Equivalent to [`Executor::spawn_unbounded_coroutine`] for scoped
    /// tasks.
    pub fn spawn_unbounded_coroutine<T, F, Fut>(
        &self,
        mut f: F,
    ) -> UnboundedCommunicationTask<T>
    where
        F: FnMut(UnboundedReceiver<T>) -> Fut,
        Fut: Future<Output = ()> + Send + 'env,
    {
        let (tx, rx) = futures::channel::mpsc::unbounded();
        let task_handle = self.spawn_abortable(f(rx));
        UnboundedCommunicationTask::new(task_handle, tx)
    }

    /// Like [`Scope::spawn_unbounded_coroutine`] but passes a
    /// caller-provided context into the coroutine.
    pub fn spawn_unbounded_coroutine_with_context<T, F, C, Fut>(
        &self,
        context: C,
        mut f: F,
    ) -> UnboundedCommunicationTask<T>
    where
        F: FnMut(C, UnboundedReceiver<T>) -> Fut,
        Fut: Future<Output = ()> + Send + 'env,
    {
        let (tx, rx) = futures::channel::mpsc::unbounded();
        let task_handle = self.spawn_abortable(f(context, rx));
        UnboundedCommunicationTask::new(task_handle, tx)
    }

    /// Drive the spawned task set, returning `(made_progress, empty)`.
    ///
    /// Polls tasks in a loop until no more immediate progress can be made.
    /// `made_progress` is `true` if at least one task completed this call;
    /// `empty` is `true` if no tasks remain after polling.
    fn drive_tasks(&self, cx: &mut Context<'_>) -> (bool, bool) {
        let mut made_progress = false;
        loop {
            let mut tasks = self.tasks.lock();
            match tasks.poll_next_unpin(cx) {
                Poll::Ready(Some(())) => {
                    made_progress = true;
                    continue;
                }
                Poll::Ready(None) => return (made_progress, true),
                Poll::Pending => return (made_progress, false),
            }
        }
    }
}

/// A handle to a task spawned on a [`Scope`].
///
/// Awaiting the handle yields the task's output. If the scope is dropped
/// before the task finishes (for example, because the scope future was
/// cancelled), awaiting yields [`JoinError::Cancelled`].
///
/// # Note
/// The handle is `'static` so it can  be moved into other spawned tasks, channels, or futures
/// without lifetime gymnastics.
pub struct ScopedJoinHandle<T> {
    rx: oneshot::Receiver<T>,
}

/// Error returned when a scoped task was cancelled before it produced an output
#[derive(Debug)]
pub enum JoinError {
    /// The task was cancelled before it produced an output.
    Cancelled,
}

impl core::fmt::Display for JoinError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JoinError::Cancelled => f.write_str("scoped task was cancelled"),
        }
    }
}

impl core::error::Error for JoinError {}

impl<T> Future for ScopedJoinHandle<T> {
    type Output = Result<T, JoinError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match Pin::new(&mut self.rx).poll(cx) {
            Poll::Ready(Ok(v)) => Poll::Ready(Ok(v)),
            Poll::Ready(Err(_)) => Poll::Ready(Err(JoinError::Cancelled)),
            Poll::Pending => Poll::Pending,
        }
    }
}

/// Create a scope for spawning tasks that borrow from the enclosing stack
/// frame.
///
/// The provided async closure receives a reference to the [`Scope`] and
/// returns a future. That future is driven alongside any tasks spawned on
/// the scope; when it completes, any still-running spawned tasks are
/// drained before `scope` returns its value.
///
/// # Example
///
/// ```no_run
/// # async fn run() {
/// use async_rt::scoped::scope;
///
/// let data = vec![1, 2, 3, 4];
/// let data = &data;
/// let sum = scope(async |s| {
///     let a = s.spawn(async { data[0] + data[1] });
///     let b = s.spawn(async { data[2] + data[3] });
///     a.await.unwrap() + b.await.unwrap()
/// })
/// .await;
/// assert_eq!(sum, 10);
/// # }
/// ```
pub async fn scope<'env, F, T>(f: F) -> T
where
    F: AsyncFnOnce(&Scope<'env>) -> T,
{
    let scope: Scope<'env> = Scope::new();
    let result = {
        let user_fut = f(&scope);
        let mut user_fut = std::pin::pin!(user_fut);

        poll_fn(|cx| {
            loop {
                if let Poll::Ready(r) = user_fut.as_mut().poll(cx) {
                    return Poll::Ready(r);
                }
                let (made_progress, _empty) = scope.drive_tasks(cx);
                if !made_progress {
                    return Poll::Pending;
                }
            }
        })
        .await
    };

    // Drain any remaining spawned tasks before the scope goes away, so
    // every borrow of the data is released before this frame unwinds.
    poll_fn(|cx| {
        let (_made_progress, empty) = scope.drive_tasks(cx);
        if empty {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    })
    .await;

    result
}

/// An [`Executor`] wrapper that tracks spawned tasks so they can be
/// cancelled when a scope ends.
///
/// `ScopeExecutor` itself implements [`Executor`], so it can be passed to
/// any code that takes `Executor` every spawn performed through
/// that code will be tracked by the scope.
///
/// Because submitted futures must still be `Send + 'static` (enforced by
/// [`Executor::spawn`]), this flavour of scope does *not* allow borrowing
/// from the enclosing stack frame. Use [`scope`] for that.
pub struct ScopeExecutor<'scope, E> {
    inner: &'scope E,
    task_handles: Mutex<Vec<JoinHandle<()>>>,
    _scope: PhantomData<&'scope mut &'scope ()>,
}

impl<'scope, E> ScopeExecutor<'scope, E> {
    fn new(inner: &'scope E) -> Self {
        Self {
            inner,
            task_handles: Mutex::new(Vec::new()),
            _scope: PhantomData,
        }
    }

    /// Abort every task currently tracked by this scope.
    fn abort_all(&self) {
        for handle in self.task_handles.lock().iter() {
            handle.abort();
        }
    }
}

impl<E> Drop for ScopeExecutor<'_, E> {
    fn drop(&mut self) {
        self.abort_all();
    }
}

impl<'scope, E> Executor for ScopeExecutor<'scope, E>
where
    E: Executor,
{
    fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        let (abort_handle, abort_registration) = AbortHandle::new_pair();
        let abortable = Abortable::new(future, abort_registration);
        let (tx, rx) = oneshot::channel();
        let wrapped = async move {
            let val = abortable.await;
            let _ = tx.send(val);
        };

        let task_handle = self.inner.spawn(wrapped);
        self.task_handles.lock().push(task_handle);

        JoinHandle {
            inner: InnerJoinHandle::CustomHandle {
                inner: Some(rx),
                handle: abort_handle,
            },
        }
    }
}

/// Run an async closure with a scoped [`Executor`] wrapper.
///
/// If the future containing `executor_scope` itself is dropped
/// (canceled externally), it will abort every outstanding task.
///
/// Unlike [`scope`], futures spawned here must be `Send + 'static`
/// they're going onto the real executor, so they can't borrow from the
/// enclosing stack frame.
///
/// # Panics in spawned tasks
///
/// Panics inside spawned tasks are **not** propagated to the scope.
/// Unlike [`std::thread::scope`], `executor_scope` does not re-raise
/// panics collected from its tasks: the panic is caught at the task
/// boundary by the underlying runtime, and the scope simply treats the
/// task as finished.
///
/// If you need to react to a task panic, poll or `.await` the returned
/// [`JoinHandle`] yourself and check its result and don't rely on the
/// scope to surface it.
///
/// # Example
///
/// ```no_run
/// # async fn run() {
/// use async_rt::Executor;
/// use async_rt::rt::tokio::TokioExecutor;
///
/// let executor = TokioExecutor;
/// let total = executor
///     .executor_scope(async |s| {
///         let a = s.spawn(async { 1 + 2 });
///         let b = s.spawn(async { 3 + 4 });
///         a.await.unwrap() + b.await.unwrap()
///     })
///     .await;
/// assert_eq!(total, 10);
/// # }
/// ```
pub async fn executor_scope<'scope, E, F, T>(executor: &'scope E, f: F) -> T
where
    E: Executor,
    F: AsyncFnOnce(&ScopeExecutor<'scope, E>) -> T,
{
    let scope_exec = ScopeExecutor::new(executor);
    let result = f(&scope_exec).await;

    // Wait for every spawned task to complete
    let handles: Vec<_> = scope_exec.task_handles.lock().drain(..).collect();
    for handle in handles {
        let _ = handle.await;
    }

    result
}

#[cfg(test)]
mod tests {
    use std::time::Duration;
    use futures_timer::Delay;
    use super::*;

    #[tokio::test]
    async fn borrows_stack_data() {
        let data = vec![1, 2, 3, 4];
        let data = &data;
        let sum = scope(async |s: &Scope<'_>| {
            let a = s.spawn(async move { data[0] + data[1] });
            let b = s.spawn(async move { data[2] + data[3] });
            a.await.unwrap() + b.await.unwrap()
        })
        .await;
        assert_eq!(sum, 10);
    }

    #[tokio::test]
    async fn drains_unawaited_tasks() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let counter = AtomicUsize::new(0);
        let counter_ref = &counter;
        scope(async |s: &Scope<'_>| {
            for _ in 0..8 {
                s.spawn(async move {
                    counter_ref.fetch_add(1, Ordering::SeqCst);
                });
            }
        })
        .await;
        assert_eq!(counter.load(Ordering::SeqCst), 8);
    }

    #[tokio::test]
    async fn returns_closure_value() {
        let v: i32 = scope(async |_s: &Scope<'_>| 42).await;
        assert_eq!(v, 42);
    }

    #[tokio::test]
    async fn join_handle_yields_output() {
        let out = scope(async |s: &Scope<'_>| {
            let h = s.spawn(async { "hello" });
            h.await.unwrap()
        })
        .await;
        assert_eq!(out, "hello");
    }

    #[tokio::test]
    async fn many_concurrent_tasks_complete() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let counter = AtomicUsize::new(0);
        let counter_ref = &counter;
        let total: usize = scope(async |s: &Scope<'_>| {
            let handles: Vec<_> = (0..32)
                .map(|i| {
                    s.spawn(async move {
                        counter_ref.fetch_add(1, Ordering::SeqCst);
                        i
                    })
                })
                .collect();
            let mut sum = 0usize;
            for h in handles {
                sum += h.await.unwrap();
            }
            sum
        })
        .await;
        assert_eq!(total, (0..32).sum());
        assert_eq!(counter.load(Ordering::SeqCst), 32);
    }

    #[tokio::test]
    async fn executor_scope_runs_tasks() {
        use crate::rt::tokio::TokioExecutor;
        let executor = TokioExecutor;
        let total = executor
            .executor_scope(async |s| {
                let a = s.spawn(async { 1 + 2 });
                let b = s.spawn(async { 3 + 4 });
                a.await.unwrap() + b.await.unwrap()
            })
            .await;
        assert_eq!(total, 10);
    }

    #[tokio::test]
    async fn scope_spawn_coroutine_receives_messages() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let total = AtomicUsize::new(0);
        let total_ref = &total;

        scope(async |s: &Scope<'_>| {
            let mut task = s.spawn_coroutine(|mut rx| async move {
                while let Some(value) = rx.next().await {
                    total_ref.fetch_add(value, Ordering::SeqCst);
                }
            });
            for v in [1usize, 2, 3, 4] {
                task.send(v).await.unwrap();
            }
            drop(task); // closes channel → coroutine exits cleanly
        })
        .await;

        assert_eq!(total.load(Ordering::SeqCst), 10);
    }

    #[tokio::test]
    async fn scope_dispatch_runs_fire_and_forget() {
        use std::sync::atomic::{AtomicBool, Ordering};
        let flag = AtomicBool::new(false);
        let flag_ref = &flag;

        scope(async |s: &Scope<'_>| {
            s.dispatch(async move {
                flag_ref.store(true, Ordering::SeqCst);
            });
        })
        .await;

        assert!(flag.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn executor_scope_drains_unawaited_tasks() {
        use crate::rt::tokio::TokioExecutor;
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};

        let executor = TokioExecutor;
        let flag = Arc::new(AtomicBool::new(false));
        let flag_clone = flag.clone();

        executor
            .executor_scope(async move |s| {
                // Spawn a task but do NOT await its handle.
                // executor_scope should wait for it to complete before
                // returning.
                let _h = s.spawn(async move {
                    Delay::new(Duration::from_millis(50)).await;
                    flag_clone.store(true, Ordering::SeqCst);
                });
            })
            .await;

        assert!(
            flag.load(Ordering::SeqCst),
            "unawaited task should have completed before executor_scope returned"
        );
    }

    #[tokio::test]
    async fn executor_scope_swallows_task_panic() {
        use crate::rt::tokio::TokioExecutor;
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};

        let executor = TokioExecutor;
        let sibling_done = Arc::new(AtomicUsize::new(0));
        let sibling_done_clone = sibling_done.clone();

        let result = executor
            .executor_scope(async move |s| {
                // Task A: panics. Its JoinHandle is dropped without
                // being awaited. the panic goes to the runtime.
                let _panicker = s.spawn(async {
                    panic!("deliberate test panic");
                });
                // Task B: completes normally. The scope should still
                // drain it before returning.
                let _sibling = s.spawn(async move {
                    sibling_done_clone.fetch_add(1, Ordering::SeqCst);
                });
                42usize
            })
            .await;

        assert_eq!(result, 42);
        assert_eq!(sibling_done.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn executor_scope_aborts_on_external_cancel() {
        use crate::rt::tokio::TokioExecutor;
        use futures::future::Either;
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};

        let executor = TokioExecutor;
        let flag = Arc::new(AtomicBool::new(false));
        let flag_clone = flag.clone();

        {
            let scope_fut = executor.executor_scope(async move |s| {
                let _h = s.spawn(async move {
                    Delay::new(Duration::from_millis(200)).await;
                    flag_clone.store(true, Ordering::SeqCst);
                });
                futures::future::pending::<()>().await;
            });

            let scope_fut = std::pin::pin!(scope_fut);
            let timer = std::pin::pin!(Delay::new(Duration::from_millis(30)));
            let result = futures::future::select(scope_fut, timer).await;
            assert!(
                matches!(result, Either::Right(_)),
                "timer should have won the race"
            );
        }

        // Give the (supposedly aborted) task plenty of time.
        Delay::new(Duration::from_millis(300)).await;
        assert!(
            !flag.load(Ordering::SeqCst),
            "task should have been aborted by scope drop"
        );
    }
}
