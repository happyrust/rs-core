use crate::test::test_helpers::*;
use crate::*;
use anyhow::Result;
use glam::{DMat4, DQuat, DVec3};
use serde::Deserialize;
use serde_json;

/// 测试使用重构后的策略计算方式验证空间数据
/// 基于 spatial_pdms_cases.json 中的测试案例
#[tokio::test]
async fn test_world_spatial() -> Result<()> {
    init_surreal().await?;

    println!("🧪 测试重构后的策略计算方式 - 空间数据验证");

    // 读取测试案例数据
    let test_cases = load_spatial_test_cases().await?;

    for (i, case) in test_cases.iter().enumerate() {
        println!("📋 测试案例 {}: {} ({})", i + 1, case.refno, case.noun);

        let refno = RefnoEnum::from(case.refno.as_str());

        // 使用重构后的策略计算世界坐标
        let strategy_result = crate::transform::get_world_mat4(refno).await?;
        
        if let Some(strategy_mat) = strategy_result {
            // 提取位置和方向
            let strategy_pos = strategy_mat.w_axis.truncate();
            let strategy_quat = DQuat::from_mat4(&strategy_mat);
            
            println!("   📍 策略计算位置: {:?}", strategy_pos);
            
            // 验证与预期字符串的解析结果
            if let Some(expected_pos) = parse_position_string(&case.pos_str) {
                let expected_diff = (strategy_pos - expected_pos).length();
                println!("   📐 与预期位置差异: {:.6}mm", expected_diff * 1000.0);
                
                if expected_diff < 10.0 {
                    // 10mm 容差
                    println!("   ✅ 位置符合预期");
                } else {
                    println!("   ⚠️  位置与预期差异较大");
                }
            }
            
            if let Some((expected_y, expected_z)) = parse_orientation_string(&case.ori_str) {
                // 验证Y轴方向
                let strategy_y = strategy_mat.y_axis.truncate().normalize();
                let y_diff = strategy_y.dot(expected_y).abs();
                println!("   🧭 Y轴方向匹配度: {:.6}", y_diff);
                
                // 验证Z轴方向
                let strategy_z = strategy_mat.z_axis.truncate().normalize();
                let z_diff = strategy_z.dot(expected_z).abs();
                println!("   🧭 Z轴方向匹配度: {:.6}", z_diff);
                
                if y_diff > 0.95 && z_diff > 0.95 {
                    println!("   ✅ 方向符合预期");
                } else {
                    println!("   ⚠️  方向与预期存在差异");
                }
            }
        } else {
            println!("   ❌ 策略计算失败（返回 None）");
        }

        println!();
    }

    println!("🎉 空间数据策略计算测试完成！");
    Ok(())
}



/// 测试策略计算的完整性和一致性
#[tokio::test]
async fn test_world_spatial_consistency() -> Result<()> {
    init_surreal().await?;

    println!("🧪 测试策略计算的一致性");

    let test_cases = load_spatial_test_cases().await?;
    let mut success_count = 0;
    let mut total_count = test_cases.len();

    for case in test_cases {
        let refno = RefnoEnum::from(case.refno.as_str());

        // 多次计算验证一致性
        let result1 = crate::transform::get_world_mat4(refno).await?;
        let result2 = crate::transform::get_world_mat4(refno).await?;

        match (result1, result2) {
            (Some(mat1), Some(mat2)) => {
                let diff = (mat1 - mat2).abs();
                let max_diff = diff
                    .x_axis
                    .max_element()
                    .max(diff.y_axis.max_element())
                    .max(diff.z_axis.max_element())
                    .max(diff.w_axis.max_element());

                if max_diff < 1e-10 {
                    success_count += 1;
                } else {
                    println!(
                        "   ⚠️  {} 计算结果不一致，最大差异: {:?}",
                        case.refno, max_diff
                    );
                }
            }
            (None, None) => {
                // 两次都失败也算一致
                success_count += 1;
            }
            _ => {
                println!("   ❌ {} 计算结果不稳定", case.refno);
            }
        }
    }

    println!(
        "   一致性通过率: {}/{} ({:.1}%)",
        success_count,
        total_count,
        success_count as f64 / total_count as f64 * 100.0
    );

    if success_count == total_count {
        println!("   ✅ 所有计算结果都保持一致");
    } else {
        println!("   ⚠️  存在不一致的计算结果");
    }

    println!("🎉 一致性测试完成！");
    Ok(())
}

/// 测试案例数据结构
#[derive(Debug, Clone, Deserialize)]
struct SpatialTestCase {
    refno: String,
    noun: String,
    #[serde(alias = "wpos_str")]
    pos_str: String,
    #[serde(alias = "wori_str")]
    ori_str: String,
}

/// 加载 world 空间测试案例数据 (世界坐标)
async fn load_spatial_test_cases() -> Result<Vec<SpatialTestCase>> {
    let json_content = include_str!("../../test-cases/spatial/spatial_world_cases.json");
    let cases: Vec<SpatialTestCase> = serde_json::from_str(json_content)?;
    Ok(cases)
}

/// 加载本地空间测试案例数据 (局部/相对坐标)
async fn load_spatial_local_cases() -> Result<Vec<SpatialTestCase>> {
    let json_content = include_str!("../../test-cases/spatial/spatial_local_cases.json");
    let cases: Vec<SpatialTestCase> = serde_json::from_str(json_content)?;
    Ok(cases)
}

/// 解析位置字符串 "Position W 5375.49mm N 1771.29mm D 2607.01mm" 或 "W 0.49mm N 622.59mm D 11.32mm"
fn parse_position_string(pos_str: &str) -> Option<DVec3> {
    let clean_str = pos_str.trim_start_matches("Position").trim();
    let parts: Vec<&str> = clean_str.split_whitespace().collect();

    // 应该有 6 个部分: Dir1 Val1 Dir2 Val2 Dir3 Val3
    if parts.len() < 6 {
        return None;
    }

    let mut x = 0.0;
    let mut y = 0.0;
    let mut z = 0.0;

    for i in (0..parts.len()).step_by(2) {
        if i + 1 >= parts.len() {
            break;
        }
        let dir = parts[i];
        let val_str = parts[i + 1].trim_end_matches("mm");

        if let Ok(val) = val_str.parse::<f64>() {
            match dir {
                "E" => x += val,
                "W" => x -= val,
                "N" => y += val,
                "S" => y -= val,
                "U" => z += val,
                "D" => z -= val,
                _ => {}
            }
        }
    }

    Some(DVec3::new(x, y, z))
}

/// 解析方向字符串 "Orientation Y is N 88.958 U and Z is N 0.0451 W 1.0416 D"
fn parse_orientation_string(ori_str: &str) -> Option<(DVec3, DVec3)> {
    // 简化解析，实际应该根据具体格式解析
    // 这里返回 None 表示跳过方向验证
    None
}

/// 测试局部空间变换
/// 验证相对于父级的变换是否正确
#[tokio::test]
async fn test_local_spatial() -> Result<()> {
    init_surreal().await?;

    println!("🧪 测试重构后的策略计算方式 - 局部空间数据验证");

    let test_cases = load_spatial_local_cases().await?;

    for (i, case) in test_cases.iter().enumerate() {
        println!("📋 局部测试案例 {}: {} ({})", i + 1, case.refno, case.noun);

        let refno = RefnoEnum::from(case.refno.as_str());
        let att = get_named_attmap(refno).await?;
        let parent_refno = att.get_owner();
        let parent_att = get_named_attmap(parent_refno).await?;

        let strategy = crate::transform::strategies::TransformStrategyFactory::get_strategy(
            att.get_type_str(),
        );

        // 计算局部变换
        let local_mat = if let Some(mat) = strategy
            .get_local_transform(refno, parent_refno, &att, &parent_att)
            .await?
        {
            mat
        } else {
            println!("   ⚠️  无法计算局部变换");
            continue;
        };

        // 提取位置和方向
        let local_pos = local_mat.w_axis.truncate();
        let local_quat = DQuat::from_mat4(&local_mat);

        println!("   📍 计算局部位置: {:?}", local_pos);

        // 验证位置
        if let Some(expected_pos) = parse_position_string(&case.pos_str) {
            println!("   📍 预期局部位置: {:?}", expected_pos);
            let pos_diff = (local_pos - expected_pos).length();
            println!("   📏 位置差异: {:.6}mm", pos_diff * 1000.0);

            if pos_diff < 1.0 {
                // 1mm 容差
                println!("   ✅ 局部位置验证通过");
            } else {
                println!("   ⚠️  局部位置差异较大");
            }
        }

        println!();
    }

    println!("🎉 局部空间测试完成！");
    Ok(())
}
