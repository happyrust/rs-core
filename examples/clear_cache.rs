use aios_core::*;
use anyhow::Result;

/// 清除特定refno的变换矩阵缓存
#[tokio::main]
async fn main() -> Result<()> {
    // 初始化数据库连接
    init_surreal().await?;

    let poinsp_refno = RefnoEnum::from("17496_266220");

    println!("🧹 清除POINSP {} 的缓存", poinsp_refno);

    // 方法1: 重新编译后运行（推荐）
    println!("✅ 缓存清理方案:");
    println!("  1. 重新编译项目: cargo build");
    println!("  2. 重启应用程序");
    println!("  3. 缓存将自动清除，使用新的修复逻辑");

    // 方法2: 临时移除缓存装饰器进行测试
    println!("\n🔧 临时测试方案:");
    println!("  1. 注释掉 get_world_mat4 函数的 #[cached(result = true)] 装饰器");
    println!("  2. 重新编译测试");
    println!("  3. 确认修复效果后恢复缓存装饰器");

    // 验证当前缓存状态
    if let Some(cached_result) = get_world_mat4(poinsp_refno, false).await? {
        let cached_pos = cached_result.w_axis.truncate();
        let expected_pos = glam::DVec3::new(-5375.49, 1771.29, -2607.01);
        let error = (cached_pos - expected_pos).length();

        println!("\n📊 当前缓存状态:");
        println!("  缓存位置: {:?}", cached_pos);
        println!("  误差: {:.1}mm", error);

        if error > 100.0 {
            println!("  ⚠️ 缓存中仍是旧结果，需要清理");
        } else {
            println!("  ✅ 缓存已更新或已清除");
        }
    }

    Ok(())
}
