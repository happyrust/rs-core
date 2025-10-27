//! Debug macros for model generation and processing
//!
//! This module provides debug macros that can be controlled at runtime
//! to enable/disable debugging output without recompiling.

use once_cell::sync::Lazy;
use std::sync::atomic::{AtomicBool, Ordering};

/// Global debug flag (runtime controlled)
pub static DEBUG_MODEL_ENABLED: Lazy<AtomicBool> = Lazy::new(|| AtomicBool::new(false));

/// Check if debug model is enabled
#[inline]
pub fn is_debug_model_enabled() -> bool {
    DEBUG_MODEL_ENABLED.load(Ordering::Relaxed)
}

/// Set debug model enabled state
#[inline]
pub fn set_debug_model_enabled(enabled: bool) {
    DEBUG_MODEL_ENABLED.store(enabled, Ordering::Relaxed);
}

/// Debug model trace - most detailed debugging information
#[macro_export]
macro_rules! debug_model_trace {
    ($($arg:tt)*) => {{
        if $crate::debug_macros::is_debug_model_enabled() {
            tracing::trace!($($arg)*);
        }
    }};
}

/// Debug model debug - debug level information
#[macro_export]
macro_rules! debug_model_debug {
    ($($arg:tt)*) => {{
        if $crate::debug_macros::is_debug_model_enabled() {
            tracing::debug!($($arg)*);
        }
    }};
}

/// Debug model info - general information (default level)
#[macro_export]
macro_rules! debug_model {
    ($($arg:tt)*) => {{
        if $crate::debug_macros::is_debug_model_enabled() {
            tracing::info!($($arg)*);
        }
    }};
}

/// Debug model warn - warning information
#[macro_export]
macro_rules! debug_model_warn {
    ($($arg:tt)*) => {{
        if $crate::debug_macros::is_debug_model_enabled() {
            tracing::warn!($($arg)*);
        }
    }};
}
