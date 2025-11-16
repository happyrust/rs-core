pub mod data;

pub mod algorithm;

pub mod query;

// 改进版本的房间查询模块
#[cfg(all(not(target_arch = "wasm32"), feature = "sqlite"))]
pub mod query_v2;

// 房间系统监控模块
pub mod monitoring;

// 统一数据模型
pub mod data_model;

// 房间代码标准化处理
pub mod room_code_processor;

// 数据迁移和验证工具
pub mod migration_tools;

// 版本控制系统
pub mod version_control;

// 房间系统管理器
pub mod room_system_manager;
