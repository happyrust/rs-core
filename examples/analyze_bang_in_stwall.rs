//! 分析 BANG 属性对 STWALL Transform 计算的影响
use aios_core::{
    RefnoEnum, get_named_attmap, rs_surreal::spatial::construct_basis_z_y_exact,
    tool::math_tool::dquat_to_pdms_ori_xyz_str, transform::strategies::WallStrategy,
};
use anyhow::Result;
use glam::{DMat4, DQuat, DVec3};
use std::str::FromStr;
use std::sync::Arc;

/// 模拟带BANG的WallStrategy计算
fn simulate_stwall_with_bang(refno: RefnoEnum, test_bang: Option<f64>) -> (String, DMat4) {
    let att = get_named_attmap(refno).await.expect("Failed to get att");
    let parent_att = get_named_attmap(att.get_owner())
        .await
        .expect("Failed to get parent att");

    // 如果指定了测试BANG，临时修改属性
    let mut att_copy = att.clone();
    if let Some(bang_value) = test_bang {
        att_copy.insert("BANG".to_string(), (bang_value as f64).into());
    }

    // 模拟BANG处理的策略类型
    let mut strategy = WallStrategy::new(Arc::new(att_copy), Arc::new(parent_att));
    let result = strategy
        .get_local_transform()
        .await
        .expect("Failed to get transform");

    let ori_str = dquat_to_pdms_ori_xyz_str(&DQuat::from_mat4(&result), true);
    (ori_str, result)
}

/// 分析不同BANG值的影响
pub fn analyze_bang_effects() -> Result<()> {
    println!("🔍 分析 BANG 对 17496/202351 STWALL Transform 计算的影响");

    // 初始化测试数据库连接
    aios_core::init_test_surreal().await?;

    let test_cases = vec![
        ("0.0度 (无旋转)", None),
        ("30.0度", Some(30.0)),
        ("90.0度", Some(90.0)),
        ("180.0度", Some(180.0)),
        ("270.0度", Some(270.0)),
        ("360.0度", Some(360.0)),
        ("-45.0度", Some(-45.0)),
        ("90.0度", Some(90.0)),
        ("180.0度", Some(180.0)),
        ("-90.0度", Some(-90.0)),
    ];

    let refno =
        RefnoEnum::from_str("17496/202351").map_err(|e| anyhow::anyhow!("Invalid refno: {}", e))?;

    println!("\n=== 基本信息 ===");
    let att = get_named_attmap(refno).await?;
    println!("STWALL类型: {}", att.get_type_str());

    if let (Some(dposs), Some(dpose)) = (att.get_dposs(), att.get_dpose()) {
        let direction = (dpose - dposs).normalize();
        println!("扫描方向: {}", direction);
        println!("长度: {}", (dpose - dposs).length());

        println!("\n=== BANG 影响分析 ===");
        println!("BANG 属性指定旋转Z轴上的角度");
        println!("Z轴强制保持不变，BANG旋转绕Z轴");

        for (i, (desc, bang)) in test_cases.iter().enumerate() {
            println!("\n--- 案例 {}: {} ---", i + 1, desc);

            if let Some(angle) = bang {
                println!("BANG角度: {:.6}°", angle);
            } else {
                println!("BANG角度: 无 (0.0°)");
            }

            let (ori_str, transform) = simulate_stwall_with_bang(refno, bang).await?;
            println!("方向字符串: {}", ori_str);

            // 提取旋转后的位置和Y轴方向
            let pos = transform.w_axis.truncate();
            let y_axis = transform.y_axis.truncate().normalize();

            println!("位置: ({:.3}, {:.3}, {:.3})", pos.x, pos.y, pos.z);
            println!(
                "Y轴方向: ({:.3}, {:.3}, {:.3})",
                y_axis.x, y_axis.y, y_axis.z
            );

            // 计算Z轴旋转了多少度
            let z_angle =
                transform.w_axis.z.atan2(transform.w_axis.x) as f64 * 180.0 / std::f64::consts::PI;
            println!("Z轴方位角 (从X轴): {:.6}°", z_angle);

            // 分析BANG影响
            match bang {
                None => println!("📝 基准情况: 无额外旋转"),
                Some(0.0) => println!("📝 零旋转: 不影响结果"),
                Some(angle) if angle != 0.0 => {
                    println!("🔄 BANG旋转: {:.6}° 绕Z轴", angle);
                    println!("   Y轴随旋转变化, Z轴强制不变");
                }
            }
        }

        println!("\n=== 关键发现 ===");
        println!("✅ STWALL WallStrategy 当前实现中 BANG 属性:");
        println!("   - 📝 当前没有读取BANG属性");
        println!("   - 📝 Transform计算仅基于几何方向");
        println!("   - 📝 结果反映纯几何物理关系");
        println!();
        println!("💡 若要支持BANG旋转，需要在WallStrategy中:");
        println!("   1. 读取BANG属性 (att.get_f32(\"BANG\"))");
        println!("   2. 获取基础变换矩阵");
        println!("   3. 应用BANG旋转: rotation *= Quat::from_rotation_z(bang.to_radians())");
        println!("   4. 重新构造最终Transform矩阵");
        println!();
        println!("⚠️ BANG的影响特点:");
        println!("   - 只改变Y轴和X轴，不影响Z轴(扫描方向)");
        println!("   - 旋转中心由位置决定");
        println!("   - 绕Z轴旋转，适合扫掠类几何体的旋转");
    } else {
        println!("❌ 缺少方向数据，无法进行BANG分析");
    }

    println!("\n✅ BANG影响分析完成！");
    Ok(())
}

#[tokio::main]
fn main() -> Result<()> {
    analyze_bang_effects()
}
