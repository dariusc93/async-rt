use crate::{Executor, InnerJoinHandle, JoinHandle};
use std::future::Future;

/// Tokio executor
#[derive(Clone, Copy, Debug, PartialOrd, PartialEq, Eq)]
pub struct TokioExecutor;

impl Executor for TokioExecutor {
    fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        let handle = tokio::task::spawn(future);
        let inner = InnerJoinHandle::TokioHandle(handle);
        JoinHandle { inner }
    }
}

#[cfg(test)]
mod tests {
    use super::TokioExecutor;
    use crate::Executor;
    use futures::channel::mpsc::{Receiver, UnboundedReceiver};

    #[tokio::test]
    async fn default_abortable_task() {
        let executor = TokioExecutor;

        async fn task(tx: futures::channel::oneshot::Sender<()>) {
            futures_timer::Delay::new(std::time::Duration::from_secs(5)).await;
            let _ = tx.send(());
            unreachable!();
        }

        let (tx, rx) = futures::channel::oneshot::channel::<()>();

        let handle = executor.spawn_abortable(task(tx));

        drop(handle);
        let result = rx.await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn task_coroutine() {
        use futures::stream::StreamExt;
        let executor = TokioExecutor;

        enum Message {
            Send(String, futures::channel::oneshot::Sender<String>),
        }

        let mut task = executor.spawn_coroutine(|mut rx: Receiver<Message>| async move {
            while let Some(msg) = rx.next().await {
                match msg {
                    Message::Send(msg, sender) => {
                        sender.send(msg).unwrap();
                    }
                }
            }
        });

        let (tx, rx) = futures::channel::oneshot::channel::<String>();
        let msg = Message::Send("Hello".into(), tx);

        task.send(msg).await.unwrap();
        let resp = rx.await.unwrap();
        assert_eq!(resp, "Hello");
    }

    #[tokio::test]
    async fn task_coroutine_with_context() {
        use futures::stream::StreamExt;
        let executor = TokioExecutor;

        #[derive(Default)]
        struct State {
            message: String,
        }

        enum Message {
            Set(String),
            Get(futures::channel::oneshot::Sender<String>),
        }

        let mut task = executor.spawn_coroutine_with_context(
            State::default(),
            |mut state, mut rx: Receiver<Message>| async move {
                while let Some(msg) = rx.next().await {
                    match msg {
                        Message::Set(msg) => {
                            state.message = msg;
                        }
                        Message::Get(resp) => {
                            resp.send(state.message.clone()).unwrap();
                        }
                    }
                }
            },
        );

        let msg = Message::Set("Hello".into());

        task.send(msg).await.unwrap();
        let (tx, rx) = futures::channel::oneshot::channel::<String>();
        let msg = Message::Get(tx);
        task.send(msg).await.unwrap();
        let resp = rx.await.unwrap();
        assert_eq!(resp, "Hello");
    }

    #[tokio::test]
    async fn task_unbounded_coroutine() {
        use futures::stream::StreamExt;
        let executor = TokioExecutor;

        enum Message {
            Send(String, futures::channel::oneshot::Sender<String>),
        }

        let mut task =
            executor.spawn_unbounded_coroutine(|mut rx: UnboundedReceiver<Message>| async move {
                while let Some(msg) = rx.next().await {
                    match msg {
                        Message::Send(msg, sender) => {
                            sender.send(msg).unwrap();
                        }
                    }
                }
            });

        let (tx, rx) = futures::channel::oneshot::channel::<String>();
        let msg = Message::Send("Hello".into(), tx);

        task.send(msg).unwrap();
        let resp = rx.await.unwrap();
        assert_eq!(resp, "Hello");
    }

    #[tokio::test]
    async fn task_unbounded_coroutine_with_context() {
        use futures::stream::StreamExt;
        let executor = TokioExecutor;

        #[derive(Default)]
        struct State {
            message: String,
        }

        enum Message {
            Set(String),
            Get(futures::channel::oneshot::Sender<String>),
        }

        let mut task = executor.spawn_unbounded_coroutine_with_context(
            State::default(),
            |mut state, mut rx: UnboundedReceiver<Message>| async move {
                while let Some(msg) = rx.next().await {
                    match msg {
                        Message::Set(msg) => {
                            state.message = msg;
                        }
                        Message::Get(resp) => {
                            resp.send(state.message.clone()).unwrap();
                        }
                    }
                }
            },
        );

        let msg = Message::Set("Hello".into());

        task.send(msg).unwrap();
        let (tx, rx) = futures::channel::oneshot::channel::<String>();
        let msg = Message::Get(tx);
        task.send(msg).unwrap();
        let resp = rx.await.unwrap();
        assert_eq!(resp, "Hello");
    }
}
