//! 兼容层：为历史依赖保留 `aios_core::parse::*` 路径。
//!
//! 说明：部分下游 crate（例如 parse_pdms_db）仍会使用 `use aios_core::parse::*;`。
//! 近期重构后解析相关类型已拆分到其他模块，本文件仅做 re-export，避免破坏依赖方编译。

pub use crate::attlib_parser::*;
pub use crate::parsed_data::*;

