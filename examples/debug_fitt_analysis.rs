use aios_core::*;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化数据库连接
    init_surreal().await?;

    let fitt_refno = "25688/7960".parse::<RefnoEnum>()?;
    let parent_refno = "25688/7958".parse::<RefnoEnum>()?;

    println!("🔧 FITT 元素分析");
    println!("================");

    // 查询FITT元素属性
    let fitt_att = get_named_attmap(fitt_refno).await?;
    println!("FITT (25688/7960) 属性:");
    println!("  类型: {}", fitt_att.get_type_str());
    println!("  位置: {:?}", fitt_att.get_dvec3("POS"));
    println!("  方向: {:?}", fitt_att.get_dvec3("ORI"));
    println!("  ZDIS: {:?}", fitt_att.get_dvec3("ZDIS"));
    println!("  BANG: {:?}", fitt_att.get_f64("BANG"));

    // 查询父级STWALL元素属性
    let parent_att = get_named_attmap(parent_refno).await?;
    println!("\n父级 STWALL (25688/7958) 属性:");
    println!("  类型: {}", parent_att.get_type_str());
    println!("  位置: {:?}", parent_att.get_dvec3("POS"));
    println!("  方向: {:?}", parent_att.get_dvec3("ORI"));

    // 计算世界坐标
    if let Ok(Some(fitt_world)) = get_world_mat4_with_strategies(fitt_refno, false).await {
        let fitt_world_pos = fitt_world.project_point3(DVec3::ZERO);
        println!("\nFITT 世界坐标位置: {:?}", fitt_world_pos);
    }

    if let Ok(Some(parent_world)) = get_world_mat4_with_strategies(parent_refno, false).await {
        let parent_world_pos = parent_world.project_point3(DVec3::ZERO);
        println!("父级世界坐标位置: {:?}", parent_world_pos);
    }

    // 计算局部变换
    if let Ok(Some(local_transform)) = get_local_mat4(fitt_refno, parent_refno).await {
        let local_pos = local_transform.project_point3(DVec3::ZERO);
        println!("\nFITT 局部变换位置: {:?}", local_pos);
    }

    // 验证测试数据是否为世界坐标
    let expected_pos = DVec3::new(59375.0, 21200.0, -7350.0);
    println!("\n测试数据分析:");
    println!("期望位置: {:?}", expected_pos);

    if let Ok(Some(parent_world)) = get_world_mat4_with_strategies(parent_refno, false).await {
        if let Ok(Some(fitt_world)) = get_world_mat4_with_strategies(fitt_refno, false).await {
            let fitt_world_pos = fitt_world.project_point3(DVec3::ZERO);
            let parent_world_pos = parent_world.project_point3(DVec3::ZERO);

            // 计算实际局部偏移
            let actual_local_offset = fitt_world_pos - parent_world_pos;
            println!("实际局部偏移: {:?}", actual_local_offset);

            // 检查期望位置是否接近世界坐标
            let world_diff = (expected_pos - fitt_world_pos).length();
            let local_diff = (expected_pos - actual_local_offset).length();

            println!("期望位置 vs 世界坐标差异: {:.3}mm", world_diff);
            println!("期望位置 vs 局部偏移差异: {:.3}mm", local_diff);

            if world_diff < local_diff {
                println!("✅ 期望位置更接近世界坐标");
            } else {
                println!("✅ 期望位置更接近局部偏移");
            }
        }
    }

    Ok(())
}
