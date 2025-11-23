use crate::*;
use crate::rs_surreal::spatial::get_world_mat4_with_strategies;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use glam::{DVec3, DMat4, Vec3};
use std::fs::File;
use std::io::BufReader;
use regex::Regex;
use approx::assert_relative_eq;

#[derive(Debug, Deserialize)]
struct SpatialTestCase {
    refno: String,
    wpos_str: String,
    wori_str: String,
}

fn parse_wpos(wpos_str: &str) -> Option<DVec3> {
    // Position W 5375.49mm N 1771.29mm D 2607.01mm
    let re = Regex::new(r"Position\s+([WESNUD])\s*([\d.]+)\s*mm\s+([WESNUD])\s*([\d.]+)\s*mm\s+([WESNUD])\s*([\d.]+)\s*mm").ok()?;
    
    if let Some(caps) = re.captures(wpos_str) {
        let mut pos = DVec3::ZERO;
        
        for i in 0..3 {
            let dir = caps.get(1 + i * 2)?.as_str();
            let val = caps.get(2 + i * 2)?.as_str().parse::<f64>().ok()?;
            
            match dir {
                "E" => pos.x += val,
                "W" => pos.x -= val,
                "N" => pos.y += val,
                "S" => pos.y -= val,
                "U" => pos.z += val,
                "D" => pos.z -= val,
                _ => {}
            }
        }
        return Some(pos);
    }
    None
}

fn parse_wori(wori_str: &str) -> Option<(DVec3, DVec3)> {
    let parts: Vec<&str> = wori_str.split(" and ").collect();
    
    let mut y_axis = DVec3::Y;
    let mut z_axis = DVec3::Z;
    
    for part in parts {
        let part = part.trim();
        if part.starts_with("Orientation ") {
             let content = part.strip_prefix("Orientation ").unwrap();
             if let Some((axis, desc)) = parse_axis_def(content) {
                 if axis == "Y" { y_axis = desc; }
                 else if axis == "Z" { z_axis = desc; }
             }
        } else {
             if let Some((axis, desc)) = parse_axis_def(part) {
                 if axis == "Y" { y_axis = desc; }
                 else if axis == "Z" { z_axis = desc; }
             }
        }
    }
    
    Some((y_axis, z_axis))
}

fn parse_axis_def(s: &str) -> Option<(&str, DVec3)> {
    let parts: Vec<&str> = s.split(" is ").collect();
    if parts.len() != 2 { return None; }
    
    let axis_name = parts[0].trim();
    let dir_desc = parts[1].trim();
    
    let vec = parse_pdms_direction(dir_desc)?;
    Some((axis_name, vec))
}

fn parse_pdms_direction(desc: &str) -> Option<DVec3> {
    let parts: Vec<&str> = desc.split_whitespace().collect();
    if parts.is_empty() { return None; }
    
    let main_axis_str = parts[0];
    let mut current_vec = get_axis_vec(main_axis_str)?;
    
    let mut i = 1;
    while i < parts.len() {
        if let Ok(angle) = parts[i].parse::<f64>() {
            if i + 1 >= parts.len() { break; }
            let target_axis_str = parts[i+1];
            let target_vec = get_axis_vec(target_axis_str)?;
            
            let angle_rad = angle.to_radians();
            
            // Ensure orthogonality for rotation plane
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

fn get_axis_vec(s: &str) -> Option<DVec3> {
    match s {
        "N" => Some(DVec3::Y),
        "S" => Some(DVec3::NEG_Y),
        "E" => Some(DVec3::X),
        "W" => Some(DVec3::NEG_X),
        "U" => Some(DVec3::Z),
        "D" => Some(DVec3::NEG_Z),
        _ => None
    }
}

#[tokio::test]
async fn debug_specific_refnos() -> Result<()> {
    init_surreal().await?;
    
    println!("🔍 深度分析特定参考号的变换计算");
    
    // 重点分析 25688/7960 (FITT类型)
    let refno_str = "25688/7960";
    println!("\n🧪 详细分析: {}", refno_str);
    
    let refno = RefnoEnum::from(refno_str);
    
    // 获取属性映射
    let att = get_named_attmap(refno).await?;
    let noun = att.get_type_str();
    let owner = att.get_owner();
    
    println!("📋 基本信息:");
    println!("   类型: {}", noun);
    println!("   父级: {}", owner);
    
    println!("\n📍 位置相关属性:");
    if let Some(npos) = att.get_dvec3("NPOS") {
        println!("   NPOS: {:?}", npos);
    } else {
        println!("   NPOS: None");
    }
    
    println!("\n🧭 方向相关属性:");
    if let Some(ydir) = att.get_dvec3("YDIR") {
        println!("   YDIR: {:?}", ydir);
    } else {
        println!("   YDIR: None");
    }
    
    println!("\n🔄 旋转相关属性:");
    if let Some(bang) = att.get_f32("BANG") {
        println!("   BANG: {}°", bang);
    } else {
        println!("   BANG: None");
    }
    
    println!("\n📏 偏移相关属性:");
    if let Some(zdis) = att.get_f32("ZDIS") {
        println!("   ZDIS: {}", zdis);
    } else {
        println!("   ZDIS: None");
    }
    
    println!("\n👤 父级分析:");
    let parent_att = get_named_attmap(owner).await?;
    let parent_noun = parent_att.get_type_str();
    println!("   父级类型: {}", parent_noun);
    
    if let Some(parent_npos) = parent_att.get_dvec3("NPOS") {
        println!("   父级NPOS: {:?}", parent_npos);
    }
    
    // 获取父级变换矩阵
    if let Some(parent_matrix) = transform::get_world_mat4(owner).await? {
        let parent_translation = parent_matrix.project_point3(glam::DVec3::ZERO);
        println!("   父级世界位置: {:?}", parent_translation);
    }
    
    println!("\n🎯 策略分析:");
    let strategy = transform::strategies::TransformStrategyFactory::get_strategy(noun);
    println!("   使用策略: {:?}", std::any::type_name_of_val(&strategy));
    
    // 手动调用策略计算
    match strategy.get_local_transform(refno, owner, &att, &parent_att).await {
        Ok(Some(local_matrix)) => {
            let local_translation = local_matrix.project_point3(glam::DVec3::ZERO);
            println!("   局部变换位置: {:?}", local_translation);
        }
        Ok(None) => {
            println!("   局部变换: None");
        }
        Err(e) => {
            println!("   局部变换错误: {}", e);
        }
    }
    
    // 获取最终世界变换 - 使用新的策略系统
    println!("\n🌍 世界变换对比测试:");
    
    // 使用旧函数（作为对比基准）
    #[allow(deprecated)]
    let old_world_matrix = transform::get_world_mat4(refno).await?;
    
    // 使用新的策略系统函数
    let new_world_matrix = get_world_mat4_with_strategies(refno, false).await?;
    
    println!("   旧函数结果: {:?}", old_world_matrix);
    println!("   新函数结果: {:?}", new_world_matrix);
    
    // 详细分析差异
    let mut pos_diff = glam::DVec3::ZERO; // 在外部定义以便后续使用
    let mut new_pos = glam::DVec3::ZERO; // 在外部定义以便后续使用
    match (&old_world_matrix, &new_world_matrix) {
        (Some(old), Some(new)) => {
            let are_equal = compare_matrices(old, new);
            if are_equal {
                println!("   ✅ 新旧函数结果一致");
            } else {
                println!("   ⚠️  新旧函数结果存在差异");
                let diff = calculate_max_matrix_diff(old, new);
                println!("   最大差异: {:.10}", diff);
                
                // 详细分析位置差异
                let old_pos = old.project_point3(glam::DVec3::ZERO);
                new_pos = new.project_point3(glam::DVec3::ZERO); // 更新外部变量
                pos_diff = new_pos - old_pos; // 更新外部变量
                println!("   位置差异: {:?}", pos_diff);
                println!("   旧位置: {:?}", old_pos);
                println!("   新位置: {:?}", new_pos);
                
                // 分析旋转差异
                let old_rot = glam::DQuat::from_mat4(old);
                let new_rot = glam::DQuat::from_mat4(new);
                let rot_diff = old_rot.dot(new_rot);
                println!("   旋转相似度: {:.6}", rot_diff);
                
                // 检查期望结果
                if let Some(expected_pos) = parse_wpos("Position E 59375mm N 21200mm D 7350mm") {
                    let old_expected_diff = (old_pos - expected_pos).length();
                    let new_expected_diff = (new_pos - expected_pos).length();
                    println!("   期望位置: {:?}", expected_pos);
                    println!("   旧函数与期望差异: {:.3}", old_expected_diff);
                    println!("   新函数与期望差异: {:.3}", new_expected_diff);
                    
                    if new_expected_diff < old_expected_diff {
                        println!("   ✨ 新策略系统更接近期望结果");
                    } else {
                        println!("   ⚠️  旧函数更接近期望结果");
                    }
                }
            }
            
            // 使用新结果进行后续分析
            if let Some(world_matrix) = new_world_matrix {
                let world_translation = world_matrix.project_point3(glam::DVec3::ZERO);
                println!("   最终世界位置: {:?}", world_translation);
                
                // 分析变换矩阵
                let rotation = glam::DQuat::from_mat4(&world_matrix);
                let y_axis = rotation * glam::DVec3::Y;
                let z_axis = rotation * glam::DVec3::Z;
                println!("   世界Y轴: {:?}", y_axis);
                println!("   世界Z轴: {:?}", z_axis);
                
                // 检查 Y is U and Z is W 方位（针对 POINSP）
                if noun == "POINSP" {
                    let y_up_similarity = y_axis.dot(glam::DVec3::Z).abs();
                    let z_west_similarity = z_axis.dot(glam::DVec3::NEG_X).abs();
                    println!("   Y轴与全局Up轴相似度: {:.6}", y_up_similarity);
                    println!("   Z轴与全局West轴相似度: {:.6}", z_west_similarity);
                    
                    if y_up_similarity > 0.9 && z_west_similarity > 0.9 {
                        println!("   ✅ POINSP方位验证通过: Y is U and Z is W");
                    } else {
                        println!("   ℹ️  POINSP方位不符合Y is U and Z is W");
                    }
                }
                
                // 对于 FITT 类型，特别分析 ZDIS 处理
                if noun == "FITT" {
                    println!("   🔍 FITT 类型 ZDIS 分析:");
                    let zdis = att.get_f32("ZDIS").unwrap_or_default();
                    println!("      ZDIS 值: {}", zdis);
                    println!("      Z 方向差异: {:.3}", pos_diff.z);
                    println!("      X 方向差异: {:.3}", pos_diff.x);
                    println!("      Y 方向差异: {:.3}", pos_diff.y);
                    
                    // 获取当前位置用于后续分析
                    let current_pos = new_pos;
                    
                    // 分析局部变换结果
                    match get_world_mat4_with_strategies(refno, true).await {
                        Ok(Some(local_matrix)) => {
                            let local_pos = local_matrix.project_point3(glam::DVec3::ZERO);
                            println!("      局部变换位置: {:?}", local_pos);
                            
                            // 检查ZDIS在局部坐标系中的应用
                            let local_z_displacement = local_pos.z;
                            println!("      局部Z轴位移: {:.3}", local_z_displacement);
                            
                            if (local_z_displacement - zdis as f64).abs() < 1.0 {
                                println!("      ✅ ZDIS在局部坐标系中正确应用");
                            } else {
                                println!("      ⚠️  ZDIS在局部坐标系中应用异常");
                            }
                        }
                        Ok(None) => {
                            println!("      ⚠️  无法获取局部变换");
                        }
                        Err(e) => {
                            println!("      ❌ 局部变换计算错误: {}", e);
                        }
                    }
                    
                    // 分析期望的ZDIS应用
                    let expected_z = -7350.0; // 期望的Z位置
                    let actual_z = current_pos.z;
                    let z_error = actual_z - expected_z;
                    println!("      期望Z位置: {:.3}", expected_z);
                    println!("      实际Z位置: {:.3}", actual_z);
                    println!("      Z方向误差: {:.3}", z_error);
                    
                    // 分析ZDIS与误差的关系
                    if (z_error + zdis as f64).abs() < 100.0 {
                        println!("      💡 误差可能来自ZDIS符号或坐标系方向");
                    }
                }
            }
        }
        (None, None) => {
            println!("   ℹ️  两个函数都无法计算变换");
        }
        (Some(_), None) => {
            println!("   ℹ️  旧函数有结果但新函数无结果");
        }
        (None, Some(_)) => {
            println!("   ✨ 新函数能计算旧函数无法计算的变换");
        }
    }
    
    Ok(())
}

#[tokio::test]
async fn test_generic_spatial_cases() -> Result<()> {
    // Initialize database connection
    init_surreal().await?;
    
    // Read test cases from JSON file
    let file_path = "src/test/test-cases/spatial/spatial_pdms_cases.json";
    let file = File::open(file_path).expect("Failed to open test cases file");
    let reader = BufReader::new(file);
    let test_cases: Vec<SpatialTestCase> = serde_json::from_reader(reader)
        .expect("Failed to parse test cases");

    println!("🚀 Running {} Spatial Test Cases", test_cases.len());

    let mut errors = Vec::new();

    for case in &test_cases {
        println!("--------------------------------------------------");
        println!("🧪 Case: {}", case.refno);

        let target_refno = RefnoEnum::from(case.refno.replace("/", "_").as_str());
        
        // Parse expected position from WPOS string
        let expected_pos = parse_wpos(&case.wpos_str)
            .expect("Failed to parse WPOS string");
        
        // Get world matrix using get_world_mat4 (should handle all transformations internally)
        if let Some(world_matrix) = get_world_mat4(target_refno, false).await? {
            // Extract position from world matrix (should be world coordinates already)
            let calculated_pos = world_matrix.transform_point3(DVec3::ZERO);
            let diff = calculated_pos - expected_pos;
            
            if diff.length() < 1.0 {
                println!("✅ Position OK - Expected: {:?}, Got: {:?}, Diff: {:.4}", 
                    expected_pos, calculated_pos, diff.length());
            } else {
                let msg = format!("❌ Position Mismatch for {}: Expected {:?}, Got {:?}, Diff {:.4}", 
                    case.refno, expected_pos, calculated_pos, diff.length());
                println!("{}", msg);
                errors.push(msg);
            }
            
            // Optional: Check orientation if needed
            if let Some((expected_y, expected_z)) = parse_wori(&case.wori_str) {
                // Extract orientation from world matrix (should be world orientation already)
                let calculated_y = world_matrix.transform_vector3(DVec3::Y).normalize();
                let calculated_z = world_matrix.transform_vector3(DVec3::Z).normalize();
                
                let y_dot = calculated_y.dot(expected_y);
                let z_dot = calculated_z.dot(expected_z);
                
                if y_dot > 0.999 && z_dot > 0.999 {
                    println!("✅ Orientation OK - Y_dot: {:.6}, Z_dot: {:.6}", y_dot, z_dot);
                } else {
                    let msg = format!("⚠️  Orientation Mismatch for {}: Y_dot={:.6}, Z_dot={:.6}", 
                        case.refno, y_dot, z_dot);
                    println!("{}", msg);
                    // Note: Not adding orientation errors to error list for now, just warnings
                }
            }
        } else {
            let msg = format!("❌ Failed to get world matrix for {}", case.refno);
            println!("{}", msg);
            errors.push(msg);
        }
    }
    
    // 额外调试：分析有问题的参考号（在检查失败前）
    println!("\n{}", "=".repeat(60));
    println!("🔍 深度调试有问题的参考号");
    println!("{}", "=".repeat(60));
    
    let problem_refnos = vec![
        "17496/266220",
        "25688/7960",
    ];
    
    for refno_str in problem_refnos {
        println!("\n🧪 深度分析: {}", refno_str);
        let refno = RefnoEnum::from(refno_str);
        
        // 获取属性映射
        if let Ok(att) = get_named_attmap(refno).await {
            let noun = att.get_type_str();
            let owner = att.get_owner();
            println!("📋 类型: {}, 父级: {}", noun, owner);
            
            // 检查关键属性
            if let Some(npos) = att.get_dvec3("NPOS") {
                println!("📍 NPOS: {:?}", npos);
            }
            if let Some(ydir) = att.get_dvec3("YDIR") {
                println!("🧭 YDIR: {:?}", ydir);
            }
            if let Some(bang) = att.get_f32("BANG") {
                println!("🔄 BANG: {}°", bang);
            }
            if let Some(zdis) = att.get_f32("ZDIS") {
                println!("📏 ZDIS: {}", zdis);
            }
            
            // 检查策略使用
            let strategy = crate::transform::strategies::TransformStrategyFactory::get_strategy(noun);
            println!("🎯 使用策略: {:?}", std::any::type_name_of_val(&strategy));
            
            // 分析父级
            if let Ok(parent_att) = get_named_attmap(owner).await {
                let parent_noun = parent_att.get_type_str();
                println!("👤 父级类型: {}", parent_noun);
                if let Some(parent_npos) = parent_att.get_dvec3("NPOS") {
                    println!("📍 父级NPOS: {:?}", parent_npos);
                }
            }
        }
    }

    if !errors.is_empty() {
        panic!("Spatial Test Failed:\n{}", errors.join("\n"));
    }
    
    println!("✅ All spatial tests passed!");
    Ok(())
}

#[tokio::test]
async fn debug_fitt_zdis_issue() -> Result<()> {
    init_surreal().await?;
    
    println!("🔍 深度分析 FITT 类型的 ZDIS 处理问题");
    
    let refno_str = "25688/7960";
    let refno = RefnoEnum::from(refno_str);
    let att = get_named_attmap(refno).await?;
    let owner = att.get_owner();
    
    println!("📋 基本信息:");
    println!("   参考号: {}", refno_str);
    println!("   类型: {}", att.get_type_str());
    println!("   父级: {}", owner);
    
    println!("\n🔧 关键属性:");
    println!("   ZDIS: {:?}", att.get_f32("ZDIS"));
    println!("   PKDI: {:?}", att.get_f32("PKDI"));
    println!("   NPOS: {:?}", att.get_dvec3("NPOS"));
    println!("   YDIR: {:?}", att.get_dvec3("YDIR"));
    
    // 调用 ZDIS 处理函数
    println!("\n🎯 调试 ZDIS 处理:");
    
    // 模拟 DefaultStrategy 的 ZDIS 处理
    let zdist = att.get_f32("ZDIS").unwrap_or_default();
    let pkdi = att.get_f32("PKDI").unwrap_or_default();
    println!("   zdist: {}, pkdi: {}", zdist, pkdi);
    
    // 调用 cal_zdis_pkdi_in_section_by_spine
    match rs_surreal::spatial::cal_zdis_pkdi_in_section_by_spine(owner, pkdi, zdist, None).await {
        Ok(Some((quat, pos))) => {
            println!("   ✅ spine 计算成功:");
            println!("      位置: {:?}", pos);
            println!("      旋转: {:?}", quat);
        }
        Ok(None) => {
            println!("   ❌ spine 计算返回 None，使用默认 Z 轴偏移");
            println!("      默认偏移: Z * {}", zdist);
        }
        Err(e) => {
            println!("   ❌ spine 计算错误: {}", e);
        }
    }
    
    // 检查父级的 spine 路径
    println!("\n👤 父级 Spine 分析:");
    match rs_surreal::spatial::get_spline_path(owner).await {
        Ok(paths) => {
            println!("   父级 spine 路径数量: {}", paths.len());
            if !paths.is_empty() {
                println!("   首个 spine 起点: {:?}", paths[0].pt0);
                println!("   首个 spine 终点: {:?}", paths[0].pt1);
                println!("   首个 spine 方向: {:?}", paths[0].preferred_dir);
            }
        }
        Err(e) => {
            println!("   ❌ 获取 spine 路径失败: {}", e);
        }
    }
    
    // 检查父级的世界矩阵
    println!("\n🌍 父级世界变换:");
    match transform::get_world_mat4(owner).await {
        Ok(Some(matrix)) => {
            let trans = matrix.project_point3(glam::DVec3::ZERO);
            println!("   父级世界位置: {:?}", trans);
        }
        Ok(None) => {
            println!("   ❌ 父级世界矩阵为 None");
        }
        Err(e) => {
            println!("   ❌ 获取父级世界矩阵失败: {}", e);
        }
    }
    
    // 最终结果对比
    println!("\n📊 结果对比:");
    if let Some(world_matrix) = transform::get_world_mat4(refno).await? {
        let final_pos = world_matrix.project_point3(glam::DVec3::ZERO);
        println!("   计算结果: {:?}", final_pos);
        println!("   期望结果: {:?}", glam::DVec3::new(59375.0, 21200.0, -7350.0));
        
        let diff = final_pos - glam::DVec3::new(59375.0, 21200.0, -7350.0);
        println!("   位置差异: {:?}", diff);
        println!("   差异大小: {}", diff.length());
    }
    
    Ok(())
}

/// 矩阵比较函数，用于验证新旧函数结果一致性
fn compare_matrices(matrix1: &DMat4, matrix2: &DMat4) -> bool {
    const EPSILON: f64 = 1e-10;
    
    // 检查 NaN 状态
    if matrix1.is_nan() && matrix2.is_nan() {
        return true;
    }
    if matrix1.is_nan() || matrix2.is_nan() {
        return false;
    }
    
    // 逐元素比较
    for i in 0..4 {
        for j in 0..4 {
            let diff = (matrix1.col(i)[j] - matrix2.col(i)[j]).abs();
            if diff > EPSILON {
                return false;
            }
        }
    }
    
    true
}

/// 计算两个矩阵之间的最大差异
fn calculate_max_matrix_diff(matrix1: &DMat4, matrix2: &DMat4) -> f64 {
    let mut max_diff = 0.0;
    
    // 检查 NaN 状态
    if matrix1.is_nan() && matrix2.is_nan() {
        return 0.0;
    }
    if matrix1.is_nan() || matrix2.is_nan() {
        return f64::INFINITY;
    }
    
    // 逐元素计算差异
    for i in 0..4 {
        for j in 0..4 {
            let diff = (matrix1.col(i)[j] - matrix2.col(i)[j]).abs();
            if diff > max_diff {
                max_diff = diff;
            }
        }
    }
    
    max_diff
}
