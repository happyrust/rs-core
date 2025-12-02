use aios_core::*;
use anyhow::Result;
use glam::DVec3;

fn parse_pdms_direction_old(desc: &str) -> Option<DVec3> {
    // 当前验证程序中的有问题的实现
    let parts: Vec<&str> = desc.split_whitespace().collect();
    if parts.is_empty() {
        return None;
    }

    let main_axis_str = parts[0];
    let mut current_vec = get_axis_vec(main_axis_str)?;

    let mut i = 1;
    while i < parts.len() {
        if let Ok(angle) = parts[i].parse::<f64>() {
            if i + 1 >= parts.len() {
                break;
            }
            let target_axis_str = parts[i + 1];
            let target_vec = get_axis_vec(target_axis_str)?;

            let angle_rad = angle.to_radians();
            let rotation_axis = current_vec.cross(target_vec);
            if rotation_axis.length_squared() > 1e-6 {
                if current_vec.dot(target_vec).abs() < 1e-3 {
                    current_vec = current_vec * angle_rad.cos() + target_vec * angle_rad.sin();
                }
            }
            i += 2;
        } else {
            i += 1;
        }
    }

    Some(current_vec.normalize())
}

fn parse_pdms_direction_correct(desc: &str) -> Option<DVec3> {
    // 正确的解析逻辑：基于IDA分析，"N 0.0451 W 1.0416 D"应该是分量描述
    // 而不是角度旋转描述

    let parts: Vec<&str> = desc.split_whitespace().collect();
    if parts.len() < 2 {
        return None;
    }

    // 检查是否是分量格式：N 0.0451 W 1.0416 D
    // 格式：主轴 分量1 次轴1 分量2 次轴2 ...
    if parts.len() >= 5 && parts[1].parse::<f64>().is_ok() {
        let mut result = DVec3::ZERO;
        let mut i = 0;

        while i < parts.len() {
            let axis_str = parts[i];
            let axis_vec = get_axis_vec(axis_str)?;

            if i + 1 < parts.len() {
                if let Ok(magnitude) = parts[i + 1].parse::<f64>() {
                    result += axis_vec * magnitude;
                    i += 2;
                    continue;
                }
            }

            // 如果没有数值，则默认为1.0
            result += axis_vec;
            i += 1;
        }

        if result.length() > 1e-6 {
            Some(result.normalize())
        } else {
            None
        }
    } else {
        // 简单格式：如 "N", "W", "U" 等
        get_axis_vec(parts[0])
    }
}

fn get_axis_vec(s: &str) -> Option<DVec3> {
    match s {
        "N" => Some(DVec3::Y),
        "S" => Some(DVec3::NEG_Y),
        "E" => Some(DVec3::X),
        "W" => Some(DVec3::NEG_X),
        "U" => Some(DVec3::Z),
        "D" => Some(DVec3::NEG_Z),
        _ => None,
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("🔍 测试方向字符串解析逻辑");

    let test_cases = vec!["N 88.958 U", "N 0.0451 W 1.0416 D", "N", "W", "U"];

    for case in test_cases {
        println!("\n📝 测试方向: '{}'", case);

        if let Some(old_result) = parse_pdms_direction_old(case) {
            println!("  旧解析结果: {:?}", old_result);
        } else {
            println!("  旧解析: 失败");
        }

        if let Some(new_result) = parse_pdms_direction_correct(case) {
            println!("  新解析结果: {:?}", new_result);
        } else {
            println!("  新解析: 失败");
        }

        // 分析差异
        if let (Some(old), Some(new)) = (
            parse_pdms_direction_old(case),
            parse_pdms_direction_correct(case),
        ) {
            let diff = (old - new).length();
            if diff > 1e-6 {
                println!("  ⚠️ 解析结果差异: {:.6}", diff);
            } else {
                println!("  ✅ 解析结果一致");
            }
        }
    }

    // 测试实际POINSP案例
    println!("\n🎯 分析POINSP 17496/266220的期望方向:");
    let y_desc = "N 88.958 U";
    let z_desc = "N 0.0451 W 1.0416 D";

    println!("Y轴期望方向 '{}':", y_desc);
    if let Some(y_expected) = parse_pdms_direction_correct(y_desc) {
        println!("  解析结果: {:?}", y_expected);
        println!("  北向分量: {:.6}", y_expected.y);
        println!("  上向分量: {:.6}", y_expected.z);
        println!(
            "  仰角: {:.3}°",
            (y_expected.z.atan2(y_expected.y).to_degrees())
        );
    }

    println!("Z轴期望方向 '{}':", z_desc);
    if let Some(z_expected) = parse_pdms_direction_correct(z_desc) {
        println!("  解析结果: {:?}", z_expected);
        println!("  西向分量: {:.6}", -z_expected.x);
        println!("  北向分量: {:.6}", z_expected.y);
        println!("  下向分量: {:.6}", -z_expected.z);

        // 分析水平方位角
        let horizontal_angle = z_expected.y.atan2(-z_expected.x).to_degrees();
        let vertical_angle = (-z_expected.z)
            .atan2((z_expected.x * z_expected.x + z_expected.y * z_expected.y).sqrt())
            .to_degrees();
        println!("  水平方位角: {:.3}° (从北顺时针)", horizontal_angle);
        println!("  俯仰角: {:.3}° (水平为0°, 向下为正)", vertical_angle);
    }

    Ok(())
}
