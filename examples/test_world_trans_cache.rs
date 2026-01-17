/// 测试世界变换矩阵缓存机制
///
/// 此示例验证：
/// 1. 首次调用 get_world_mat4 时计算并缓存到 pe_transform 表
/// 2. 第二次调用时从缓存读取（提高性能）
/// 3. 缓存失效机制正常工作
use aios_core::{init_surreal, pe_key, query_pe_transform};
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化数据库连接
    init_surreal().await?;

    // 使用一个测试 refno (请根据实际数据库修改)
    let test_refno = pe_key!("17496_172825");

    println!("========================================");
    println!("🧪 测试世界变换矩阵缓存机制");
    println!("========================================\n");

    // 第一步：清除现有缓存
    println!("📝 步骤 1: 清除现有缓存");
    aios_core::transform::invalidate_world_trans_cache(test_refno).await?;
    println!("✅ 缓存已清除\n");

    // 第二步：首次调用 - 应该计算并缓存
    println!("📝 步骤 2: 首次调用 get_world_mat4 (应该缓存未命中)");
    let start = std::time::Instant::now();
    let mat4_1 = aios_core::transform::get_world_mat4(test_refno, false).await?;
    let duration_1 = start.elapsed();
    println!("⏱️  首次调用耗时: {:?}", duration_1);
    println!(
        "📊 计算结果: {:?}\n",
        mat4_1.map(|m| m.to_scale_rotation_translation())
    );

    // 等待异步缓存写入完成
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // 第三步：验证缓存已写入数据库
    println!("📝 步骤 3: 验证缓存已写入数据库");
    if let Some(cache) = query_pe_transform(test_refno).await? {
        if let Some(world_trans) = &cache.world {
            println!("✅ 缓存已写入 pe_transform 表");
            println!("   Translation: {:?}", world_trans.translation);
            println!("   Rotation: {:?}", world_trans.rotation);
            println!("   Scale: {:?}\n", world_trans.scale);
        } else {
            println!("❌ 警告: 缓存未写入!\n");
        }
    }

    // 第四步：第二次调用 - 应该从缓存读取
    println!("📝 步骤 4: 第二次调用 get_world_mat4 (应该缓存命中)");
    let start = std::time::Instant::now();
    let mat4_2 = aios_core::transform::get_world_mat4(test_refno, false).await?;
    let duration_2 = start.elapsed();
    println!("⏱️  第二次调用耗时: {:?}", duration_2);
    println!(
        "📊 读取结果: {:?}\n",
        mat4_2.map(|m| m.to_scale_rotation_translation())
    );

    // 第五步：对比两次结果
    println!("📝 步骤 5: 对比两次结果");
    if let (Some(m1), Some(m2)) = (mat4_1, mat4_2) {
        let diff = (m1 - m2).abs();
        let max_diff = diff.to_cols_array().iter().fold(0.0_f64, |a, &b| a.max(b));
        println!("✅ 两次结果一致性检查:");
        println!("   最大差异: {:.10}", max_diff);
        if max_diff < 1e-6 {
            println!("   ✅ 结果完全一致!");
        } else {
            println!("   ⚠️  结果存在差异!");
        }
    }

    // 第六步：性能对比
    println!("\n📝 步骤 6: 性能对比");
    println!("   首次调用(计算): {:?}", duration_1);
    println!("   第二次调用(缓存): {:?}", duration_2);
    if duration_1 > duration_2 {
        let speedup = duration_1.as_secs_f64() / duration_2.as_secs_f64();
        println!("   🚀 性能提升: {:.2}x", speedup);
    }

    println!("\n========================================");
    println!("✅ 测试完成!");
    println!("========================================");

    Ok(())
}
