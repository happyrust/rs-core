//! Kuzu 模型保存示例
//!
//! 演示如何将解析的 PDMS 数据保存到 Kuzu 图数据库
//!
//! 运行方式: cargo run --example kuzu_save_demo --features kuzu

use aios_core::rs_kuzu::*;
use aios_core::rs_kuzu::operations::*;
use aios_core::types::*;
use glam::Vec3;
use kuzu::SystemConfig;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    env_logger::init();

    println!("=== Kuzu 模型保存示例 ===\n");

    // 1. 初始化 Kuzu 数据库
    println!("1. 初始化 Kuzu 数据库...");
    let db_path = "./test_output/kuzu_demo.db";

    // 删除旧数据库
    let _ = std::fs::remove_dir_all(db_path);
    std::fs::create_dir_all(db_path)?;

    init_kuzu(db_path, SystemConfig::default()).await?;
    println!("   ✅ 数据库初始化成功");

    // 2. 初始化图模式
    println!("\n2. 初始化图模式...");
    init_kuzu_schema().await?;
    println!("   ✅ 模式初始化成功");

    // 3. 创建示例模型数据
    println!("\n3. 创建示例模型数据...");

    // 创建一个 ELBO (弯头)
    let elbo_pe = SPdmsElement {
        refno: RefnoEnum::Refno(RefU64(100001)),
        name: "ELBO-001".to_string(),
        noun: "ELBO".to_string(),
        dbnum: 1,
        sesno: 1,
        owner: RefnoEnum::Refno(RefU64(0)),
        deleted: false,
        lock: false,
        ..Default::default()
    };

    let mut elbo_attr = NamedAttrMap::default();
    elbo_attr.insert("TYPE".to_string(), NamedAttrValue::StringType("ELBO".to_string()));
    elbo_attr.insert("REFNO".to_string(), NamedAttrValue::RefU64Type(RefU64(100001)));
    elbo_attr.insert("NAME".to_string(), NamedAttrValue::StringType("ELBO-001".to_string()));
    elbo_attr.insert("BORE".to_string(), NamedAttrValue::F32Type(100.0)); // 管径
    elbo_attr.insert("ANGLE".to_string(), NamedAttrValue::F32Type(90.0)); // 角度
    elbo_attr.insert("RADIUS".to_string(), NamedAttrValue::F32Type(150.0)); // 弯曲半径
    elbo_attr.insert("POS".to_string(), NamedAttrValue::Vec3Type(Vec3::new(1000.0, 2000.0, 3000.0))); // 位置

    println!("   创建 ELBO: {} (refno={})", elbo_pe.name, elbo_pe.refno.refno());

    // 创建一个 PIPE (管道)
    let pipe_pe = SPdmsElement {
        refno: RefnoEnum::Refno(RefU64(100002)),
        name: "PIPE-001".to_string(),
        noun: "PIPE".to_string(),
        dbnum: 1,
        sesno: 1,
        owner: RefnoEnum::Refno(RefU64(0)),
        deleted: false,
        lock: false,
        ..Default::default()
    };

    let mut pipe_attr = NamedAttrMap::default();
    pipe_attr.insert("TYPE".to_string(), NamedAttrValue::StringType("PIPE".to_string()));
    pipe_attr.insert("REFNO".to_string(), NamedAttrValue::RefU64Type(RefU64(100002)));
    pipe_attr.insert("NAME".to_string(), NamedAttrValue::StringType("PIPE-001".to_string()));
    pipe_attr.insert("BORE".to_string(), NamedAttrValue::F32Type(50.0)); // 管径
    pipe_attr.insert("LENGTH".to_string(), NamedAttrValue::F32Type(5000.0)); // 长度
    pipe_attr.insert("POSS".to_string(), NamedAttrValue::Vec3Type(Vec3::new(0.0, 0.0, 0.0))); // 起点
    pipe_attr.insert("POSE".to_string(), NamedAttrValue::Vec3Type(Vec3::new(5000.0, 0.0, 0.0))); // 终点

    println!("   创建 PIPE: {} (refno={})", pipe_pe.name, pipe_pe.refno.refno());

    // 4. 保存单个模型
    println!("\n4. 保存单个模型...");
    save_model_to_kuzu(&elbo_pe, &elbo_attr).await?;
    println!("   ✅ 成功保存 ELBO-001");

    save_model_to_kuzu(&pipe_pe, &pipe_attr).await?;
    println!("   ✅ 成功保存 PIPE-001");

    // 5. 批量保存模型
    println!("\n5. 批量保存模型...");
    let mut batch_models = Vec::new();

    for i in 0..5 {
        let refno = 200000 + i;
        let pe = SPdmsElement {
            refno: RefnoEnum::Refno(RefU64(refno)),
            name: format!("BATCH-ELBO-{:03}", i),
            noun: "ELBO".to_string(),
            dbnum: 1,
            sesno: 1,
            owner: RefnoEnum::Refno(RefU64(0)),
            deleted: false,
            lock: false,
            ..Default::default()
        };

        let mut attmap = NamedAttrMap::default();
        attmap.insert("TYPE".to_string(), NamedAttrValue::StringType("ELBO".to_string()));
        attmap.insert("REFNO".to_string(), NamedAttrValue::RefU64Type(RefU64(refno)));
        attmap.insert("NAME".to_string(), NamedAttrValue::StringType(format!("BATCH-ELBO-{:03}", i)));
        attmap.insert("BORE".to_string(), NamedAttrValue::F32Type(100.0 + i as f32 * 10.0));
        attmap.insert("ANGLE".to_string(), NamedAttrValue::F32Type(45.0 + i as f32 * 15.0));

        batch_models.push((pe, attmap));
    }

    save_models_batch(batch_models).await?;
    println!("   ✅ 成功批量保存 5 个 ELBO 模型");

    // 6. 保存带引用关系的模型
    println!("\n6. 保存带引用关系的模型...");

    // 先创建一个被引用的 PE
    let ref_target_pe = SPdmsElement {
        refno: RefnoEnum::Refno(RefU64(300001)),
        name: "TARGET-ELBO".to_string(),
        noun: "ELBO".to_string(),
        dbnum: 1,
        sesno: 1,
        owner: RefnoEnum::Refno(RefU64(0)),
        deleted: false,
        lock: false,
        ..Default::default()
    };

    let mut ref_target_attr = NamedAttrMap::default();
    ref_target_attr.insert("TYPE".to_string(), NamedAttrValue::StringType("ELBO".to_string()));
    ref_target_attr.insert("REFNO".to_string(), NamedAttrValue::RefU64Type(RefU64(300001)));
    ref_target_attr.insert("NAME".to_string(), NamedAttrValue::StringType("TARGET-ELBO".to_string()));
    ref_target_attr.insert("BORE".to_string(), NamedAttrValue::F32Type(80.0));

    save_model_to_kuzu(&ref_target_pe, &ref_target_attr).await?;
    println!("   保存目标 PE: TARGET-ELBO");

    // 创建引用它的 PE
    let ref_source_pe = SPdmsElement {
        refno: RefnoEnum::Refno(RefU64(300002)),
        name: "SOURCE-PIPE".to_string(),
        noun: "PIPE".to_string(),
        dbnum: 1,
        sesno: 1,
        owner: RefnoEnum::Refno(RefU64(0)),
        deleted: false,
        lock: false,
        ..Default::default()
    };

    let mut ref_source_attr = NamedAttrMap::default();
    ref_source_attr.insert("TYPE".to_string(), NamedAttrValue::StringType("PIPE".to_string()));
    ref_source_attr.insert("REFNO".to_string(), NamedAttrValue::RefU64Type(RefU64(300002)));
    ref_source_attr.insert("NAME".to_string(), NamedAttrValue::StringType("SOURCE-PIPE".to_string()));
    ref_source_attr.insert("BORE".to_string(), NamedAttrValue::F32Type(80.0));
    // 添加引用
    ref_source_attr.insert("PREF".to_string(), NamedAttrValue::RefU64Type(RefU64(300001)));

    save_model_to_kuzu(&ref_source_pe, &ref_source_attr).await?;
    println!("   ✅ 成功保存带引用关系的模型: SOURCE-PIPE -> TARGET-ELBO");

    // 7. 统计信息
    println!("\n=== 保存完成 ===");
    println!("总计保存:");
    println!("  - 单个模型: 2 个 (ELBO-001, PIPE-001)");
    println!("  - 批量模型: 5 个 (BATCH-ELBO-000 ~ 004)");
    println!("  - 引用模型: 2 个 (SOURCE-PIPE, TARGET-ELBO)");
    println!("  - 合计: 9 个模型");
    println!("\n数据库路径: {}", db_path);
    println!("\n✅ 所有操作成功完成!");

    Ok(())
}
