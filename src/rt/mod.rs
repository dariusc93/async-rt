#[cfg(not(target_arch = "wasm32"))]
#[cfg(feature = "tokio")]
pub mod tokio;
#[cfg(target_arch = "wasm32")]
pub mod wasm;
