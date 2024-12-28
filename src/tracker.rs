use std::fmt::Debug;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::task::{Context, Poll};
use crate::{Executor, JoinHandle};

/// Track running tasks. 
/// 
/// Note that there is no guarantee that the runtime would drop the future after it is done, therefore
/// this should only be used for purely approx statistics and not actual numbers. Additionally,
/// it does not track any tasks spawned directly by the runtime but only by [`Executor::spawn`] through
/// this struct.
pub struct TrackerExecutor<E> {
    executor: E,
    counter: Arc<AtomicUsize>
}

impl<E> Debug for TrackerExecutor<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TrackerExecutor").finish()
    }
}

impl<E: Executor> TrackerExecutor<E> {
    pub fn new(executor: E) -> Self {
        Self {
            executor,
            counter: Arc::default()
        }
    }
    
    /// Number of active tasks.
    pub fn count(&self) -> usize {
        self.counter.load(std::sync::atomic::Ordering::SeqCst)
    }
}

struct FutureCounter<F> {
    future: F,
    counter: Arc<AtomicUsize>
}

impl<F> FutureCounter<F> {
    pub fn new(future: F, counter: Arc<AtomicUsize>) -> Self {
        counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Self {
            future,
            counter
        }
    }
}

impl<F> Future for FutureCounter<F>
where
    F: Future + 'static + Unpin,
{
    type Output = F::Output;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.future).poll(cx)
    }
}

impl<F> Drop for FutureCounter<F> {
    fn drop(&mut self) {
        self.counter.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
    }
}

impl<E: Executor> Executor for TrackerExecutor<E> {
    fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        let counter = self.counter.clone();
        let future = Box::pin(future);
        let future = FutureCounter::new(future, counter);
        self.executor.spawn(future)
    }
}

#[cfg(test)]
mod tests {
    use std::future::Future;
    use std::pin::Pin;
    use std::task::{Context, Poll};
    use crate::Executor;
    use crate::rt::tokio::TokioExecutor;
    use super::TrackerExecutor;

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
    
    fn _yield() -> Yield {
        Yield::default()
    }
    
    #[tokio::test]
    async fn test_tracker_executor() {
        let executor = TrackerExecutor::new(TokioExecutor);
        let handle = executor.spawn(futures::future::pending::<()>());
        assert_eq!(executor.count(), 1);
        handle.abort();
        // We yield back to the runtime to allow progress to be made after aborting the task.
        _yield().await;
        assert_eq!(executor.count(), 0);
    }
}