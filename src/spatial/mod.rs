pub mod pipe;
#[cfg(all(not(target_arch = "wasm32"), feature = "sqlite"))]
pub mod sqlite;
