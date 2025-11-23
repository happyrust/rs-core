use aios_core::*;
use anyhow::Result;
use glam::{DMat4, DQuat, DVec3};

/// 绕过缓存的POINSP世界变换计算，用于验证修复效果
async fn get_poinsp_world_transform_uncached(poinsp_refno: RefnoEnum) -> Result<Option<DMat4>> {
    // 获取POINSP基本信息
    let poinsp_att = get_named_attmap(poinsp_refno).await?;
    let poinsp_local_pos = poinsp_att.get_position().unwrap_or_default().as_dvec3();

    // 获取父级SPINE
    let spine_refno = poinsp_att.get_owner();
    let spine_att = get_named_attmap(spine_refno).await?;

    if spine_att.get_type_str() != "SPINE" {
        println!("POINSP的父级不是SPINE类型: {}", spine_att.get_type_str());
        return get_world_mat4(poinsp_refno, false).await;
    }

    // 获取GENSEC（SPINE的父级）
    let gensec_refno = spine_att.get_owner();
    let gensec_att = get_named_attmap(gensec_refno).await?;

    if gensec_att.get_type_str() != "GENSEC" && gensec_att.get_type_str() != "WALL" {
        println!("SPINE的父级不是GENSEC类型: {}", gensec_att.get_type_str());
        return get_world_mat4(poinsp_refno, false).await;
    }

    // 获取GENSEC的世界变换矩阵（绕过缓存）
    let gensec_world_mat = match calculate_world_mat4_uncached(gensec_refno).await? {
        Some(mat) => mat,
        None => return Ok(None),
    };

    // 获取SPINE路径信息
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

    // 获取SPINE的YDIR属性
    let spine_ydir = spine_att.get_dvec3("YDIR");

    // 计算SPINE路径上对应POINSP位置的变换矩阵
    let distance_along_spine = poinsp_local_pos.y;
    let spine_transform = calculate_spine_transform_at_distance_uncached(
        &spline_pts,
        distance_along_spine,
        spine_ydir,
    )?;

    // 应用POINSP在SPINE局部坐标系中的横向偏移
    let lateral_offset = DVec3::new(poinsp_local_pos.x, 0.0, poinsp_local_pos.z);
    let final_transform = gensec_world_mat * spine_transform;
    let final_position = final_transform.transform_point3(lateral_offset);

    println!("\n🔧 无缓存计算结果:");
    println!("  GENSEC世界位置: {:?}", gensec_world_mat.w_axis.truncate());
    println!(
        "  SPINE路径变换位置: {:?}",
        spine_transform.w_axis.truncate()
    );
    println!("  POINSP最终位置: {:?}", final_position);

    // 构建最终的变换矩阵
    let final_mat =
        DMat4::from_rotation_translation(DQuat::from_mat4(&gensec_world_mat), final_position);

    Ok(Some(final_mat))
}

/// 绕过缓存的世界矩阵计算（简化版本）
async fn calculate_world_mat4_uncached(refno: RefnoEnum) -> Result<Option<DMat4>> {
    // 为了避免递归异步函数问题，我们使用迭代方式
    let mut current_refno = refno;
    let mut accumulated_transform = DMat4::IDENTITY;

    // 限制递归深度避免无限循环
    for _depth in 0..10 {
        let att = get_named_attmap(current_refno).await?;
        let owner = att.get_owner();

        // 如果是根节点，停止递归
        if owner == current_refno {
            if let Some(pos) = att.get_position() {
                accumulated_transform =
                    DMat4::from_translation(pos.as_dvec3()) * accumulated_transform;
            }
            break;
        }

        // 应用当前节点的局部变换
        if let Some(local_pos) = att.get_position() {
            let local_transform = DMat4::from_translation(local_pos.as_dvec3());
            accumulated_transform = local_transform * accumulated_transform;
        }

        current_refno = owner;
    }

    Ok(Some(accumulated_transform))
}

/// 计算SPINE路径上指定距离处的变换矩阵
fn calculate_spine_transform_at_distance_uncached(
    spline_pts: &[DVec3],
    distance: f64,
    ydir: Option<DVec3>,
) -> anyhow::Result<DMat4> {
    if spline_pts.len() < 2 {
        return Err(anyhow::anyhow!("路径点不足"));
    }

    let start_point = spline_pts[0];
    let end_point = spline_pts[1];
    let spine_direction = (end_point - start_point).normalize();

    let point_at_distance = start_point + spine_direction * distance;

    let spine_rotation = if let Some(ydir_vec) = ydir {
        cal_spine_orientation_basis_with_ydir(spine_direction, Some(ydir_vec), false)
    } else {
        cal_spine_orientation_basis(spine_direction, false)
    };

    let spine_transform = DMat4::from_rotation_translation(spine_rotation, point_at_distance);
    Ok(spine_transform)
}

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化数据库连接
    init_surreal().await?;

    let poinsp_refno = RefnoEnum::from("17496_266220");

    println!("🔧 验证修复后的POINSP位置计算（绕过缓存）");
    println!("目标: POINSP {}", poinsp_refno);

    // 使用绕过缓存的计算方法
    let uncached_transform = get_poinsp_world_transform_uncached(poinsp_refno).await?;

    if let Some(uncached_mat) = uncached_transform {
        let uncached_position = uncached_mat.w_axis.truncate();

        // 与期望位置对比
        let expected_position = DVec3::new(-5375.49, 1771.29, -2607.01);
        let diff = uncached_position - expected_position;

        println!("\n📊 无缓存计算结果:");
        println!("  期望位置: {:?}", expected_position);
        println!("  计算位置: {:?}", uncached_position);
        println!("  位置差异: {:?}", diff);
        println!("  距离误差: {:.4} mm", diff.length());

        // 与缓存结果对比
        if let Some(cached_mat) = get_world_mat4(poinsp_refno, false).await? {
            let cached_position = cached_mat.w_axis.truncate();
            let cached_diff = cached_position - expected_position;

            println!("\n🔄 缓存 vs 无缓存对比:");
            println!("  缓存位置: {:?}", cached_position);
            println!("  缓存误差: {:.4} mm", cached_diff.length());
            println!("  无缓存误差: {:.4} mm", diff.length());

            let improvement = cached_diff.length() - diff.length();
            println!("  改进幅度: {:.4} mm", improvement);
        }
    } else {
        println!("❌ 无缓存计算失败");
    }

    Ok(())
}
