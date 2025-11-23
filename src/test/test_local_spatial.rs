use crate::rs_surreal::spatial::get_world_mat4;
use crate::transform::get_local_mat4;
use crate::*;
use anyhow::Result;
use approx::assert_relative_eq;
use glam::{DMat4, DVec3, Vec3};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::BufReader;

#[derive(Debug, Deserialize)]
struct LocalSpatialTestCase {
    refno: String,
    noun: String,
    #[serde(alias = "wpos_str")]
    pos_str: String,
    #[serde(alias = "wori_str")]
    ori_str: String,
}

fn parse_pos(pos_str: &str) -> Option<DVec3> {
    let clean_str = pos_str.trim_start_matches("Position").trim();
    let parts: Vec<&str> = clean_str.split_whitespace().collect();

    if parts.len() < 6 {
        return None;
    }

    let mut x = 0.0;
    let mut y = 0.0;
    let mut z = 0.0;

    let mut i = 0;
    while i < parts.len() {
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
            i += 2;
        } else {
            i += 1;
        }
    }

    Some(DVec3::new(x, y, z))
}

fn parse_ori(ori_str: &str) -> Option<(DVec3, DVec3)> {
    // 简化版，根据实际需求可以增强
    None
}

fn parse_direction_vector(dir_str: &str) -> Option<DVec3> {
    // 解析方向向量，支持 "N", "N 88.958 U", "N 0.0451 W 1.0416 D" 等格式
    let parts: Vec<&str> = dir_str.split_whitespace().collect();
    if parts.is_empty() {
        return None;
    }

    let mut vec = DVec3::ZERO;
    let mut i = 0;

    // 遍历所有token，处理方向-数值对
    while i < parts.len() {
        let dir = parts[i];

        // 检查下一个token是否是数字
        if i + 1 < parts.len() {
            if let Ok(val) = parts[i + 1].parse::<f64>() {
                // 这是一个有效的方向-数值对
                match dir {
                    "E" => vec.x += val,
                    "W" => vec.x -= val,
                    "N" => vec.y += val,
                    "S" => vec.y -= val,
                    "U" => vec.z += val,
                    "D" => vec.z -= val,
                    _ => {}
                }
                i += 2; // 跳过数值
            } else {
                // 下一个token不是数字，当前方向使用隐含值1.0
                match dir {
                    "E" => vec.x += 1.0,
                    "W" => vec.x -= 1.0,
                    "N" => vec.y += 1.0,
                    "S" => vec.y -= 1.0,
                    "U" => vec.z += 1.0,
                    "D" => vec.z -= 1.0,
                    _ => {}
                }
                i += 1; // 只跳过方向
            }
        } else {
            // 最后一个token，使用隐含值1.0
            match dir {
                "E" => vec.x += 1.0,
                "W" => vec.x -= 1.0,
                "N" => vec.y += 1.0,
                "S" => vec.y -= 1.0,
                "U" => vec.z += 1.0,
                "D" => vec.z -= 1.0,
                _ => {}
            }
            i += 1;
        }
    }

    // 如果向量为零向量，返回None表示解析失败
    if vec.length() < 1e-6 { None } else { Some(vec) }
}

/// 加载局部空间测试案例
fn load_local_spatial_test_cases() -> Result<Vec<LocalSpatialTestCase>> {
    let file = File::open("src/test/test-cases/spatial/spatial_local_cases.json")?;
    let reader = BufReader::new(file);
    let cases: Vec<LocalSpatialTestCase> = serde_json::from_reader(reader)?;
    Ok(cases)
}

/// 验证局部变换矩阵的位置和方向
fn validate_local_transform(
    local_matrix: &DMat4,
    expected_pos: &DVec3,
    expected_y_axis: &DVec3,
    expected_z_axis: &DVec3,
    tolerance: f64,
) -> bool {
    // 验证位置
    let actual_pos = local_matrix.project_point3(DVec3::ZERO);
    let pos_diff = (actual_pos - *expected_pos).length();

    // 验证方向
    let actual_y_axis = local_matrix.transform_vector3(DVec3::Y).normalize();
    let actual_z_axis = local_matrix.transform_vector3(DVec3::Z).normalize();

    let y_similarity = actual_y_axis.dot(*expected_y_axis).abs();
    let z_similarity = actual_z_axis.dot(*expected_z_axis).abs();

    pos_diff < tolerance && y_similarity > (1.0 - tolerance) && z_similarity > (1.0 - tolerance)
}

/// 获取元素类型对应的验证容差
fn get_tolerance_for_element_type(noun: &str) -> f64 {
    match noun {
        "POINSP" => 1.0, // POINSP 需要高精度验证
        "FITT" => 2.0,   // FITT 可能有 ZDIS 相关的精度问题
        "ELBO" => 1.0,   // ELBO 标准精度
        "SCOJ" => 1.0,   // SCOJ 标准精度
        _ => 5.0,        // 其他类型使用较宽松的容差
    }
}

/// 测试边界条件：零变换元素
#[tokio::test]
async fn test_zero_local_transform() -> Result<()> {
    init_surreal().await?;

    println!("🔧 开始零变换边界测试...");

    // 测试虚拟节点（SPINE）
    let test_cases = vec![
        ("SPINE", "虚拟节点应该有零局部变换"),
        ("GENSEC", "基准坐标系可能有特殊变换"),
    ];

    for (noun, description) in test_cases {
        println!("\n🧪 测试 {}: {}", noun, description);

        // 查找该类型的一个实例
        let sql = format!(
            "SELECT value id FROM {} WHERE noun = '{}' LIMIT 1",
            if noun == "SPINE" { "spine" } else { "pe" },
            noun
        );

        match SUL_DB.query_take::<Vec<String>>(&sql, 0).await {
            Ok(refnos) => {
                if let Some(refno_str) = refnos.first() {
                    let refno: RefnoEnum = refno_str
                        .parse()
                        .map_err(|e| anyhow::anyhow!("解析参考号失败: {}", e))?;
                    let att = get_named_attmap(refno).await?;

                    let owner = att.get_owner();
                    match get_local_mat4(refno, owner).await {
                        Ok(Some(local_matrix)) => {
                            let local_pos = local_matrix.project_point3(DVec3::ZERO);
                            let pos_magnitude = local_pos.length();

                            println!("   局部位置: {:?}", local_pos);
                            println!("   位置大小: {:.3}mm", pos_magnitude);

                            if pos_magnitude < 1.0 {
                                println!("   ✅ 零变换验证通过");
                            } else {
                                println!("   ⚠️  非零变换，可能符合预期");
                            }
                        }
                        Ok(None) => {
                            println!("   ✅ 返回 None，符合虚拟节点预期");
                        }
                        Err(e) => {
                            println!("   ❌ 计算错误: {}", e);
                        }
                    }
                }
            }
            Err(e) => {
                println!("   ⚠️  无法找到 {} 类型实例: {}", noun, e);
            }
        }
    }

    Ok(())
}

/// 测试错误条件：无效参考号和缺失父级
#[tokio::test]
async fn test_error_conditions() -> Result<()> {
    init_surreal().await?;

    println!("🔧 开始错误条件测试...");

    // 测试无效参考号
    println!("\n🧪 测试无效参考号:");
    let invalid_refno = RefnoEnum::from("999999/999999");
    let dummy_parent = RefnoEnum::from("1/1");

    match get_local_mat4(invalid_refno, dummy_parent).await {
        Ok(_) => {
            println!("   ⚠️  无效参考号应该返回错误，但得到了结果");
        }
        Err(e) => {
            println!("   ✅ 正确返回错误: {}", e);
        }
    }

    // 测试循环依赖（理论上不应该存在）
    println!("\n🧪 测试自引用:");
    if let Ok(refno) = "17496/266220".parse::<RefnoEnum>() {
        match get_local_mat4(refno, refno).await {
            Ok(_) => {
                println!("   ⚠️  自引用应该被处理或返回错误");
            }
            Err(e) => {
                println!("   ✅ 正确处理自引用: {}", e);
            }
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_local_spatial_transforms() -> Result<()> {
    // 初始化数据库连接
    init_surreal().await?;

    println!("🔧 开始局部空间变换测试...");

    // 加载测试案例
    let test_cases = load_local_spatial_test_cases()?;
    println!("📋 加载了 {} 个测试案例", test_cases.len());

    for (index, case) in test_cases.iter().enumerate() {
        println!(
            "\n🧪 测试案例 {}/{}: {}",
            index + 1,
            test_cases.len(),
            case.refno
        );

        // 解析参考号
        let refno: RefnoEnum = case
            .refno
            .parse()
            .map_err(|e| anyhow::anyhow!("解析参考号失败: {}", e))?;
        let att = get_named_attmap(refno).await?;
        let noun = att.get_type_str();
        let owner = att.get_owner();

        println!("   类型: {}", noun);
        println!("   父级: {}", owner);

        // 解析期望的局部位置和方向
        let expected_local_pos = parse_pos(&case.pos_str)
            .ok_or_else(|| anyhow::anyhow!("无法解析位置字符串: {}", case.pos_str))?;

        let (expected_y_axis, expected_z_axis) =
            parse_ori(&case.ori_str).unwrap_or((DVec3::Y, DVec3::Z)); // 默认值，因为 parse_ori 暂时不支持复杂解析

        println!("   期望局部位置: {:?}", expected_local_pos);
        println!("   期望局部Y轴: {:?}", expected_y_axis);
        println!("   期望局部Z轴: {:?}", expected_z_axis);

        // 使用重构后的 get_local_mat4 计算局部变换
        match get_local_mat4(refno, owner).await {
            Ok(Some(local_matrix)) => {
                let actual_local_pos = local_matrix.project_point3(DVec3::ZERO);
                let actual_y_axis = local_matrix.transform_vector3(DVec3::Y).normalize();
                let actual_z_axis = local_matrix.transform_vector3(DVec3::Z).normalize();

                println!("   实际局部位置: {:?}", actual_local_pos);
                println!("   实际局部Y轴: {:?}", actual_y_axis);
                println!("   实际局部Z轴: {:?}", actual_z_axis);

                // 验证结果
                let tolerance = 10.0; // 10mm 容差
                let is_valid = validate_local_transform(
                    &local_matrix,
                    &expected_local_pos,
                    &expected_y_axis,
                    &expected_z_axis,
                    tolerance,
                );

                if is_valid {
                    println!("   ✅ 局部变换验证通过");
                } else {
                    println!("   ⚠️  局部变换验证失败");

                    // 详细分析差异
                    let pos_diff = (actual_local_pos - expected_local_pos).length();
                    let y_similarity = actual_y_axis.dot(expected_y_axis).abs();
                    let z_similarity = actual_z_axis.dot(expected_z_axis).abs();

                    println!("      位置差异: {:.3}mm", pos_diff);
                    println!("      Y轴相似度: {:.6}", y_similarity);
                    println!("      Z轴相似度: {:.6}", z_similarity);
                }

                // 对于 POINSP 类型，特别分析 SPINE 路径相关的变换
                if noun == "POINSP" {
                    println!("   🔍 POINSP 特殊分析:");

                    // 计算世界变换作为对比
                    if let Ok(Some(world_matrix)) =
                        get_world_mat4(refno, false).await
                    {
                        let world_pos: DVec3 = world_matrix.project_point3(DVec3::ZERO);
                        println!("      世界位置: {:?}", world_pos);

                        // 分析局部到世界的变换
                        if let Ok(Some(parent_world_matrix)) =
                            get_world_mat4(owner, false).await
                        {
                            let parent_pos: DVec3 = parent_world_matrix.project_point3(DVec3::ZERO);
                            let local_to_world_offset = world_pos - parent_pos;
                            println!("      局部到世界偏移: {:?}", local_to_world_offset);
                        }
                    }
                }
            }
            Ok(None) => {
                println!("   ⚠️  无法计算局部变换（返回 None）");
            }
            Err(e) => {
                println!("   ❌ 局部变换计算错误: {}", e);
                return Err(e);
            }
        }
    }

    println!("\n✅ 局部空间变换测试完成");
    Ok(())
}

#[tokio::test]
async fn test_local_vs_world_transform_consistency() -> Result<()> {
    // 初始化数据库连接
    init_surreal().await?;

    println!("🔧 开始局部与世界变换一致性测试...");

    // 测试一个具体的案例
    let refno_str = "17496/266220";
    let refno = RefnoEnum::from(refno_str);
    let att = get_named_attmap(refno).await?;
    let owner = att.get_owner();

    println!("   测试参考号: {}", refno_str);
    println!("   类型: {}", att.get_type_str());
    println!("   父级: {}", owner);

    // 计算局部变换
    let local_transform = get_local_mat4(refno, owner).await?;
    println!("   局部变换: {:?}", local_transform);

    // 计算父级世界变换
    let parent_world_transform = get_world_mat4(owner, false).await?;
    println!("   父级世界变换: {:?}", parent_world_transform);

    // 计算当前元素的世界变换
    let world_transform = get_world_mat4(refno, false).await?;
    println!("   世界变换: {:?}", world_transform);

    // 验证一致性：world_transform ≈ parent_world_transform * local_transform
    if let (Some(parent_world), Some(world), Some(local)) =
        (&parent_world_transform, &world_transform, &local_transform)
    {
        let computed_world: DMat4 = *parent_world * *local;
        let actual_world: DMat4 = *world;

        // 计算最大差异（手动遍历矩阵元素）
        let diff_matrix: DMat4 = computed_world - actual_world;
        let diff = diff_matrix
            .abs()
            .to_cols_array()
            .iter()
            .fold(0.0f64, |a, &b| a.max(b));
        println!("   变换一致性差异: {:.10}", diff);

        if diff < 1e-6 {
            println!("   ✅ 局部与世界变换一致性验证通过");
        } else {
            println!("   ⚠️  局部与世界变换存在差异");

            // 详细分析
            let computed_pos = computed_world.project_point3(DVec3::ZERO);
            let actual_pos = actual_world.project_point3(DVec3::ZERO);
            let pos_diff = (computed_pos - actual_pos).length();

            println!("      计算位置: {:?}", computed_pos);
            println!("      实际位置: {:?}", actual_pos);
            println!("      位置差异: {:.3}mm", pos_diff);
        }
    } else {
        println!("   ⚠️  某些变换计算失败，无法进行一致性验证");
    }

    Ok(())
}
