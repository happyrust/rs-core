use aios_core::RefnoEnum;
use aios_core::room::{
    data_model::{RoomCode, RoomRelationType},
    migration_tools::{MigrationTool, ValidationTool},
    room_code_processor::{batch_process_room_codes, process_room_code},
    room_system_manager::{ManagerConfig, RoomSystemManager, initialize_room_system},
    version_control::{create_relation_snapshot, get_global_version_control},
};
use tracing::{Level, info, warn};
use tracing_subscriber;

/// 房间计算系统阶段二演示程序
///
/// 展示数据模型重构与一致性保障的功能：
/// 1. 统一关系模型设计
/// 2. 房间代码标准化处理
/// 3. 数据迁移和验证工具
/// 4. 关系数据的版本控制

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    info!("🚀 房间计算系统阶段二演示程序启动");

    // 1. 初始化房间系统管理器
    info!("📊 初始化房间系统管理器...");
    demo_system_initialization().await?;

    // 2. 演示房间代码标准化处理
    info!("🔧 演示房间代码标准化处理...");
    demo_room_code_processing().await?;

    // 3. 演示统一数据模型
    info!("📋 演示统一数据模型...");
    demo_unified_data_model().await?;

    // 4. 演示数据迁移工具
    info!("🔄 演示数据迁移工具...");
    demo_data_migration().await?;

    // 5. 演示版本控制功能
    info!("📚 演示版本控制功能...");
    demo_version_control().await?;

    // 6. 演示数据验证工具
    info!("✅ 演示数据验证工具...");
    demo_data_validation().await?;

    // 7. 演示系统管理功能
    info!("⚙️ 演示系统管理功能...");
    demo_system_management().await?;

    info!("✅ 房间计算系统阶段二演示完成");
    Ok(())
}

/// 演示系统初始化
async fn demo_system_initialization() -> anyhow::Result<()> {
    let config = ManagerConfig {
        auto_snapshot_enabled: true,
        snapshot_interval_hours: 1, // 演示用短间隔
        change_retention_days: 7,
        validation_enabled: true,
        batch_size: 100,
    };

    let mut manager = RoomSystemManager::new(Some(config));
    let init_result = manager.initialize().await?;

    info!("系统初始化结果:");
    info!("  成功: {}", init_result.success);
    info!("  消息: {}", init_result.message);
    info!("  操作ID: {}", init_result.operation_id);
    info!("  详情: {:?}", init_result.details);

    Ok(())
}

/// 演示房间代码标准化处理
async fn demo_room_code_processing() -> anyhow::Result<()> {
    let test_codes = vec![
        "SSC-A001".to_string(),
        "SSC-A1001".to_string(), // 需要转换的5位格式
        "HD-B123".to_string(),
        "HH-ROOM001".to_string(),
        "ssc-a002".to_string(), // 需要预处理
        "SSC_A003".to_string(), // 需要预处理
        "INVALID".to_string(),  // 无效格式
    ];

    info!("🔍 单个房间代码处理:");
    for code in &test_codes[..3] {
        let result = process_room_code(code).await;
        info!("  输入: {} -> 状态: {:?}", code, result.status);
        if let Some(standardized) = result.standardized_code {
            info!("    标准化: {}", standardized.full_code);
            info!(
                "    项目: {}, 区域: {}, 房间号: {}",
                standardized.project_prefix, standardized.area_code, standardized.room_number
            );
        }
        if !result.validation.errors.is_empty() {
            info!("    错误: {:?}", result.validation.errors);
        }
        if !result.validation.warnings.is_empty() {
            info!("    警告: {:?}", result.validation.warnings);
        }
    }

    info!("📦 批量房间代码处理:");
    let batch_results = batch_process_room_codes(test_codes).await;
    let success_count = batch_results
        .iter()
        .filter(|r| {
            matches!(
                r.status,
                aios_core::room::room_code_processor::ProcessingStatus::Success
            )
        })
        .count();
    let warning_count = batch_results
        .iter()
        .filter(|r| {
            matches!(
                r.status,
                aios_core::room::room_code_processor::ProcessingStatus::Warning
            )
        })
        .count();
    let error_count = batch_results
        .iter()
        .filter(|r| {
            matches!(
                r.status,
                aios_core::room::room_code_processor::ProcessingStatus::Error
            )
        })
        .count();

    info!(
        "  批量处理结果: 成功={}, 警告={}, 错误={}",
        success_count, warning_count, error_count
    );

    Ok(())
}

/// 演示统一数据模型
async fn demo_unified_data_model() -> anyhow::Result<()> {
    use aios_core::room::data_model::RoomRelation;

    // 创建标准化房间代码
    let room_codes = vec![
        RoomCode::build("SSC", "A", "001"),
        RoomCode::build("HD", "B", "102"),
        RoomCode::build("HH", "C", "203"),
    ];

    info!("🏗️ 创建房间关系:");
    let mut relations = Vec::new();

    for (i, room_code) in room_codes.iter().enumerate() {
        let relation = RoomRelation::new(
            if i % 2 == 0 {
                RoomRelationType::RoomContains
            } else {
                RoomRelationType::RoomPanel
            },
            RefnoEnum::Refno(10000 + i as u64),
            RefnoEnum::Refno(20000 + i as u64),
            room_code.clone(),
            0.85 + (i as f64 * 0.05),
        );

        info!(
            "  关系 {}: {} -> {}",
            i + 1,
            relation.from_refno,
            relation.to_refno
        );
        info!("    房间代码: {}", relation.room_code.full_code);
        info!("    关系类型: {:?}", relation.relation_type);
        info!("    置信度: {:.2}", relation.confidence);
        info!("    创建时间: {}", relation.created_at);

        // 验证关系
        match relation.validate() {
            Ok(_) => info!("    验证: ✅ 通过"),
            Err(e) => warn!("    验证: ❌ 失败 - {}", e),
        }

        relations.push(relation);
    }

    info!("📊 关系统计:");
    info!("  总关系数: {}", relations.len());
    let avg_confidence =
        relations.iter().map(|r| r.confidence).sum::<f64>() / relations.len() as f64;
    info!("  平均置信度: {:.3}", avg_confidence);

    Ok(())
}

/// 演示数据迁移工具
async fn demo_data_migration() -> anyhow::Result<()> {
    info!("🔄 数据迁移演示:");

    let mut migration_tool = MigrationTool::new();

    // 注意：这里只是演示迁移工具的接口，实际迁移需要真实的数据库连接
    info!("  迁移工具已创建");
    info!("  支持的迁移类型:");
    info!("    - room_relate 表迁移");
    info!("    - room_panel_relate 表迁移");
    info!("    - 房间代码标准化");
    info!("    - 数据一致性验证");

    // 模拟迁移统计
    info!("  模拟迁移结果:");
    info!("    总记录数: 1500");
    info!("    成功迁移: 1450");
    info!("    失败记录: 30");
    info!("    跳过记录: 20");
    info!("    处理时间: 2.5 秒");

    Ok(())
}

/// 演示版本控制功能
async fn demo_version_control() -> anyhow::Result<()> {
    use aios_core::room::data_model::RoomRelation;

    info!("📚 版本控制演示:");

    // 创建测试关系数据
    let room_code = RoomCode::build("SSC", "A", "001");
    let relations = vec![
        RoomRelation::new(
            RoomRelationType::RoomContains,
            RefnoEnum::Refno(12345),
            RefnoEnum::Refno(67890),
            room_code.clone(),
            0.95,
        ),
        RoomRelation::new(
            RoomRelationType::RoomPanel,
            RefnoEnum::Refno(11111),
            RefnoEnum::Refno(22222),
            room_code,
            0.90,
        ),
    ];

    // 创建快照
    let snapshot_id =
        create_relation_snapshot("演示快照 - 初始数据".to_string(), relations.clone()).await?;

    info!("  ✅ 创建快照: {}", snapshot_id);
    info!("  📊 快照统计:");
    info!("    关系数量: {}", relations.len());
    info!(
        "    数据大小: ~{} bytes",
        serde_json::to_vec(&relations)?.len()
    );

    // 获取版本控制实例并展示功能
    let vc = get_global_version_control().await;
    let vc = vc.lock().await;

    let snapshots = vc.list_snapshots();
    info!("  📋 快照列表:");
    for snapshot in snapshots {
        info!(
            "    ID: {}, 版本: {}, 时间: {}",
            snapshot.snapshot_id,
            snapshot.version,
            snapshot.created_at.format("%Y-%m-%d %H:%M:%S")
        );
        info!("      描述: {}", snapshot.description);
        info!("      关系数: {}", snapshot.stats.total_relations);
    }

    Ok(())
}

/// 演示数据验证工具
async fn demo_data_validation() -> anyhow::Result<()> {
    info!("✅ 数据验证演示:");

    // 注意：这里只是演示验证工具的接口，实际验证需要真实的数据库连接
    info!("  验证工具功能:");
    info!("    - 数据一致性检查");
    info!("    - 房间代码格式验证");
    info!("    - 引用完整性验证");
    info!("    - 重复关系检测");
    info!("    - 空间一致性验证");

    // 模拟验证结果
    info!("  模拟验证结果:");
    info!("    ✅ 数据一致性: 通过");
    info!("    ✅ 房间代码格式: 通过 (98.5%)");
    info!("    ⚠️  引用完整性: 警告 (5个孤立引用)");
    info!("    ✅ 重复关系: 通过");
    info!("    ✅ 空间一致性: 通过 (95.2%)");

    Ok(())
}

/// 演示系统管理功能
async fn demo_system_management() -> anyhow::Result<()> {
    info!("⚙️ 系统管理演示:");

    let mut manager = RoomSystemManager::new(None);

    // 演示创建房间关系
    info!("  🏗️ 创建房间关系:");
    let create_result = manager
        .create_room_relation(
            RoomRelationType::RoomContains,
            RefnoEnum::Refno(99999),
            RefnoEnum::Refno(88888),
            "SSC-A999",
            0.92,
        )
        .await?;

    info!("    操作结果: {}", create_result.success);
    info!("    消息: {}", create_result.message);
    info!("    操作ID: {}", create_result.operation_id);

    // 演示系统指标
    info!("  📊 系统指标:");
    let metrics = manager.get_system_metrics().await;
    info!("    内存使用: {:.2} MB", metrics.system.memory_usage_mb);
    info!("    总查询数: {}", metrics.query.total_queries);
    info!(
        "    平均查询时间: {:.2} ms",
        metrics.query.avg_query_time_ms
    );
    info!(
        "    缓存命中率: {:.2}%",
        metrics.cache.geometry_cache_hit_rate * 100.0
    );
    info!("    运行时间: {} 秒", metrics.uptime_seconds);

    // 演示快照创建
    info!("  📸 创建手动快照:");
    let snapshot_result = manager
        .create_manual_snapshot("演示程序手动快照".to_string())
        .await?;

    info!("    快照创建: {}", snapshot_result.success);
    info!(
        "    快照ID: {:?}",
        snapshot_result.details.get("snapshot_id")
    );

    // 演示系统清理
    info!("  🧹 系统清理:");
    let cleanup_result = manager.cleanup_system().await?;
    info!("    清理结果: {}", cleanup_result.success);
    info!("    清理消息: {}", cleanup_result.message);

    Ok(())
}

/// 性能基准测试
#[allow(dead_code)]
async fn benchmark_phase2_features() -> anyhow::Result<()> {
    use std::time::Instant;

    info!("⚡ 阶段二功能性能基准测试:");

    // 房间代码处理性能测试
    let test_codes: Vec<String> = (0..1000)
        .map(|i| format!("SSC-A{:03}", i % 999 + 1))
        .collect();

    let start_time = Instant::now();
    let results = batch_process_room_codes(test_codes.clone()).await;
    let processing_time = start_time.elapsed();

    let success_count = results
        .iter()
        .filter(|r| {
            matches!(
                r.status,
                aios_core::room::room_code_processor::ProcessingStatus::Success
            )
        })
        .count();

    info!("  房间代码处理基准:");
    info!("    处理数量: {}", test_codes.len());
    info!("    成功数量: {}", success_count);
    info!("    总耗时: {:?}", processing_time);
    info!(
        "    吞吐量: {:.2} 代码/秒",
        test_codes.len() as f64 / processing_time.as_secs_f64()
    );
    info!(
        "    平均处理时间: {:.2} ms",
        processing_time.as_millis() as f64 / test_codes.len() as f64
    );

    Ok(())
}
