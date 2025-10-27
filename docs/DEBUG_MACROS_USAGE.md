# Debug Macros Usage

## Overview

The debug macros in `aios_core` provide runtime-controlled debugging output that can be enabled/disabled without recompiling the code.

## Available Macros

1. `debug_model_trace!` - Most detailed debugging information
2. `debug_model_debug!` - Debug level information  
3. `debug_model!` - General information (default level)
4. `debug_model_warn!` - Warning information

## Usage

### Importing

```rust
use aios_core::{
    debug_model_trace, debug_model_debug, debug_model, debug_model_warn,
    set_debug_model_enabled
};
```

### Controlling Debug Output

```rust
// Enable debug output
set_debug_model_enabled(true);

// Disable debug output  
set_debug_model_enabled(false);
```

### Example Usage

```rust
// Initialize debug state
set_debug_model_enabled(true);

// Use the macros
debug_model_debug!("🔧 rust-ploop-processor 处理完成，得到 {} 个顶点", 4);
debug_model_debug!("✅ Polyline 转换完成，包含 {} 个顶点", 4);
debug_model_debug!("🔧 使用 rust-ploop-processor 统一处理 {} 个顶点", 4);
debug_model_debug!("🔧 开始处理PLOOP顶点: POLYLINE_GENERATION");
debug_model_debug!("   输入顶点数: {}", 4);
debug_model_debug!("   处理后顶点数: {}", 4);
debug_model_debug!("   其中包含 {} 个FRADIUS顶点", 0);
debug_model_debug!("✅ PLOOP顶点处理完成，返回 {} 个顶点", 4);
debug_model_debug!("   rust-ploop-processor 处理完成，得到 {} 个顶点", 4);
```

## Replace Existing Print Statements

To replace existing `println!` statements with debug macros:

```rust
// Before
println!("🔧 rust-ploop-processor 处理完成，得到 {} 个顶点", vertex_count);

// After
debug_model_debug!("🔧 rust-ploop-processor 处理完成，得到 {} 个顶点", vertex_count);
```

## Benefits

1. **Runtime Control**: Debug output can be enabled/disabled without recompiling
2. **Performance**: When disabled, debug statements have minimal overhead
3. **Consistent Logging**: Uses `tracing` backend for consistent log formatting
4. **Level-based**: Different macros for different debug levels

## Migration from gen-model

The debug macros were originally defined in `gen-model`'s `fast_model/mod.rs` and have been moved to `aios_core` for better reusability across projects.

- Original location: `gen-model/src/fast_model/mod.rs`
- New location: `rs-core/src/debug_macros.rs` (exported from `aios_core`)

### Import Changes Required

If you were previously importing from the local module:

```rust
// Old way (in gen-model)
use crate::{debug_model_debug, debug_model_trace};

// New way (recommended)
use aios_core::{debug_model_debug, debug_model_trace};

// Or via re-export in fast_model
use crate::fast_model::{debug_model_debug, debug_model_trace};
```
