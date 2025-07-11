pub mod arc;
pub mod global;
pub mod rt;
pub mod task;
pub mod tracker;

#[cfg(feature = "either")]
pub mod either;
pub mod rc;

use std::fmt::{Debug, Formatter};

use futures::channel::mpsc::{Receiver, UnboundedReceiver};
use futures::future::{AbortHandle, Aborted};
use futures::SinkExt;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

#[cfg(all(
    not(feature = "threadpool"),
    not(feature = "tokio"),
    not(target_arch = "wasm32")
))]
compile_error!(
    "At least one runtime (i.e 'tokio', 'threadpool', 'wasm-bindgen-futures') must be enabled"
);

/// An owned permission to join on a task (await its termination).
///
/// This can be seen as an equivalent to [`std::thread::JoinHandle`] but for [`Future`] tasks rather than a thread.
/// Note that the task associated with this `JoinHandle` will start running at the time [`Executor::spawn`] is called as
/// well as according to the implemented runtime (i.e. [`tokio`]), even if `JoinHandle` has not been awaited.
///
/// Dropping `JoinHandle` will not abort or cancel the task. In other words, the task will continue to run in the background
/// and any return value will be lost.
///
/// This `struct` is created by the [`Executor::spawn`].
pub struct JoinHandle<T> {
    inner: InnerJoinHandle<T>,
}

impl<T> Debug for JoinHandle<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JoinHandle").finish()
    }
}

enum InnerJoinHandle<T> {
    #[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
    TokioHandle(::tokio::task::JoinHandle<T>),
    #[allow(dead_code)]
    CustomHandle {
        inner: Option<futures::channel::oneshot::Receiver<Result<T, Aborted>>>,
        handle: AbortHandle,
    },
    Empty,
}

impl<T> Default for InnerJoinHandle<T> {
    fn default() -> Self {
        Self::Empty
    }
}

impl<T> JoinHandle<T> {
    /// Provide an empty [`JoinHandle`] with no associated task.
    pub fn empty() -> Self {
        JoinHandle {
            inner: InnerJoinHandle::Empty,
        }
    }
}

impl<T> JoinHandle<T> {
    /// Abort the task associated with the handle.
    pub fn abort(&self) {
        match self.inner {
            #[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
            InnerJoinHandle::TokioHandle(ref handle) => handle.abort(),
            InnerJoinHandle::CustomHandle { ref handle, .. } => handle.abort(),
            InnerJoinHandle::Empty => {}
        }
    }

    /// Check if the task associated with this `JoinHandle` has finished.
    ///
    /// Note that this method can return false even if [`JoinHandle::abort`] has been called on the
    /// task due to the time it may take for the task to cancel.
    pub fn is_finished(&self) -> bool {
        match self.inner {
            #[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
            InnerJoinHandle::TokioHandle(ref handle) => handle.is_finished(),
            InnerJoinHandle::CustomHandle {
                ref handle,
                ref inner,
            } => handle.is_aborted() || inner.is_none(),
            InnerJoinHandle::Empty => true,
        }
    }

    /// Replace the current handle with the provided [`JoinHandle`].
    ///
    /// # Safety
    ///
    /// Note that if this is called with a non-empty handle, the existing task
    /// will not be terminated when it is replaced.
    pub unsafe fn replace(&mut self, mut handle: JoinHandle<T>) {
        self.inner = std::mem::take(&mut handle.inner);
    }

    /// Replace the current handle with the provided [`JoinHandle`].
    ///
    /// # Safety
    ///
    /// Note that if this is called with a non-empty handle, the existing task
    /// will not be terminated when it is replaced.
    pub unsafe fn replace_in_place(&mut self, handle: &mut JoinHandle<T>) {
        self.inner = std::mem::take(&mut handle.inner);
    }
}

impl<T> Future for JoinHandle<T> {
    type Output = std::io::Result<T>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let inner = &mut self.inner;
        match inner {
            #[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
            InnerJoinHandle::TokioHandle(handle) => {
                let fut = futures::ready!(Pin::new(handle).poll(cx));

                match fut {
                    Ok(val) => Poll::Ready(Ok(val)),
                    Err(e) => {
                        let e = std::io::Error::other(e);
                        Poll::Ready(Err(e))
                    }
                }
            }
            InnerJoinHandle::CustomHandle { inner, .. } => {
                let Some(this) = inner.as_mut() else {
                    unreachable!("cannot poll a completed future");
                };

                let fut = futures::ready!(Pin::new(this).poll(cx));
                inner.take();

                match fut {
                    Ok(Ok(val)) => Poll::Ready(Ok(val)),
                    Ok(Err(e)) => {
                        let e = std::io::Error::other(e);
                        Poll::Ready(Err(e))
                    }
                    Err(e) => {
                        let e = std::io::Error::other(e);
                        Poll::Ready(Err(e))
                    }
                }
            }
            InnerJoinHandle::Empty => {
                Poll::Ready(Err(std::io::Error::from(std::io::ErrorKind::Other)))
            }
        }
    }
}

/// The same as [`JoinHandle`] but designed to abort the task when all associated references
/// to the returned `AbortableJoinHandle` have been dropped.
#[derive(Clone)]
pub struct AbortableJoinHandle<T> {
    handle: Arc<InnerHandle<T>>,
}

impl<T> Debug for AbortableJoinHandle<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AbortableJoinHandle").finish()
    }
}

impl<T> From<JoinHandle<T>> for AbortableJoinHandle<T> {
    fn from(handle: JoinHandle<T>) -> Self {
        AbortableJoinHandle {
            handle: Arc::new(InnerHandle {
                inner: parking_lot::Mutex::new(handle),
            }),
        }
    }
}

impl<T> AbortableJoinHandle<T> {
    /// Provide a empty [`AbortableJoinHandle`] with no associated task.
    pub fn empty() -> Self {
        Self {
            handle: Arc::new(InnerHandle {
                inner: parking_lot::Mutex::new(JoinHandle::empty()),
            }),
        }
    }
}

impl<T> AbortableJoinHandle<T> {
    /// See [`JoinHandle::abort`]
    pub fn abort(&self) {
        self.handle.inner.lock().abort();
    }

    /// See [`JoinHandle::is_finished`]
    pub fn is_finished(&self) -> bool {
        self.handle.inner.lock().is_finished()
    }

    /// Replace the current handle with an existing one.
    ///
    /// # Safety
    ///
    /// Note that if this is called with a non-empty handle, the existing task
    /// will not be terminated when it is replaced.
    pub unsafe fn replace(&mut self, inner: AbortableJoinHandle<T>) {
        let current_handle = &mut *self.handle.inner.lock();
        let inner_handle = &mut *inner.handle.inner.lock();
        current_handle.replace_in_place(inner_handle);
    }
}

struct InnerHandle<T> {
    pub inner: parking_lot::Mutex<JoinHandle<T>>,
}

impl<T> Drop for InnerHandle<T> {
    fn drop(&mut self) {
        self.inner.lock().abort();
    }
}

impl<T> Future for AbortableJoinHandle<T> {
    type Output = std::io::Result<T>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let inner = &mut *self.handle.inner.lock();
        Pin::new(inner).poll(cx).map_err(std::io::Error::other)
    }
}

/// A task that accepts messages
pub struct CommunicationTask<T> {
    _task_handle: AbortableJoinHandle<()>,
    _channel_tx: futures::channel::mpsc::Sender<T>,
}

unsafe impl<T: Send> Send for CommunicationTask<T> {}
unsafe impl<T: Send> Sync for CommunicationTask<T> {}

impl<T> Clone for CommunicationTask<T> {
    fn clone(&self) -> Self {
        CommunicationTask {
            _task_handle: self._task_handle.clone(),
            _channel_tx: self._channel_tx.clone(),
        }
    }
}

impl<T> Debug for CommunicationTask<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CommunicationTask").finish()
    }
}

impl<T> CommunicationTask<T>
where
    T: 'static,
{
    /// Send a message to the task
    pub async fn send(&mut self, data: T) -> std::io::Result<()> {
        self._channel_tx
            .send(data)
            .await
            .map_err(std::io::Error::other)
    }

    /// Attempts to send a message to the task, returning an error if the channel is full or closed due to the task being aborted.
    pub fn try_send(&self, data: T) -> std::io::Result<()>
    where
        T: Send + Sync,
    {
        self._channel_tx
            .clone()
            .try_send(data)
            .map_err(std::io::Error::other)
    }

    /// Abort the task
    pub fn abort(mut self) {
        self._channel_tx.close_channel();
        self._task_handle.abort();
    }

    /// Check to determine if the task is active.
    pub fn is_active(&self) -> bool {
        !self._task_handle.is_finished() && !self._channel_tx.is_closed()
    }
}

/// A task that accepts messages
pub struct UnboundedCommunicationTask<T> {
    _task_handle: AbortableJoinHandle<()>,
    _channel_tx: futures::channel::mpsc::UnboundedSender<T>,
}

unsafe impl<T: Send> Send for UnboundedCommunicationTask<T> {}
unsafe impl<T: Send> Sync for UnboundedCommunicationTask<T> {}

impl<T> Clone for UnboundedCommunicationTask<T> {
    fn clone(&self) -> Self {
        UnboundedCommunicationTask {
            _task_handle: self._task_handle.clone(),
            _channel_tx: self._channel_tx.clone(),
        }
    }
}

impl<T> Debug for UnboundedCommunicationTask<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UnboundedCommunicationTask").finish()
    }
}

impl<T> UnboundedCommunicationTask<T>
where
    T: 'static,
{
    /// Send a message to task
    pub fn send(&mut self, data: T) -> std::io::Result<()>
    where
        T: Send + Sync,
    {
        self._channel_tx
            .unbounded_send(data)
            .map_err(std::io::Error::other)
    }

    /// Abort the task
    pub fn abort(self) {
        self._channel_tx.close_channel();
        self._task_handle.abort();
    }

    /// Check to determine if the task is active.
    pub fn is_active(&self) -> bool {
        !self._task_handle.is_finished() && !self._channel_tx.is_closed()
    }
}

pub trait Executor {
    /// Spawns a new asynchronous task in the background, returning a Future [`JoinHandle`] for it.
    fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + SendBound + 'static,
        F::Output: SendBound + 'static;

    /// Spawns a new asynchronous task in the background, returning an abortable handle that will cancel the task
    /// once the handle is dropped.
    ///
    /// Note: This function is used if the task is expected to run until the handle is dropped. It is recommended to use
    /// [`Executor::spawn`] or [`Executor::dispatch`] otherwise.
    fn spawn_abortable<F>(&self, future: F) -> AbortableJoinHandle<F::Output>
    where
        F: Future + SendBound + 'static,
        F::Output: SendBound + 'static,
    {
        let handle = self.spawn(future);
        handle.into()
    }

    /// Spawns a new asynchronous task in the background without an handle.
    /// Basically the same as [`Executor::spawn`].
    fn dispatch<F>(&self, future: F)
    where
        F: Future + SendBound + 'static,
        F::Output: SendBound + 'static,
    {
        self.spawn(future);
    }

    /// Spawns a new asynchronous task that accepts messages to the task using [`channels`](futures::channel::mpsc).
    /// This function returns a handle that allows sending a message, or if there is no reference to the handle at all
    /// (in other words, all handles are dropped), the task would be aborted.
    fn spawn_coroutine<T, F, Fut>(&self, mut f: F) -> CommunicationTask<T>
    where
        F: FnMut(Receiver<T>) -> Fut,
        Fut: Future<Output = ()> + SendBound + 'static,
    {
        let (tx, rx) = futures::channel::mpsc::channel(1);
        let fut = f(rx);
        let _task_handle = self.spawn_abortable(fut);
        CommunicationTask {
            _task_handle,
            _channel_tx: tx,
        }
    }

    /// Spawns a new asynchronous task with provided context that accepts messages to the task using [`channels`](futures::channel::mpsc).
    /// This function returns a handle that allows sending a message, or if there is no reference to the handle at all
    /// (in other words, all handles are dropped), the task would be aborted.
    fn spawn_coroutine_with_context<T, F, C, Fut>(
        &self,
        context: C,
        mut f: F,
    ) -> CommunicationTask<T>
    where
        F: FnMut(C, Receiver<T>) -> Fut,
        Fut: Future<Output = ()> + SendBound + 'static,
    {
        let (tx, rx) = futures::channel::mpsc::channel(1);
        let fut = f(context, rx);
        let _task_handle = self.spawn_abortable(fut);
        CommunicationTask {
            _task_handle,
            _channel_tx: tx,
        }
    }

    /// Spawns a new asynchronous task that accepts messages to the task using [`channels`](futures::channel::mpsc).
    /// This function returns a handle that allows sending a message, or if there is no reference to the handle at all
    /// (in other words, all handles are dropped), the task would be aborted.
    fn spawn_unbounded_coroutine<T, F, Fut>(&self, mut f: F) -> UnboundedCommunicationTask<T>
    where
        F: FnMut(UnboundedReceiver<T>) -> Fut,
        Fut: Future<Output = ()> + SendBound + 'static,
    {
        let (tx, rx) = futures::channel::mpsc::unbounded();
        let fut = f(rx);
        let _task_handle = self.spawn_abortable(fut);
        UnboundedCommunicationTask {
            _task_handle,
            _channel_tx: tx,
        }
    }

    /// Spawns a new asynchronous task with provided context that accepts messages to the task using [`channels`](futures::channel::mpsc).
    /// This function returns a handle that allows sending a message, or if there is no reference to the handle at all
    /// (in other words, all handles are dropped), the task would be aborted.
    fn spawn_unbounded_coroutine_with_context<T, F, C, Fut>(
        &self,
        context: C,
        mut f: F,
    ) -> UnboundedCommunicationTask<T>
    where
        F: FnMut(C, UnboundedReceiver<T>) -> Fut,
        Fut: Future<Output = ()> + SendBound + 'static,
    {
        let (tx, rx) = futures::channel::mpsc::unbounded();
        let fut = f(context, rx);
        let _task_handle = self.spawn_abortable(fut);
        UnboundedCommunicationTask {
            _task_handle,
            _channel_tx: tx,
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub trait SendBound: Send {}

#[cfg(target_arch = "wasm32")]
pub trait SendBound {}

#[cfg(target_arch = "wasm32")]
impl<T> SendBound for T {}

#[cfg(not(target_arch = "wasm32"))]
impl<T: Send> SendBound for T {}


#[cfg(test)]
mod tests {
    use crate::{Executor, InnerJoinHandle, JoinHandle, SendBound};
    use futures::future::AbortHandle;
    use std::future::Future;

    async fn task(tx: futures::channel::oneshot::Sender<()>) {
        futures_timer::Delay::new(std::time::Duration::from_secs(5)).await;
        let _ = tx.send(());
        unreachable!();
    }

    #[test]
    fn custom_abortable_task() {
        use futures::future::Abortable;
        struct FuturesExecutor {
            pool: futures::executor::ThreadPool,
        }

        impl Default for FuturesExecutor {
            fn default() -> Self {
                Self {
                    pool: futures::executor::ThreadPool::new().unwrap(),
                }
            }
        }

        impl Executor for FuturesExecutor {
            fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
            where
                F: Future + SendBound + 'static,
                F::Output: SendBound + 'static,
            {
                let (abort_handle, abort_registration) = AbortHandle::new_pair();
                let future = Abortable::new(future, abort_registration);
                let (tx, rx) = futures::channel::oneshot::channel();
                let fut = async {
                    let val = future.await;
                    let _ = tx.send(val);
                };

                self.pool.spawn_ok(fut);
                let inner = InnerJoinHandle::CustomHandle {
                    inner: Some(rx),
                    handle: abort_handle,
                };

                JoinHandle { inner }
            }
        }

        futures::executor::block_on(async move {
            let executor = FuturesExecutor::default();

            let (tx, rx) = futures::channel::oneshot::channel::<()>();
            let handle = executor.spawn_abortable(task(tx));
            drop(handle);
            let result = rx.await;
            assert!(result.is_err());
        });
    }
}
