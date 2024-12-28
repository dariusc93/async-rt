#[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
pub mod tokio;

#[cfg(target_arch = "wasm32")]
pub mod wasm;
