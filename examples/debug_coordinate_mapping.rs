use aios_core::*;
use anyhow::Result;
use glam::{DMat4, DQuat, DVec3};

/// 分析POINSP局部坐标系映射问题
#[tokio::main]
async fn main() -> Result<()> {
    // 初始化数据库连接
    init_surreal().await?;

    let poinsp_refno = RefnoEnum::from("17496_266220");

    println!("🔍 深度分析POINSP局部坐标系映射");
    println!("目标: POINSP {}", poinsp_refno);

    // 1. 获取POINSP基本信息
    let poinsp_att = get_named_attmap(poinsp_refno).await?;
    let poinsp_local_pos = poinsp_att.get_position().unwrap_or_default().as_dvec3();

    println!("\n📍 POINSP局部坐标分析:");
    println!("  局部坐标: {:?}", poinsp_local_pos);
    println!("  X分量: {:.3} (可能=横向偏移)", poinsp_local_pos.x);
    println!("  Y分量: {:.3} (可能=沿路径距离)", poinsp_local_pos.y);
    println!("  Z分量: {:.3} (可能=垂直偏移)", poinsp_local_pos.z);

    // 2. 获取SPINE信息
    let spine_refno = poinsp_att.get_owner();
    let spine_att = get_named_attmap(spine_refno).await?;
    let spine_ydir = spine_att.get_dvec3("YDIR");

    println!("\n📐 SPINE信息:");
    println!("  SPINE: {}", spine_refno);
    println!("  YDIR: {:?}", spine_ydir);

    // 3. 获取GENSEC和SPINE路径
    let gensec_refno = spine_att.get_owner();
    let spline_pts = get_spline_pts(gensec_refno).await?;

    println!("\n🛤️ SPINE路径信息:");
    for (i, pt) in spline_pts.iter().enumerate() {
        println!("  路径点{}: {:?}", i, pt);
    }

    // 4. 测试不同的坐标轴映射假设
    println!("\n🧪 测试坐标轴映射假设:");

    let gensec_world_mat = get_world_mat4(gensec_refno, false).await?.unwrap();
    let gensec_world_pos = gensec_world_mat.w_axis.truncate();

    // 假设1: Y=沿路径距离, X=横向, Z=垂直 (当前实现)
    let result1 =
        calculate_with_mapping1(&spline_pts, poinsp_local_pos, spine_ydir, &gensec_world_mat)
            .await?;
    println!(
        "假设1 (Y=路径, X=横向, Z=垂直): 误差 {:.3}mm",
        result1.error
    );
    println!("  计算位置: {:?}", result1.position);

    // 假设2: X=沿路径距离, Y=横向, Z=垂直
    let result2 =
        calculate_with_mapping2(&spline_pts, poinsp_local_pos, spine_ydir, &gensec_world_mat)
            .await?;
    println!(
        "假设2 (X=路径, Y=横向, Z=垂直): 误差 {:.3}mm",
        result2.error
    );
    println!("  计算位置: {:?}", result2.position);

    // 假设3: Z=沿路径距离, X=横向, Y=垂直
    let result3 =
        calculate_with_mapping3(&spline_pts, poinsp_local_pos, spine_ydir, &gensec_world_mat)
            .await?;
    println!(
        "假设3 (Z=路径, X=横向, Y=垂直): 误差 {:.3}mm",
        result3.error
    );
    println!("  计算位置: {:?}", result3.position);

    // 5. 分析最佳假设
    let results = vec![result1, result2, result3];
    let best_result = results
        .iter()
        .min_by(|a, b| a.error.partial_cmp(&b.error).unwrap());

    if let Some(best) = best_result {
        println!("\n🎯 最佳映射假设:");
        println!("  {}", best.description);
        println!("  误差: {:.3}mm", best.error);
        println!("  位置: {:?}", best.position);

        let expected = DVec3::new(-5375.49, 1771.29, -2607.01);
        let diff = best.position - expected;
        println!("  与期望差异: {:?}", diff);
    }

    Ok(())
}

struct CalculationResult {
    position: DVec3,
    error: f64,
    description: String,
}

/// 假设1: Y=沿路径距离, X=横向, Z=垂直
async fn calculate_with_mapping1(
    spline_pts: &[DVec3],
    local_pos: DVec3,
    ydir: Option<DVec3>,
    gensec_mat: &DMat4,
) -> Result<CalculationResult> {
    let distance_along = local_pos.y;
    let lateral_offset = DVec3::new(local_pos.x, 0.0, local_pos.z);

    let spine_transform = calculate_spine_transform_at_distance(spline_pts, distance_along, ydir)?;
    let final_pos = gensec_mat.transform_point3(spine_transform.transform_point3(lateral_offset));

    let expected = DVec3::new(-5375.49, 1771.29, -2607.01);
    let error = (final_pos - expected).length();

    Ok(CalculationResult {
        position: final_pos,
        error,
        description: "假设1: Y=沿路径距离, X=横向, Z=垂直".to_string(),
    })
}

/// 假设2: X=沿路径距离, Y=横向, Z=垂直
async fn calculate_with_mapping2(
    spline_pts: &[DVec3],
    local_pos: DVec3,
    ydir: Option<DVec3>,
    gensec_mat: &DMat4,
) -> Result<CalculationResult> {
    let distance_along = local_pos.x;
    let lateral_offset = DVec3::new(0.0, local_pos.y, local_pos.z);

    let spine_transform = calculate_spine_transform_at_distance(spline_pts, distance_along, ydir)?;
    let final_pos = gensec_mat.transform_point3(spine_transform.transform_point3(lateral_offset));

    let expected = DVec3::new(-5375.49, 1771.29, -2607.01);
    let error = (final_pos - expected).length();

    Ok(CalculationResult {
        position: final_pos,
        error,
        description: "假设2: X=沿路径距离, Y=横向, Z=垂直".to_string(),
    })
}

/// 假设3: Z=沿路径距离, X=横向, Y=垂直
async fn calculate_with_mapping3(
    spline_pts: &[DVec3],
    local_pos: DVec3,
    ydir: Option<DVec3>,
    gensec_mat: &DMat4,
) -> Result<CalculationResult> {
    let distance_along = local_pos.z;
    let lateral_offset = DVec3::new(local_pos.x, local_pos.y, 0.0);

    let spine_transform = calculate_spine_transform_at_distance(spline_pts, distance_along, ydir)?;
    let final_pos = gensec_mat.transform_point3(spine_transform.transform_point3(lateral_offset));

    let expected = DVec3::new(-5375.49, 1771.29, -2607.01);
    let error = (final_pos - expected).length();

    Ok(CalculationResult {
        position: final_pos,
        error,
        description: "假设3: Z=沿路径距离, X=横向, Y=垂直".to_string(),
    })
}

/// 计算SPINE路径上指定距离处的变换矩阵
fn calculate_spine_transform_at_distance(
    spline_pts: &[DVec3],
    distance: f64,
    ydir: Option<DVec3>,
) -> Result<DMat4> {
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
