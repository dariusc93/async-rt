//! Structured concurrency scopes for async tasks.
//!
//! A [`Scope`] lets you spawn futures that borrow from the enclosing stack
//! frame, analogous to [`std::thread::scope`]. Unlike thread scope, the
//! tasks are driven cooperatively by the scope's own future — they make
//! progress whenever the scope is polled — so no runtime-level `spawn` is
//! involved and no `unsafe` is needed to extend lifetimes.
//!
//! The [`scope`] function is the entry point. Inside the async closure,
//! call [`Scope::spawn`] to enqueue a task; the returned [`ScopedJoinHandle`]
//! resolves with the task's output. Any tasks still running when the user
//! closure's future completes are drained before `scope` returns, so every
//! borrow is released before the stack frame goes away.

use futures::channel::oneshot;
use futures::future::BoxFuture;
use futures::stream::FuturesUnordered;
use futures::{FutureExt, StreamExt};
use parking_lot::Mutex;
use std::future::{Future, poll_fn};
use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Context, Poll};

/// A scope within which tasks can be spawned that borrow from the enclosing
/// stack frame.
///
/// Obtain a `Scope` via the [`scope`] free function. The `'env` lifetime is
/// the lifetime of data borrowed from outside the scope — spawned futures
/// may reference any data that outlives `'env`.
pub struct Scope<'env> {
    tasks: Mutex<FuturesUnordered<BoxFuture<'env, ()>>>,
    // Invariant in 'env: prevents the compiler from shrinking or extending
    // 'env, which would otherwise let callers smuggle references in or out.
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
    /// The future may borrow any data that outlives `'env`. The task will
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
            // the output — just discard the send error.
            let _ = tx.send(output);
        }
        .boxed();

        self.tasks.lock().push(wrapped);

        ScopedJoinHandle { rx }
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
/// The handle is `'static` — it carries no borrow of the scope — so it can
/// be moved into other spawned tasks, channels, or futures without lifetime
/// gymnastics.
pub struct ScopedJoinHandle<T> {
    rx: oneshot::Receiver<T>,
}

/// Error returned when a scoped task could not produce a value — because
/// it was cancelled (its scope was dropped before it finished).
#[derive(Debug)]
pub enum JoinError {
    /// The task was cancelled before it produced an output.
    Cancelled,
}

impl std::fmt::Display for JoinError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JoinError::Cancelled => f.write_str("scoped task was cancelled"),
        }
    }
}

impl std::error::Error for JoinError {}

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
    // `AsyncFnOnce` ties the returned future's lifetime to the `&Scope`
    // argument, so the user future is allowed to borrow `&scope` (e.g. to
    // call `scope.spawn(...)` repeatedly). An ordinary HRTB over
    // `FnOnce(&Scope) -> Fut` can't express this because `Fut` is a single
    // type chosen outside the HRTB.
    F: AsyncFnOnce(&Scope<'env>) -> T,
{
    let scope: Scope<'env> = Scope::new();
    let result = {
        let user_fut = f(&scope);
        let mut user_fut = std::pin::pin!(user_fut);

        // Drive the user future and the spawned task set concurrently.
        // Order matters: poll `user_fut` first so any `h.await` inside it
        // can register its waker on the handle's oneshot channel before
        // we poll the task that will fulfil it. Then drive the tasks; if
        // a task completes, its `tx.send(...)` fires the handle's waker
        // and we loop to re-poll `user_fut` in the same call — otherwise
        // there would be no wakeup source to restart us.
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
    // every borrow of `'env` data is released before this frame unwinds.
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

#[cfg(test)]
mod tests {
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
}
