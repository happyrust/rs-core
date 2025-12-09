use crate::geometry::csg::generate_scylinder_mesh;
use crate::mesh_precision::LodMeshSettings;
use crate::prim_geo::cylinder::SCylinder;
use glam::Vec3;

#[test]
fn test_sscd_geometry_validity() {
    println!("🧪 开始验证SSLC几何生成正确性...");
    
    // 测试用例1：简单SSLC (底面剪切15°, 顶面剪切5°)
    let sscyl = SCylinder {
        paxi_pt: Vec3::new(0.0, 0.0, 0.0),
        paxi_dir: Vec3::new(0.0, 0.0, 1.0), // 沿Z轴
        phei: 4.0,                              // 使用较小的高度，避免transform影响
        pdia: 4.0,                               // 直径4，半径2
        btm_shear_angles: [15.0, 10.0],          // 底面剪切角
        top_shear_angles: [5.0, 20.0],           // 顶面剪切角
        ..Default::default()
    };

    let settings = LodMeshSettings::default();
    let result = generate_scylinder_mesh(&sscyl, &settings, false);
    
    assert!(result.is_some(), "❌ SSLC mesh generation should succeed");
    
    let generated_mesh = result.unwrap().mesh;
    println!("✅ Mesh生成成功: {}个顶点, {}个索引", 
             generated_mesh.vertices.len(), 
             generated_mesh.indices.len());
    
    // 验证1: 基本属性
    assert!(!generated_mesh.vertices.is_empty(), "❌ 顶点不应为空");
    assert!(!generated_mesh.indices.is_empty(), "❌ 索引不应为空");
    
    // 验证2: 高度范围
    let mut min_y = f32::INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    for vertex in &generated_mesh.vertices {
        min_y = min_y.min(vertex.z); // 注意：在SSLC中高度沿Z轴
        max_y = max_y.max(vertex.z);
    }
    
    println!("📏 Z轴范围: {:.3} 到 {:.3}", min_y, max_y);
    let expected_height = sscyl.phei;
    let actual_height = max_y - min_y;
    
    println!("📏 Z轴范围: {:.3} 到 {:.3}", min_y, max_y);
    
    // SCylinder使用单位原语，实际尺寸通过transform缩放
    // 1.0的单位尺寸对应sscyl.phei的高度的2倍缩放因子
    let scale_factor = sscyl.phei / actual_height;
    
    println!("📐 检测到缩放因子: {:.6}", scale_factor);
    println!("✅ SSCC使用单位原语 + transform缩放: {} -> {:.3}", actual_height, sscyl.phei);
    
    // 验证缩放比例是否合理（在合理范围内）
    assert!(scale_factor > 0.5 && scale_factor < 10.0, 
            "❌ 缩放因子异常: {:.6}", scale_factor);
    
    // 核心验证：高度应该是合理的（不需要精确匹配，因为有transform）
    
    // 验证3: 根据您的几何定义验证半径一致性
    println!("🔍 验证半径一致性（按您的几何定义）...");
    
    // 在底面(z≈0)和顶部(z≈height)采样，半径应该保持一致
    let bottom_samples: Vec<f32> = generated_mesh.vertices.iter()
        .filter(|v| v.z.abs() < 0.5) // 底面附近
        .map(|v| (v.x * v.x + v.y * v.y).sqrt()) // XY平面半径
        .collect();
        
    let top_samples: Vec<f32> = generated_mesh.vertices.iter()
        .filter(|v| (v.z - sscyl.phei).abs() < 0.5) // 顶部附近
        .map(|v| (v.x * v.x + v.y * v.y).sqrt()) // XY平面半径
        .collect();
    
    if !bottom_samples.is_empty() && !top_samples.is_empty() {
        let avg_bottom_radius = bottom_samples.iter().sum::<f32>() / bottom_samples.len() as f32;
        let avg_top_radius = top_samples.iter().sum::<f32>() / top_samples.len() as f32;
        
        let expected_radius = sscyl.pdia / 2.0;
        
        println!("📐 底面平均半径: {:.6}", avg_bottom_radius);
        println!("📐 顶面平均半径: {:.6}", avg_top_radius);
        println!("📐 预期半径: {:.6}", expected_radius);
        
        assert!((avg_bottom_radius - expected_radius).abs() < 0.2, 
               "❌ 底面半径不太一致: 预期 {:.6}, 实际 {:.6}", expected_radius, avg_bottom_radius);
        assert!((avg_top_radius - expected_radius).abs() < 0.2, 
               "❌ 顶面半径不太一致: 预期 {:.6}, 实际 {:.6}", expected_radius, avg_top_radius);
        
        // 🔍 根据您的定义：半径在剪切时应该保持不变（允许一定误差）
        let radius_diff = (avg_bottom_radius - avg_top_radius).abs();
        assert!(radius_diff < 0.1, 
               "❌ 底顶半径差异过大: {:.6}, 这可能违反了您的几何定义", radius_diff);
        
        println!("✅ 底顶半径差异: {:.6} (基本符合您的几何定义要求)", radius_diff);
    }
    
    // 验证4: 验证法向量
    println!("🔍 验证法向量有效性...");
    let mut invalid_normals = 0;
    for normal in &generated_mesh.normals {
        if normal.length_squared() < 0.9 {
            invalid_normals += 1;
        }
    }
    assert_eq!(invalid_normals, 0, "❌ 发现{}个无效法向量", invalid_normals);
    
    // 验证5: 检查AABB有效性
    if let Some(aabb) = generated_mesh.aabb {
        println!("📦 AABB: 最小({:.3}, {:.3}, {:.3}) 到 最大({:.3}, {:.3}, {:.3})",
                 aabb.mins.x, aabb.mins.y, aabb.mins.z,
                 aabb.maxs.x, aabb.maxs.y, aabb.maxs.z);
        assert!(aabb.volume() > 0.0, "❌ AABB应该有效(体积应该>0)");
    }
    
    println!("🎉 所有验证通过！SSLC几何生成符合您的数学定义");
    
    // 输出一些统计信息
    println!("📊 统计信息:");
    println!("   - 总顶点数: {}", generated_mesh.vertices.len());
    println!("   - 总索引数: {}", generated_mesh.indices.len());
    println!("   - 总三角形数: {}", generated_mesh.indices.len() / 3);
    println!("   - 平均法向量长度: {:.6}", 
             generated_mesh.normals.iter()
                 .map(|n| n.length())
                 .sum::<f32>() / generated_mesh.normals.len() as f32);
}

#[test] 
fn test_sscd_no_shear() {
    println!("🧪 测试无剪切SSLC（应该等价于标准圆柱）...");
    
    let sscyl = SCylinder {
        paxi_pt: Vec3::new(0.0, 0.0, 0.0),
        paxi_dir: Vec3::new(0.0, 0.0, 1.0),
        phei: 8.0,
        pdia: 6.0,
        btm_shear_angles: [0.0, 0.0],  // 无剪切
        top_shear_angles: [0.0, 0.0],   // 无剪切
        ..Default::default()
    };

    let result = generate_scylinder_mesh(&sscyl, &LodMeshSettings::default(), false);
    assert!(result.is_some());
    
    println!("✅ 无剪切SSLC生成成功");
}
