#![cfg(all(feature = "macros", not(target_arch = "wasm32")))]

use async_rt::{Executor, JoinError};

#[cfg(any(
    feature = "tokio",
    feature = "smol",
    feature = "compio",
    feature = "threadpool"
))]
#[async_rt::main]
async fn default_main() -> Result<usize, JoinError> {
    async_rt::task::spawn(async { 42 }).await
}

#[cfg(any(
    feature = "tokio",
    feature = "smol",
    feature = "compio",
    feature = "threadpool"
))]
#[test]
fn main_uses_the_global_executor() {
    assert_eq!(default_main().unwrap(), 42);
}

#[cfg(any(
    feature = "tokio",
    feature = "smol",
    feature = "compio",
    feature = "threadpool"
))]
#[async_rt::test]
async fn test_uses_the_global_executor() {
    assert_eq!(async_rt::task::spawn(async { 42 }).await.unwrap(), 42);
}

#[cfg(any(
    feature = "tokio",
    feature = "smol",
    feature = "compio",
    feature = "threadpool"
))]
#[async_rt::test]
async fn test_preserves_result_return_types() -> Result<(), JoinError> {
    assert_eq!(async_rt::task::spawn(async { 42 }).await?, 42);
    Ok(())
}

#[cfg(any(
    feature = "tokio",
    feature = "smol",
    feature = "compio",
    feature = "threadpool"
))]
#[async_rt::test]
#[should_panic(expected = "expected panic")]
async fn test_preserves_harness_attributes() {
    panic!("expected panic");
}

#[cfg(feature = "tokio")]
#[async_rt::test(executor = tokio)]
async fn explicit_tokio_executor() {
    let handle = async_rt::rt::tokio::TokioExecutor.spawn(async { 42 });
    assert_eq!(handle.await.unwrap(), 42);
}

#[cfg(feature = "smol")]
#[async_rt::test(executor = "smol")]
async fn explicit_smol_executor() {
    let handle = async_rt::rt::smol::SmolExecutor.spawn(async { 42 });
    assert_eq!(handle.await.unwrap(), 42);
}

#[cfg(feature = "compio")]
#[async_rt::test(executor = "compio")]
async fn explicit_compio_executor() {
    let handle = async_rt::rt::compio::CompioExecutor.spawn(async { 42 });
    assert_eq!(handle.await.unwrap(), 42);
}

#[cfg(feature = "threadpool")]
#[async_rt::test(executor = "threadpool")]
async fn explicit_threadpool_executor() {
    let handle = async_rt::rt::threadpool::ThreadPoolExecutor.spawn(async { 42 });
    assert_eq!(handle.await.unwrap(), 42);
}
