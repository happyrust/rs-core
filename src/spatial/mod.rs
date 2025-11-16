#[cfg(all(not(target_arch = "wasm32"), feature = "sqlite"))]
pub mod hybrid_index;
pub mod pipe;
#[cfg(all(not(target_arch = "wasm32"), feature = "sqlite"))]
pub mod sqlite;
