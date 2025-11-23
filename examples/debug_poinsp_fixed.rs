use aios_core::*;
use anyhow::Result;
use glam::{DVec3, DMat4, DQuat};

/// 修复后的POINSP世界变换矩阵计算
/// 正确处理SPINE子节点的路径变换
async fn get_poinsp_world_transform_fixed(poinsp_refno: RefnoEnum) -> Result<Option<DMat4>> {
    // 1. 获取POINSP基本信息
    let poinsp_att = get_named_attmap(poinsp_refno).await?;
    let poinsp_local_pos = poinsp_att.get_position().unwrap_or_default().as_dvec3();
    
    // 2. 获取父级SPINE
    let spine_refno = poinsp_att.get_owner();
    let spine_att = get_named_attmap(spine_refno).await?;
    
    if spine_att.get_type_str() != "SPINE" {
        eprintln!("POINSP的父级不是SPINE类型: {}", spine_att.get_type_str());
        return get_world_mat4(poinsp_refno, false).await;
    }
    
    // 3. 获取GENSEC（SPINE的父级）
    let gensec_refno = spine_att.get_owner();
    let gensec_att = get_named_attmap(gensec_refno).await?;
    
    if gensec_att.get_type_str() != "GENSEC" && gensec_att.get_type_str() != "WALL" {
        eprintln!("SPINE的父级不是GENSEC类型: {}", gensec_att.get_type_str());
        return get_world_mat4(poinsp_refno, false).await;
    }
    
    // 4. 获取GENSEC的世界变换矩阵
    let gensec_world_mat = match get_world_mat4(gensec_refno, false).await? {
        Some(mat) => mat,
        None => return Ok(None),
    };
    
    // 5. 获取SPINE路径信息
    let spline_pts = match get_spline_pts(gensec_refno).await {
        Ok(pts) => pts,
        Err(e) => {
            eprintln!("无法获取SPINE路径: {}", e);
            return Ok(None);
        }
    };
    
    if spline_pts.len() < 2 {
        eprintln!("SPINE路径点不足");
        return Ok(None);
    }
    
    // 6. 获取SPINE的YDIR属性（影响局部坐标系）
    let spine_ydir = spine_att.get_dvec3("YDIR");
    
    // 7. 计算SPINE路径上对应POINSP位置的变换矩阵
    // POINSP的Y坐标（622.59）表示沿SPINE路径的距离
    let distance_along_spine = poinsp_local_pos.y; // Y轴通常表示沿路径方向
    
    println!("🔍 SPINE路径分析:");
    println!("  路径点数量: {}", spline_pts.len());
    println!("  POINSP局部坐标: {:?}", poinsp_local_pos);
    println!("  沿路径距离: {:.3}", distance_along_spine);
    println!("  SPINE YDIR: {:?}", spine_ydir);
    
    // 8. 计算路径上的位置和方向
    let spine_transform = calculate_spine_transform_at_distance(&spline_pts, distance_along_spine, spine_ydir)?;
    
    println!("  SPINE路径变换矩阵:");
    println!("    位置: {:?}", spine_transform.w_axis.truncate());
    println!("    方向X: {:?}", spine_transform.x_axis.truncate());
    println!("    方向Y: {:?}", spine_transform.y_axis.truncate());
    println!("    方向Z: {:?}", spine_transform.z_axis.truncate());
    
    // 9. 应用POINSP在SPINE局部坐标系中的偏移
    // POINSP的X和Z坐标是相对于SPINE路径的横向偏移
    let poinsp_offset_in_spine = DVec3::new(poinsp_local_pos.x, 0.0, poinsp_local_pos.z);
    let poinsp_world_pos_in_spine = spine_transform.transform_point3(poinsp_offset_in_spine);
    
    println!("  POINSP在SPINE坐标系中的位置: {:?}", poinsp_world_pos_in_spine);
    
    // 10. 构建最终的世界变换矩阵
    // 使用GENSEC的世界变换 + SPINE路径变换 + POINSP偏移
    let final_transform = gensec_world_mat * spine_transform;
    let final_position = final_transform.transform_point3(poinsp_offset_in_spine);
    
    println!("🌍 最终计算结果:");
    println!("  GENSEC世界位置: {:?}", gensec_world_mat.w_axis.truncate());
    println!("  POINSP最终位置: {:?}", final_position);
    
    // 构建最终的变换矩阵（保持GENSEC的旋转，使用计算出的位置）
    let final_mat = DMat4::from_rotation_translation(
        DQuat::from_mat4(&gensec_world_mat),
        final_position
    );
    
    Ok(Some(final_mat))
}

/// 计算SPINE路径上指定距离处的变换矩阵
fn calculate_spine_transform_at_distance(
    spline_pts: &[DVec3], 
    distance: f64, 
    ydir: Option<DVec3>
) -> Result<DMat4> {
    if spline_pts.len() < 2 {
        return Err(anyhow::anyhow!("路径点不足"));
    }
    
    // 简化版本：假设SPINE是直线，使用第一段
    let start_point = spline_pts[0];
    let end_point = spline_pts[1];
    let spine_direction = (end_point - start_point).normalize();
    
    // 计算距离起点的位置
    let point_at_distance = start_point + spine_direction * distance;
    
    // 计算SPINE的方位
    let spine_rotation = if let Some(ydir_vec) = ydir {
        cal_spine_orientation_basis_with_ydir(spine_direction, Some(ydir_vec), false)
    } else {
        cal_spine_orientation_basis(spine_direction, false)
    };
    
    // 构建SPINE路径变换矩阵
    let spine_transform = DMat4::from_rotation_translation(spine_rotation, point_at_distance);
    
    Ok(spine_transform)
}

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化数据库连接
    init_surreal().await?;
    
    let poinsp_refno = RefnoEnum::from("17496_266220");
    
    println!("🔧 测试修复后的POINSP位置计算");
    println!("目标: POINSP {}", poinsp_refno);
    
    // 使用修复后的计算方法
    let fixed_transform = get_poinsp_world_transform_fixed(poinsp_refno).await?;
    
    if let Some(fixed_mat) = fixed_transform {
        let fixed_position = fixed_mat.w_axis.truncate();
        
        // 与期望位置对比
        let expected_position = DVec3::new(-5375.49, 1771.29, -2607.01);
        let diff = fixed_position - expected_position;
        
        println!("\n📊 修复后结果对比:");
        println!("  期望位置: {:?}", expected_position);
        println!("  计算位置: {:?}", fixed_position);
        println!("  位置差异: {:?}", diff);
        println!("  距离误差: {:.4} mm", diff.length());
        
        // 与原始方法对比
        if let Some(original_mat) = get_world_mat4(poinsp_refno, false).await? {
            let original_position = original_mat.w_axis.truncate();
            let improvement = (original_position - expected_position).length() - diff.length();
            
            println!("\n🔄 改进效果:");
            println!("  原始位置: {:?}", original_position);
            println!("  原始误差: {:.4} mm", (original_position - expected_position).length());
            println!("  改进幅度: {:.4} mm", improvement);
        }
        
    } else {
        println!("❌ 修复后的计算失败");
    }
    
    Ok(())
}
