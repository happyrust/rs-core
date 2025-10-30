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
            println!($($arg)*);
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

/// Set expression debug info in CataContext (only when debug_model is enabled)
///
/// Usage:
/// ```
/// set_expr_debug_info!(context, geo_refno, geo_type, attr_name);
/// set_expr_debug_info!(context, geo_refno, geo_type, attr_name, index);
/// ```
#[macro_export]
macro_rules! set_expr_debug_info {
    // 不带索引的版本
    ($context:expr, $geo_refno:expr, $geo_type:expr, $attr_name:expr) => {{
        if $crate::debug_macros::is_debug_model_enabled() {
            *$context.debug_geo_refno.borrow_mut() = Some($geo_refno.to_string());
            *$context.debug_geo_type.borrow_mut() = Some($geo_type.to_string());
            *$context.debug_attr_name.borrow_mut() = Some($attr_name.to_string());
            *$context.debug_attr_index.borrow_mut() = None;
        }
    }};

    // 带索引的版本
    ($context:expr, $geo_refno:expr, $geo_type:expr, $attr_name:expr, $index:expr) => {{
        if $crate::debug_macros::is_debug_model_enabled() {
            *$context.debug_geo_refno.borrow_mut() = Some($geo_refno.to_string());
            *$context.debug_geo_type.borrow_mut() = Some($geo_type.to_string());
            *$context.debug_attr_name.borrow_mut() = Some($attr_name.to_string());
            *$context.debug_attr_index.borrow_mut() = Some($index);
        }
    }};
}

/// Clear expression debug info in CataContext (only when debug_model is enabled)
#[macro_export]
macro_rules! clear_expr_debug_info {
    ($context:expr) => {{
        if $crate::debug_macros::is_debug_model_enabled() {
            *$context.debug_attr_name.borrow_mut() = None;
            *$context.debug_attr_index.borrow_mut() = None;
        }
    }};
}
