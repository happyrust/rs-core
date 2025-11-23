use aios_core::*;
use anyhow::Result;
use glam::DVec3;

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化数据库连接
    init_surreal().await?;
    
    let fitt_refno: RefnoEnum = "25688/7960".parse()
        .map_err(|e| anyhow::anyhow!("解析FITT参考号失败: {}", e))?;
    let parent_refno: RefnoEnum = "25688/7958".parse()
        .map_err(|e| anyhow::anyhow!("解析父级参考号失败: {}", e))?;
    
    println!("🔍 FITT 真值验证分析");
    println!("==================");
    
    // 1. 查询FITT的实际世界坐标（使用新策略）
    println!("\n📊 FITT 实际世界坐标:");
    if let Ok(Some(fitt_world)) = get_world_mat4_with_strategies(fitt_refno, false).await {
        let fitt_world_pos = fitt_world.project_point3(DVec3::ZERO);
        let fitt_world_y = fitt_world.transform_vector3(DVec3::Y);
        let fitt_world_z = fitt_world.transform_vector3(DVec3::Z);
        
        println!("  世界位置: {:?}", fitt_world_pos);
        println!("  世界Y轴: {:?}", fitt_world_y);
        println!("  世界Z轴: {:?}", fitt_world_z);
        
        // 转换为方向字符串
        let fitt_y_dir = direction_to_string(fitt_world_y);
        let fitt_z_dir = direction_to_string(fitt_world_z);
        println!("  世界方向: Y is {}, Z is {}", fitt_y_dir, fitt_z_dir);
        
        // 2. 计算相对于父级的局部偏移
        if let Ok(Some(parent_world)) = get_world_mat4_with_strategies(parent_refno, false).await {
            let parent_world_pos = parent_world.project_point3(DVec3::ZERO);
            let actual_local_offset = fitt_world_pos - parent_world_pos;
            
            println!("\n📊 实际局部偏移:");
            println!("  FITT世界位置: {:?}", fitt_world_pos);
            println!("  父级世界位置: {:?}", parent_world_pos);
            println!("  实际局部偏移: {:?}", actual_local_offset);
            
            // 3. 与测试数据对比
            println!("\n📊 测试数据对比:");
            let expected_local_pos = DVec3::new(0.0, 1450.0, 6575.0);
            let expected_world_pos = DVec3::new(59375.0, 21200.0, -7350.0);
            
            println!("  期望局部位置: {:?}", expected_local_pos);
            println!("  期望世界位置: {:?}", expected_world_pos);
            
            let local_diff = (actual_local_offset - expected_local_pos).length();
            let world_diff = (fitt_world_pos - expected_world_pos).length();
            
            println!("  局部位置差异: {:.3}mm", local_diff);
            println!("  世界位置差异: {:.3}mm", world_diff);
            
            // 4. 判断测试数据来源
            println!("\n📊 测试数据来源分析:");
            if world_diff < local_diff {
                println!("  ✅ 期望位置更接近世界坐标");
                println!("  📝 测试数据可能基于世界坐标编写");
            } else {
                println!("  ✅ 期望位置更接近局部偏移");
                println!("  📝 测试数据可能基于局部坐标编写");
            }
            
            // 5. 坐标系分析
            println!("\n📊 坐标系分析:");
            println!("  实际世界方向: Y is {}, Z is {}", fitt_y_dir, fitt_z_dir);
            println!("  期望方向: Y is E, Z is S");
            
            // 检查是否需要90度旋转
            let expected_y = DVec3::new(1.0, 0.0, 0.0); // E
            let expected_z = DVec3::new(0.0, -1.0, 0.0); // S
            
            let y_similarity = fitt_world_y.dot(expected_y);
            let z_similarity = fitt_world_z.dot(expected_z);
            
            println!("  Y轴相似度: {:.3}", y_similarity);
            println!("  Z轴相似度: {:.3}", z_similarity);
            
            if y_similarity.abs() < 0.5 || z_similarity.abs() < 0.5 {
                println!("  ⚠️  坐标系方向不匹配，可能需要旋转");
            }
        }
    }
    
    // 6. 检查FITT元素属性
    println!("\n📊 FITT 元素属性:");
    let fitt_att = get_named_attmap(fitt_refno).await?;
    println!("  类型: {}", fitt_att.get_type_str());
    println!("  POS: {:?}", fitt_att.get_dvec3("POS"));
    println!("  ORI: {:?}", fitt_att.get_dvec3("ORI"));
    println!("  ZDIS: {:?}", fitt_att.get_dvec3("ZDIS"));
    println!("  BANG: {:?}", fitt_att.get_f64("BANG"));
    
    Ok(())
}

fn direction_to_string(dir: DVec3) -> String {
    let threshold = 0.5;
    let mut components = Vec::new();
    
    if dir.x.abs() > threshold {
        if dir.x > 0.0 {
            components.push("E");
        } else {
            components.push("W");
        }
    }
    
    if dir.y.abs() > threshold {
        if dir.y > 0.0 {
            components.push("N");
        } else {
            components.push("S");
        }
    }
    
    if dir.z.abs() > threshold {
        if dir.z > 0.0 {
            components.push("U");
        } else {
            components.push("D");
        }
    }
    
    if components.is_empty() {
        "UNKNOWN".to_string()
    } else {
        components.join(" ")
    }
}
