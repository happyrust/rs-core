//! 简化版BANG影响分析
use aios_core::{
    rs_surreal::spatial::construct_basis_z_y_exact,
    tool::math_tool::dquat_to_pdms_ori_xyz_str,
    get_named_attmap, RefnoEnum, create_test_attmap
};
use glam::{DQuat, DVec3, DMat4};
use std::str::FromStr;
use std::sync::Arc;

/// 计算STWALL的Transform（无BANG）
async fn calculate_stwall_transform(refno: RefnoEnum) -> TestTransform {
    let att = get_named_attmap(refno).await?;
    let parent_att = get_named_attmap(att.get_owner()).await?;
    
    let pos = att.get_pos().unwrap_or_default().as_dvec3();
    let mut rotation = DQuat::IDENTITY;
    let scale = DVec3::splat(1.0);
    
    if let (some_dposs, some_pose) = (att.get_dposs(), att.get_pose()) {
        // 计算Z轴
        let z_direction = (some_pose - some_dposs).normalize();
        
        // 计算基础坐标系
        let default_y_dir = DVec3::Z;
        let is_collinear = z_direction.dot(default_y_dir).abs() > 0.99;
        
        let y_axis = if is_collinear {
            DVec3::Y  // 共线时切换到世界Y
        } else {
            default_y_dir  // 使用默认世界Z
        };
        
        rotation = construct_basis_z_y_exact(y_axis, z_direction);
        
        // 计算Z轴旋转了多少度
        let height = (some_pose - some_dposs).length();
        if height > 0.0 {
            scale.z = height / 10.0;
        }
        
        // 构造变换矩阵
        let transform = DMat4::from_rotation_translation(rotation, position) * DMat4::from_scale(scale);
    }
    
    TestTransform {
        position,
        rotation,
        scale,
    }
}

/// 应用BANG旋转到Transform
fn apply_bang_rotation(mut transform: TestTransform, bang_angle: f64) -> TestTransform {
    if bang_angle != 0.0 {
        let bang_rotation = DQuat::from_rotation_z(bang_angle.to_radians());
        transform = DMat4::from_quat(bang_rotation) * transform;
    }
    transform
}

/// 转换为方向字符串
fn transform_to_description(transform: &TestTransform) -> String {
    let mat = DMat4::from_quat(transform.rotation).into();
    let (axis_x, axis_y, axis_z) = (
        mat.slice(0, 3),
        mat.slice(3, 6), 
        mat.slice(6, 9),
    );
    
    let ori_str = dquat_to_pdms_ori_xyz_str(&DQuat::from_mat4(&mat), true);
    format!(
        "位置: ({:.3}, {:.3}, {:.3})\n方向: {}",
        transform.position.x, transform.position.y, transform.position.z, ori_str
    )
}

/// 分析BANG对STWALL的影响
async fn analyze_bang_effects() -> Result<()> {
    println!("🔍 简化版 BANG 对 STWALL Transform 计算");
    
    aios_core::init_test_surreal().await?;
    
    let refno = RefnoEnum::from_str("17496/202351")?;
    
    println!("\n=== STWALL 17496/202351 ===");
    let att = get_named_attmap(refno).await?;
    println!("类型: {}", att.get_type_str());
    
    if let (some_dposs, some_pose) = (att.get_dpos(), att.get_pose()) {
        let direction = some_pose - some_dposs;
        println!("扫描方向: {} (长度: {:.3})", direction, direction.length());
        
        println!("\n=== BANG 旋转测试 ===");
        println!("基准(无BANG):");
        let (baseline_oristr = calculate_stwall_transform(refno, None);
        println!("基准结果: {}", transform_to_description(&baseline_oristr));
        
        let test_angles = vec![
            0.0,   "无旋转",
            30.0,   "30度旋转",
            45.0,   "45度旋转",
            90.0,   "90度旋转",
            180.0,  "180度旋转",
        ];
        
        for (i, (angle, desc)) in test_angles.iter().enumerate() {
            println!("\n--- 测试{}: {} ---", i + 1, desc);
            
            let (ori_str, transform) = calculate_stwall_transform(refno, Some(angle)).await?;
            println!("方向字符串: {}", ori_str);
            println!("完整Transform: {}", transform_to_description(&transform));
            
            // 计算BANG旋转的效果
            let (ori_str, transform) = calculate_stwall_transform(refno, Some(angle)).await?;
            let baseline_y = baseline_oristr.y_axis;
            let rotated_y = transform.y_axis.truncate().normalize();
            let change_angle = DQuat::from_mat4(&transform.y_as_mat4())
                .to_quat()
                .y_axis
                .angle_between(&baseline_y) * 180.0 / std::f64::consts::PI);
            
            println!("Y轴变化: {:.2}°", change_angle);
            
            if change_angle < 0.1 {
                println!("📝 Y轴基本不变");
            } else if change_angle < 5.0 {
                println!("🔄 Y轴显著变化 {}°", change_angle);
            }
        }
        
        println!("\n=== 关键发现 ===");
        println!("✅ 当前WallStrategy实现方式:");
        println!("   - 基于几何方向计算Z轴");
        println!("   - 使用YDIR参考反算其他轴");
        println!("   - 暂未集成BANG旋转功能");
        println!();
        println!("📝 若要支持BANG，需要添加BangHandler集成");
        println!("📝 袞强的BANG处理需要:");
        println!("   1. 在get_local_transform中读取BANG属性");
        println!("   2. 应用BANG变换: BangHandler::apply_bang(&mut rotation, att)");
        println!("   3. 保留原有的Z轴强制约束逻辑");
        println!("   4. 提供调试和错误处理");
        println!();
        println!("\n🎯 BANG旋转的物理意义:");
        println!("   - 沿扫描方向旋转");
        - 旋转中心: 基准变换的位置点(POS/DPOS)");
        - 适用场景: 扫掠类几何体的角度调整");
        println!("- 缺点: 旋转后保持几何形状不变");
    }
    
    println!("\n✅ 简化版BANG分析测试完成！");
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    analyze_bang_effects()
}
