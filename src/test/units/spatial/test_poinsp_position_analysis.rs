use crate::*;
use anyhow::Result;
use approx::assert_relative_eq;

#[tokio::test]
async fn test_poinsp_position_math_analysis() -> Result<()> {
    println!("🔍 Analyzing POINSP position calculation math logic");

    // 模拟17496/266220的位置计算数据
    let local_pos = glam::DVec3::new(0.0, 0.0, 0.0); // 假设POINSP在局部原点
    let expected_world_pos = glam::DVec3::new(-5375.49, 1771.29, -2607.01);

    println!("📋 Local position: {:?}", local_pos);
    println!("📋 Expected world position: {:?}", expected_world_pos);

    // 分析期望位置的坐标系特征
    let pos_magnitude = expected_world_pos.length();
    let pos_normalized = expected_world_pos.normalize();

    println!("📋 Position magnitude: {:.2} mm", pos_magnitude);
    println!("📋 Position direction: {:?}", pos_normalized);

    // 分析位置在各个坐标轴上的分量
    println!("📋 Position components:");
    println!(
        "   East-West (X): {:.2} mm ({} West)",
        expected_world_pos.x.abs(),
        if expected_world_pos.x < 0.0 {
            ""
        } else {
            "East"
        }
    );
    println!(
        "   North-South (Y): {:.2} mm ({} North)",
        expected_world_pos.y.abs(),
        if expected_world_pos.y > 0.0 {
            ""
        } else {
            "South"
        }
    );
    println!(
        "   Up-Down (Z): {:.2} mm ({} Down)",
        expected_world_pos.z.abs(),
        if expected_world_pos.z < 0.0 { "" } else { "Up" }
    );

    // 计算位置的方向角度（用于验证坐标系）
    let horizontal_dist = (expected_world_pos.x * expected_world_pos.x
        + expected_world_pos.y * expected_world_pos.y)
        .sqrt();
    let azimuth = expected_world_pos
        .y
        .atan2(expected_world_pos.x)
        .to_degrees();
    let elevation = (-expected_world_pos.z).atan2(horizontal_dist).to_degrees();

    println!("📋 Position spherical coordinates:");
    println!("   Horizontal distance: {:.2} mm", horizontal_dist);
    println!("   Azimuth (from East): {:.2}°", azimuth);
    println!("   Elevation (from horizontal): {:.2}°", elevation);

    // 模拟GENSEC变换矩阵的作用
    // 假设GENSEC有平移、旋转和可能的缩放
    println!("📋 GENSEC transformation analysis:");
    println!("   The world matrix should include:");
    println!("   - Translation: moves POINSP to world position");
    println!("   - Rotation: aligns local axes with world axes");
    println!("   - Scale: applies any size scaling factors");

    // 验证位置合理性
    assert!(
        expected_world_pos.x < -5000.0,
        "Should be significantly West"
    );
    assert!(
        expected_world_pos.y > 1000.0,
        "Should be significantly North"
    );
    assert!(
        expected_world_pos.z < -2000.0,
        "Should be significantly Down"
    );

    println!("✅ POINSP position math analysis completed!");

    Ok(())
}

#[tokio::test]
async fn test_spine_coordinate_system_analysis() -> Result<()> {
    println!("🔍 Analyzing SPINE coordinate system characteristics");

    // 基于前面测试的方向数据
    let spine_z = glam::DVec3::new(
        -0.0007869044836398384,
        0.9998344368711255,
        -0.01817909865569267,
    );
    let spine_y = glam::DVec3::new(
        -1.4307578617685256e-5,
        0.01817909302541243,
        0.9998347465316791,
    );
    let spine_x = glam::DVec3::new(
        -0.9999996902882655,
        -0.0007870345438278944,
        5.421010862427522e-20,
    );

    println!("📋 SPINE Local Axes in World Coordinates:");
    println!("   Local X (East): {:?}", spine_x);
    println!("   Local Y (Up):   {:?}", spine_y);
    println!("   Local Z (North): {:?}", spine_z);

    // 分析每个轴的世界方向
    fn analyze_axis(name: &str, axis: glam::DVec3) {
        let x_deg = axis.x.acos().to_degrees();
        let y_deg = axis.y.acos().to_degrees();
        let z_deg = axis.z.acos().to_degrees();

        println!("📋 {} axis world direction:", name);
        println!("   Angle from East: {:.2}°", x_deg);
        println!("   Angle from North: {:.2}°", y_deg);
        println!("   Angle from Up: {:.2}°", z_deg);

        // 主要方向判断
        let abs_x = axis.x.abs();
        let abs_y = axis.y.abs();
        let abs_z = axis.z.abs();

        if abs_x > 0.9 {
            println!(
                "   Primary direction: {}",
                if axis.x > 0.0 { "East" } else { "West" }
            );
        } else if abs_y > 0.9 {
            println!(
                "   Primary direction: {}",
                if axis.y > 0.0 { "North" } else { "South" }
            );
        } else if abs_z > 0.9 {
            println!(
                "   Primary direction: {}",
                if axis.z > 0.0 { "Up" } else { "Down" }
            );
        } else {
            println!("   Primary direction: Mixed");
        }
    }

    analyze_axis("X", spine_x);
    analyze_axis("Y", spine_y);
    analyze_axis("Z", spine_z);

    // 分析构件类型特征
    println!("📋 Component type analysis:");
    let verticality = spine_z.dot(glam::DVec3::Z).abs();
    if verticality > 0.99 {
        println!("   Type: Vertical component (spine points up/down)");
    } else if verticality < 0.1 {
        println!("   Type: Horizontal component (spine points horizontally)");
    } else {
        println!(
            "   Type: Inclined component (verticality: {:.1}%)",
            verticality * 100.0
        );
    }

    println!("✅ SPINE coordinate system analysis completed!");

    Ok(())
}
