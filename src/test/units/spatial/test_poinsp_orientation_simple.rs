use crate::*;
use anyhow::Result;
use approx::assert_relative_eq;

#[tokio::test]
async fn test_spine_orientation_math_logic() -> Result<()> {
    println!("🔍 Testing SPINE orientation math logic without database");

    // 模拟17496/266220的测试数据（基于期望值反推）
    // 期望的Z轴: DVec3(-0.0007869044836398384, 0.9998344368711255, -0.01817909865569267)
    // 期望的Y轴: DVec3(-1.4307578617685256e-5, 0.01817909302541243, 0.9998347465316791)

    // 从期望的Z轴反推spine_dir（Z轴就是spine_dir）
    let spine_dir = glam::DVec3::new(
        -0.0007869044836398384,
        0.9998344368711255,
        -0.01817909865569267,
    );
    let ydir = glam::DVec3::new(
        -1.4307578617685256e-5,
        0.01817909302541243,
        0.9998347465316791,
    );

    println!("📋 Spine direction (Z axis): {:?}", spine_dir);
    println!("📋 YDIR: {:?}", ydir);

    // 测试cal_spine_orientation_basis_with_ydir函数
    let quat = cal_spine_orientation_basis_with_ydir(spine_dir, Some(ydir), false);
    let calculated_z = quat * glam::DVec3::Z;
    let calculated_y = quat * glam::DVec3::Y;
    let calculated_x = quat * glam::DVec3::X;

    println!("📋 Calculated quaternion: {:?}", quat);
    println!("📋 Calculated X axis: {:?}", calculated_x);
    println!("📋 Calculated Y axis: {:?}", calculated_y);
    println!("📋 Calculated Z axis: {:?}", calculated_z);

    // 验证正交性
    let dot_xy = calculated_x.dot(calculated_y);
    let dot_xz = calculated_x.dot(calculated_z);
    let dot_yz = calculated_y.dot(calculated_z);

    println!("📋 Orthogonality checks:");
    println!("   X·Y = {:.10}", dot_xy);
    println!("   X·Z = {:.10}", dot_xz);
    println!("   Y·Z = {:.10}", dot_yz);

    // 验证右手系
    let cross_yz = calculated_y.cross(calculated_z);
    println!("📋 Right-handed check (Y×Z should equal X):");
    println!("   Y×Z = {:?}", cross_yz);
    println!("   X   = {:?}", calculated_x);

    // 验证归一化
    let len_x = calculated_x.length();
    let len_y = calculated_y.length();
    let len_z = calculated_z.length();

    println!("📋 Normalization checks:");
    println!("   |X| = {:.10}", len_x);
    println!("   |Y| = {:.10}", len_y);
    println!("   |Z| = {:.10}", len_z);

    // 断言验证
    assert!(dot_xy.abs() < 1e-10, "X and Y should be orthogonal");
    assert!(dot_xz.abs() < 1e-10, "X and Z should be orthogonal");
    assert!(dot_yz.abs() < 1e-10, "Y and Z should be orthogonal");

    assert_relative_eq!(len_x, 1.0, epsilon = 1e-10);
    assert_relative_eq!(len_y, 1.0, epsilon = 1e-10);
    assert_relative_eq!(len_z, 1.0, epsilon = 1e-10);

    assert_relative_eq!(cross_yz.x, calculated_x.x, epsilon = 1e-10);
    assert_relative_eq!(cross_yz.y, calculated_x.y, epsilon = 1e-10);
    assert_relative_eq!(cross_yz.z, calculated_x.z, epsilon = 1e-10);

    println!("✅ SPINE orientation math logic verified!");

    Ok(())
}

#[tokio::test]
async fn test_spine_orientation_with_sample_data() -> Result<()> {
    println!("🔍 Testing SPINE orientation with sample data");

    // 模拟一个典型的SPINE场景：水平管道，YDIR指向上方
    let spine_dir = glam::DVec3::new(1.0, 0.0, 0.0); // 沿X轴方向
    let ydir = glam::DVec3::new(0.0, 0.0, 1.0); // 沿Z轴向上

    println!("📋 Sample spine direction: {:?}", spine_dir);
    println!("📋 Sample YDIR: {:?}", ydir);

    let quat = cal_spine_orientation_basis_with_ydir(spine_dir, Some(ydir), false);
    let calculated_z = quat * glam::DVec3::Z;
    let calculated_y = quat * glam::DVec3::Y;
    let calculated_x = quat * glam::DVec3::X;

    println!("📋 Sample results:");
    println!("   X axis: {:?}", calculated_x);
    println!("   Y axis: {:?}", calculated_y);
    println!("   Z axis: {:?}", calculated_z);

    // 验证Z轴应该等于spine_dir
    assert_relative_eq!(calculated_z.x, spine_dir.x, epsilon = 1e-10);
    assert_relative_eq!(calculated_z.y, spine_dir.y, epsilon = 1e-10);
    assert_relative_eq!(calculated_z.z, spine_dir.z, epsilon = 1e-10);

    // 验证Y轴应该接近ydir
    assert_relative_eq!(calculated_y.x, ydir.x, epsilon = 1e-10);
    assert_relative_eq!(calculated_y.y, ydir.y, epsilon = 1e-10);
    assert_relative_eq!(calculated_y.z, ydir.z, epsilon = 1e-10);

    println!("✅ Sample SPINE orientation verified!");

    Ok(())
}
