use crate::communication::CommunicationHandle;
use crate::{AbortableJoinHandle, Executor};
use futures::channel::mpsc::Receiver;
use futures::{SinkExt, Stream, StreamExt};
use std::fmt::{Debug, Formatter};
use std::pin::Pin;

/// A task that accepts messages
pub struct CommunicationTask<In, Out = ()> {
    pub(crate) _task_handle: AbortableJoinHandle<()>,
    pub(crate) _channel_tx: futures::channel::mpsc::Sender<In>,
    pub(crate) _channel_rx: Pin<Box<async_channel::Receiver<Out>>>,
}

impl<In, Out> Clone for CommunicationTask<In, Out> {
    fn clone(&self) -> Self {
        CommunicationTask {
            _task_handle: self._task_handle.clone(),
            _channel_tx: self._channel_tx.clone(),
            _channel_rx: self._channel_rx.clone(),
        }
    }
}

impl<In, Out> Debug for CommunicationTask<In, Out> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CommunicationTask").finish()
    }
}

impl<In, Out> CommunicationTask<In, Out> {
    pub(crate) fn new<E, F, Fut>(executor: &E, buffer: usize, mut f: F) -> Self
    where
        E: Executor,
        F: FnMut(&CommunicationHandle<Out>, In) -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
        In: Send + 'static,
        Out: Send + 'static,
    {
        let (tx, mut rx) = futures::channel::mpsc::channel(buffer);
        let (channel_tx, channel_rx) = async_channel::bounded(buffer.max(1));
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

    pub(crate) fn new_with_receiver<E, F, Fut>(executor: &E, buffer: usize, mut f: F) -> Self
    where
        E: Executor,
        F: FnMut(CommunicationHandle<Out>, Receiver<In>) -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
        In: Send + 'static,
        Out: Send + 'static,
    {
        let (tx, rx) = futures::channel::mpsc::channel(buffer);
        let (channel_tx, channel_rx) = async_channel::bounded(buffer.max(1));

        let handle = CommunicationHandle::new(channel_tx);
        let fut = f(handle, rx);
        let _task_handle = executor.spawn_abortable(fut);

        Self {
            _task_handle,
            _channel_tx: tx,
            _channel_rx: Box::pin(channel_rx),
        }
    }

    pub(crate) fn new_with_context<E, F, C, Fut>(
        executor: &E,
        context: C,
        buffer: usize,
        mut f: F,
    ) -> Self
    where
        E: Executor,
        F: FnMut(&CommunicationHandle<Out>, &mut C, In) -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
        C: Send + 'static,
        In: Send + 'static,
        Out: Send + 'static,
    {
        let (tx, mut rx) = futures::channel::mpsc::channel(buffer);
        let (channel_tx, channel_rx) = async_channel::bounded(buffer.max(1));
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

    pub(crate) fn new_with_receiver_and_context<E, F, C, Fut>(
        executor: &E,
        context: C,
        buffer: usize,
        mut f: F,
    ) -> Self
    where
        E: Executor,
        F: FnMut(CommunicationHandle<Out>, C, Receiver<In>) -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
        In: Send + 'static,
        Out: Send + 'static,
    {
        let (tx, rx) = futures::channel::mpsc::channel(buffer);
        let (channel_tx, channel_rx) = async_channel::bounded(buffer.max(1));

        let handle = CommunicationHandle::new(channel_tx);
        let fut = f(handle, context, rx);
        let _task_handle = executor.spawn_abortable(fut);

        Self {
            _task_handle,
            _channel_tx: tx,
            _channel_rx: Box::pin(channel_rx),
        }
    }

    /// Send a message to the task
    pub async fn send(&mut self, data: In) -> std::io::Result<()> {
        self._channel_tx
            .send(data)
            .await
            .map_err(std::io::Error::other)
    }

    /// Attempts to send a message to the task, returning an error if the channel is full or closed due to the task being aborted.
    pub fn try_send(&mut self, data: In) -> std::io::Result<()> {
        self._channel_tx
            .try_send(data)
            .map_err(|e| std::io::Error::other(e.to_string()))
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

impl<In, Out> Stream for CommunicationTask<In, Out> {
    type Item = Out;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        self.get_mut()._channel_rx.poll_next_unpin(cx)
    }
}
