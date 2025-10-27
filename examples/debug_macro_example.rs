//! Debug macro usage example
//!
//! This example demonstrates how to use the debug macros from aios_core

use aios_core::{debug_model_debug, set_debug_model_enabled};

fn main() {
    // Enable debug output at runtime
    set_debug_model_enabled(true);

    // Use the debug macros
    debug_model_debug!("🔧 rust-ploop-processor 处理完成，得到 {} 个顶点", 4);
    debug_model_debug!("✅ Polyline 转换完成，包含 {} 个顶点", 4);
    debug_model_debug!("🔧 使用 rust-ploop-processor 统一处理 {} 个顶点", 4);
    debug_model_debug!("🔧 开始处理PLOOP顶点: POLYLINE_GENERATION");
    debug_model_debug!("   输入顶点数: {}", 4);
    debug_model_debug!("   处理后顶点数: {}", 4);
    debug_model_debug!("   其中包含 {} 个FRADIUS顶点", 0);
    debug_model_debug!("✅ PLOOP顶点处理完成，返回 {} 个顶点", 4);
    debug_model_debug!("   rust-ploop-processor 处理完成，得到 {} 个顶点", 4);

    // Disable debug output
    set_debug_model_enabled(false);

    // These won't print anything now
    debug_model_debug!("This won't be printed (debug disabled)");
}
