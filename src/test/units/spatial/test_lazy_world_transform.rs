//! 测试惰性计算 world_transform
//!
//! 验证当 pe_transform 缓存不存在时，能够自动计算并写入缓存

use aios_core::{RefnoEnum, SUL_DB, SurrealQueryExt};
use aios_core::rs_surreal::pe_transform::query_pe_transform;
use aios_core::transform::get_world_mat4;

/// 测试惰性计算 world_transform
/// 
/// 流程：
/// 1. 删除 pe_transform:17496_142306
/// 2. 调用 get_world_mat4 触发惰性计算
/// 3. 验证数据库已写入缓存
#[tokio::test]
async fn test_lazy_world_transform_17496_142306() -> anyhow::Result<()> {
    // 初始化数据库
    aios_core::init_surreal_db().await?;
    
    let refno = RefnoEnum::from_str("17496/142306")?;
    let pe_key = refno.to_table_key("pe_transform");
    
    println!("=== 测试惰性计算 world_transform ===");
    println!("目标: {}", refno);
    
    // Step 1: 删除缓存
    println!("\n[Step 1] 删除 pe_transform 缓存...");
    let delete_sql = format!("DELETE {}", pe_key);
    SUL_DB.query_response(&delete_sql).await?;
    println!("  ✅ 已删除 {}", pe_key);
    
    // 验证已删除
    let cache_before = query_pe_transform(refno).await?;
    assert!(cache_before.is_none() || cache_before.as_ref().map(|c| c.world.is_none()).unwrap_or(true),
        "缓存应该不存在或 world 为 None");
    println!("  ✅ 确认缓存已清空");
    
    // Step 2: 调用 get_world_mat4 触发惰性计算
    println!("\n[Step 2] 调用 get_world_mat4 触发惰性计算...");
    let world_mat = get_world_mat4(refno, false).await?;
    
    match &world_mat {
        Some(mat) => {
            println!("  ✅ 惰性计算成功!");
            println!("  Translation: {:?}", mat.w_axis.truncate());
        }
        None => {
            println!("  ❌ 惰性计算返回 None");
            return Err(anyhow::anyhow!("惰性计算失败，返回 None"));
        }
    }
    
    // Step 3: 等待异步写入完成
    println!("\n[Step 3] 等待缓存写入...");
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    
    // Step 4: 验证数据库已更新
    println!("\n[Step 4] 验证数据库缓存...");
    let cache_after = query_pe_transform(refno).await?;
    
    match cache_after {
        Some(cache) => {
            if let Some(world) = cache.world {
                println!("  ✅ 数据库缓存已写入!");
                println!("  world_trans.translation: {:?}", world.translation);
                println!("  world_trans.rotation: {:?}", world.rotation);
                println!("  world_trans.scale: {:?}", world.scale);
            } else {
                println!("  ❌ world_trans 仍为 None");
                return Err(anyhow::anyhow!("缓存写入失败: world_trans 为 None"));
            }
        }
        None => {
            println!("  ❌ pe_transform 记录不存在");
            return Err(anyhow::anyhow!("缓存写入失败: 记录不存在"));
        }
    }
    
    println!("\n=== 测试通过 ===");
    Ok(())
}
