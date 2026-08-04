use crate::communication::CommunicationHandle;
use crate::{AbortableJoinHandle, Executor};
use futures::channel::mpsc::UnboundedReceiver;
use futures::{Stream, StreamExt};
use std::fmt::{Debug, Formatter};
use std::pin::Pin;

/// A task that accepts messages
pub struct UnboundedCommunicationTask<In, Out = ()> {
    pub(crate) _task_handle: AbortableJoinHandle<()>,
    pub(crate) _channel_tx: futures::channel::mpsc::UnboundedSender<In>,
    pub(crate) _channel_rx: Pin<Box<async_channel::Receiver<Out>>>,
}

impl<In, Out> Clone for UnboundedCommunicationTask<In, Out> {
    fn clone(&self) -> Self {
        UnboundedCommunicationTask {
            _task_handle: self._task_handle.clone(),
            _channel_tx: self._channel_tx.clone(),
            _channel_rx: self._channel_rx.clone(),
        }
    }
}

impl<T> Debug for UnboundedCommunicationTask<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UnboundedCommunicationTask").finish()
    }
}

impl<In, Out> UnboundedCommunicationTask<In, Out> {
    pub(crate) fn new<E, F, Fut>(executor: &E, mut f: F) -> Self
    where
        E: Executor,
        F: FnMut(&CommunicationHandle<Out>, In) -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
        In: Send + 'static,
        Out: Send + 'static,
    {
        let (tx, mut rx) = futures::channel::mpsc::unbounded();
        let (channel_tx, channel_rx) = async_channel::unbounded();
        let _task_handle = executor.spawn_abortable(async move {
            let handle = CommunicationHandle::new(channel_tx);
            while let Some(msg) = rx.next().await {
                f(&handle, msg).await;
            }
        });
        Self {
            _task_handle,
            _channel_tx: tx,
            _channel_rx: Box::pin(channel_rx),
        }
    }

    pub(crate) fn new_with_context<E, F, C, Fut>(executor: &E, context: C, mut f: F) -> Self
    where
        E: Executor,
        F: FnMut(&CommunicationHandle<Out>, &mut C, In) -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
        C: Send + 'static,
        In: Send + 'static,
        Out: Send + 'static,
    {
        let (tx, mut rx) = futures::channel::mpsc::unbounded();
        let (channel_tx, channel_rx) = async_channel::unbounded();
        let _task_handle = executor.spawn_abortable(async move {
            let handle = CommunicationHandle::new(channel_tx);
            let mut context = context;
            while let Some(msg) = rx.next().await {
                f(&handle, &mut context, msg).await;
            }
        });
        Self {
            _task_handle,
            _channel_tx: tx,
            _channel_rx: Box::pin(channel_rx),
        }
    }

    pub(crate) fn new_with_receiver<E, F, Fut>(executor: &E, mut f: F) -> Self
    where
        E: Executor,
        F: FnMut(CommunicationHandle<Out>, UnboundedReceiver<In>) -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
        In: Send + 'static,
        Out: Send + 'static,
    {
        let (tx, rx) = futures::channel::mpsc::unbounded();
        let (channel_tx, channel_rx) = async_channel::unbounded();

        let handle = CommunicationHandle::new(channel_tx);
        let fut = f(handle, rx);
        let _task_handle = executor.spawn_abortable(fut);

        Self {
            _task_handle,
            _channel_tx: tx,
            _channel_rx: Box::pin(channel_rx),
        }
    }

    pub(crate) fn new_with_receiver_and_context<E, F, C, Fut>(
        executor: &E,
        context: C,
        mut f: F,
    ) -> Self
    where
        E: Executor,
        F: FnMut(CommunicationHandle<Out>, C, UnboundedReceiver<In>) -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
        In: Send + 'static,
        Out: Send + 'static,
    {
        let (tx, rx) = futures::channel::mpsc::unbounded();
        let (channel_tx, channel_rx) = async_channel::unbounded();

        let handle = CommunicationHandle::new(channel_tx);
        let fut = f(handle, context, rx);
        let _task_handle = executor.spawn_abortable(fut);

        Self {
            _task_handle,
            _channel_tx: tx,
            _channel_rx: Box::pin(channel_rx),
        }
    }

    // pub(crate) fn new(
    //     task_handle: AbortableJoinHandle<()>,
    //     channel_tx: futures::channel::mpsc::UnboundedSender<T>,
    // ) -> Self {
    //     Self {
    //         _task_handle: task_handle,
    //         _channel_tx: channel_tx,
    //     }
    // }

    /// Send a message to task
    pub fn send(&mut self, data: In) -> std::io::Result<()> {
        self._channel_tx
            .unbounded_send(data)
            .map_err(|e| std::io::Error::other(e.to_string()))
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

impl<In, Out> Stream for UnboundedCommunicationTask<In, Out> {
    type Item = Out;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        self.get_mut()._channel_rx.poll_next_unpin(cx)
    }
}
