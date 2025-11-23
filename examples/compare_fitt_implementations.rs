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
    
    println!("🔍 FITT 实现对比分析");
    println!("==================");
    
    // 1. 使用新策略系统计算
    println!("\n📊 新策略系统实现:");
    if let Ok(Some(new_local)) = aios_core::transform::get_local_mat4(fitt_refno, parent_refno).await {
        let new_pos = new_local.project_point3(DVec3::ZERO);
        let new_y = new_local.transform_vector3(DVec3::Y);
        let new_z = new_local.transform_vector3(DVec3::Z);
        
        println!("  位置: {:?}", new_pos);
        println!("  Y轴: {:?}", new_y);
        println!("  Z轴: {:?}", new_z);
        
        // 转换为方向字符串
        let new_y_dir = direction_to_string(new_y);
        let new_z_dir = direction_to_string(new_z);
        println!("  方向: Y is {}, Z is {}", new_y_dir, new_z_dir);
    }
    
    // 2. 使用旧实现计算（如果可用）
    println!("\n📊 旧实现对比:");
    // 注意：这里需要调用旧的实现，如果已经被移除则需要恢复
    
    // 3. 世界坐标对比
    println!("\n📊 世界坐标对比:");
    if let Ok(Some(new_world)) = get_world_mat4_with_strategies(fitt_refno, false).await {
        let new_world_pos = new_world.project_point3(DVec3::ZERO);
        println!("  新策略世界坐标: {:?}", new_world_pos);
    }
    
    // 4. 父级坐标系分析
    println!("\n📊 父级STWALL坐标系:");
    if let Ok(Some(parent_world)) = get_world_mat4_with_strategies(parent_refno, false).await {
        let parent_y = parent_world.transform_vector3(DVec3::Y);
        let parent_z = parent_world.transform_vector3(DVec3::Z);
        
        println!("  父级Y轴: {:?}", parent_y);
        println!("  父级Z轴: {:?}", parent_z);
        
        let parent_y_dir = direction_to_string(parent_y);
        let parent_z_dir = direction_to_string(parent_z);
        println!("  父级方向: Y is {}, Z is {}", parent_y_dir, parent_z_dir);
    }
    
    // 5. 测试数据期望值
    println!("\n📊 测试数据期望:");
    println!("  期望位置: DVec3(0.0, 1450.0, 6575.0)");
    println!("  期望方向: Y is E, Z is S");
    
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
