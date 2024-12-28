use std::fmt::{Debug, Formatter};
use std::future::Future;
use std::sync::{LazyLock};
use futures::executor::ThreadPool;
use futures::future::{AbortHandle, Abortable};
use crate::{Executor, InnerJoinHandle, JoinHandle};

const THREADPOOL_EXECUTOR: LazyLock<ThreadPool> = LazyLock::new(|| {
    ThreadPool::new().unwrap()
});

#[derive(Clone, Copy, Default)]
pub struct ThreadPoolExecutor;

impl Debug for ThreadPoolExecutor {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ThreadPoolExecutor").finish()
    }
}

impl Executor for ThreadPoolExecutor {
    fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        let (abort_handle, abort_registration) = AbortHandle::new_pair();
        let future = Abortable::new(future, abort_registration);
        let (tx, rx) = futures::channel::oneshot::channel();
        let fut = async {
            let val = future.await;
            let _ = tx.send(val);
        };

        THREADPOOL_EXECUTOR.spawn_ok(fut);
        let inner = InnerJoinHandle::CustomHandle {
            inner: Some(rx),
            handle: abort_handle,
        };

        JoinHandle { inner }
    }
}

#[cfg(test)]
mod tests {
    use futures::channel::mpsc::Receiver;
    use crate::Executor;
    use super::ThreadPoolExecutor;

    async fn task(tx: futures::channel::oneshot::Sender<()>) {
        futures_timer::Delay::new(std::time::Duration::from_secs(5)).await;
        let _ = tx.send(());
        unreachable!();
    }

    #[test]
    fn default_abortable_task() {
        let executor = ThreadPoolExecutor::default();

        let (tx, rx) = futures::channel::oneshot::channel::<()>();
        let handle = executor.spawn_abortable(task(tx));
        drop(handle);

        futures::executor::block_on(async move {
            let result = rx.await;
            assert!(result.is_err());
        });
    }

    #[test]
    fn task_coroutine() {
        use futures::stream::StreamExt;
        let executor = ThreadPoolExecutor::default();

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
        futures::executor::block_on(async move {
            task.send(msg).await.unwrap();
            let resp = rx.await.unwrap();
            assert_eq!(resp, "Hello");
        });
    }
}
