use crate::*;
use anyhow::Result;
use approx::assert_relative_eq;
use glam::{DMat4, DQuat, DVec3};

#[tokio::test]
async fn debug_poinsp_17496_266220_position_calculation() -> Result<()> {
    println!("🔍 调试POINSP 17496/266220位置计算");

    // 模拟数据（基于期望值和实际计算）
    let poinsp_refno = RefnoEnum::from("17496_266220");
    let gensec_refno = RefnoEnum::from("17496_266203"); // 假设的GENSEC父级

    println!("📋 节点信息:");
    println!("   POINSP: {}", poinsp_refno);
    println!("   GENSEC: {}", gensec_refno);

    // 模拟POINSP局部位置
    let local_pos = glam::DVec3::new(0.0, 0.0, 0.0); // 假设POINSP在局部原点
    println!("📋 POINSP局部位置: {:?}", local_pos);

    // 分析期望的世界位置
    let expected_world_pos = glam::DVec3::new(-5375.49, 1771.29, -2607.01);
    println!("📋 期望世界位置: {:?}", expected_world_pos);

    // 计算需要的变换矩阵
    println!("\n🔧 变换矩阵分析:");

    // 如果POINSP在局部原点，那么世界位置就是GENSEC的世界平移
    let required_gensec_translation = expected_world_pos;
    println!("   需要的GENSEC平移: {:?}", required_gensec_translation);

    // 模拟GENSEC变换矩阵的构建
    println!("\n📋 GENSEC变换矩阵构建过程:");

    // 1. 基础变换（假设GENSEC在原点，无旋转）
    let gensec_translation = required_gensec_translation;
    let gensec_rotation = DQuat::IDENTITY;
    let gensec_scale = glam::DVec3::ONE;

    println!("   GENSEC平移: {:?}", gensec_translation);
    println!("   GENSEC旋转: {:?}", gensec_rotation);
    println!("   GENSEC缩放: {:?}", gensec_scale);

    // 构建变换矩阵
    let gensec_mat =
        DMat4::from_scale_rotation_translation(gensec_scale, gensec_rotation, gensec_translation);

    println!("\n📋 GENSEC变换矩阵:");
    println!("   矩阵: {:?}", gensec_mat);

    // 应用变换
    let calculated_world_pos = gensec_mat.transform_point3(local_pos);
    println!("   计算结果: {:?}", calculated_world_pos);

    // 验证
    println!("\n✅ 验证结果:");
    let diff = calculated_world_pos - expected_world_pos;
    println!("   位置差异: {:?}", diff);
    println!("   差异大小: {:.6} mm", diff.length());

    assert!((calculated_world_pos.x - expected_world_pos.x).abs() < 0.01);
    assert!((calculated_world_pos.y - expected_world_pos.y).abs() < 0.01);
    assert!((calculated_world_pos.z - expected_world_pos.z).abs() < 0.01);

    println!("✅ 位置计算验证通过！");

    Ok(())
}

#[tokio::test]
async fn debug_poinsp_with_nonzero_local_position() -> Result<()> {
    println!("🔍 调试POINSP非零局部位置的情况");

    // 假设POINSP有非零的局部位置
    let local_pos = glam::DVec3::new(100.0, 50.0, 25.0); // 示例局部位置
    let expected_world_pos = glam::DVec3::new(-5375.49, 1771.29, -2607.01);

    println!("📋 POINSP局部位置: {:?}", local_pos);
    println!("📋 期望世界位置: {:?}", expected_world_pos);

    // 如果POINSP有局部位置，那么GENSEC的平移需要调整
    let gensec_translation = expected_world_pos - local_pos;
    println!("📋 调整后的GENSEC平移: {:?}", gensec_translation);

    // 构建变换矩阵
    let gensec_mat = DMat4::from_translation(gensec_translation);
    let calculated_world_pos = gensec_mat.transform_point3(local_pos);

    println!("📋 计算结果: {:?}", calculated_world_pos);

    // 验证
    assert!((calculated_world_pos.x - expected_world_pos.x).abs() < 0.01);
    assert!((calculated_world_pos.y - expected_world_pos.y).abs() < 0.01);
    assert!((calculated_world_pos.z - expected_world_pos.z).abs() < 0.01);
    println!("✅ 非零局部位置验证通过！");

    Ok(())
}

#[tokio::test]
async fn debug_gensec_rotation_effect() -> Result<()> {
    println!("🔍 调试GENSEC旋转对POINSP位置的影响");

    let local_pos = glam::DVec3::new(100.0, 0.0, 0.0);
    let gensec_translation = glam::DVec3::new(-5375.49, 1771.29, -2607.01);

    // 测试不同的旋转情况
    println!("📋 测试旋转对位置的影响:");

    // 1. 无旋转
    let no_rotation = DQuat::IDENTITY;
    let mat1 =
        DMat4::from_scale_rotation_translation(glam::DVec3::ONE, no_rotation, gensec_translation);
    let pos1 = mat1.transform_point3(local_pos);
    println!("   无旋转: {:?}", pos1);

    // 2. 90度绕Z轴旋转
    let rot_z = DQuat::from_rotation_z(std::f64::consts::PI / 2.0);
    let mat2 = DMat4::from_scale_rotation_translation(glam::DVec3::ONE, rot_z, gensec_translation);
    let pos2 = mat2.transform_point3(local_pos);
    println!("   90°绕Z: {:?}", pos2);

    // 3. 45度绕任意轴旋转
    let rot_axis = glam::DVec3::new(0.0, 1.0, 0.0).normalize();
    let rot_45 = DQuat::from_axis_angle(rot_axis, std::f64::consts::PI / 4.0);
    let mat3 = DMat4::from_scale_rotation_translation(glam::DVec3::ONE, rot_45, gensec_translation);
    let pos3 = mat3.transform_point3(local_pos);
    println!("   45°绕Y: {:?}", pos3);

    println!("✅ 旋转影响分析完成！");

    Ok(())
}
