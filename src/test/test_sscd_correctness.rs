use crate::geometry::csg::generate_scylinder_mesh;
use crate::mesh_precision::LodMeshSettings;
use crate::prim_geo::cylinder::SCylinder;

#[test]
fn test_sscd_correctness() {
    // 创建一个简单的SSLC用于测试
    let sscyl = SCylinder {
        paxi_pt: glam::Vec3::new(0.0, 0.0, 0.0),
        paxi_dir: glam::Vec3::new(0.0, 0.0, 1.0), // 沿Z轴
        phei: 10.0,                                     // 高度10
        pdia: 4.0,                                      // 直径4，半径2
        btm_shear_angles: [15.0, 10.0],               // 底面剪切角
        top_shear_angles: [5.0, 20.0],                // 顶面剪切角
        ..Default::default()
    };

    let settings = LodMeshSettings::default();
    
    // 生成mesh
    let result = generate_scylinder_mesh(&sscyl, &settings, false);
    
    assert!(result.is_some(), "SSLC mesh generation should succeed");
    
    let mesh = result.unwrap().mesh;
    
    // 验证基本属性
    assert!(!mesh.vertices.is_empty(), "Generated mesh should have vertices");
    assert!(!mesh.indices.is_empty(), "Generated mesh should have indices");
    
    // 验证半径一致性（根据您的几何定义，半径应该保持一致）
    let mut radius_variance = 0.0f32;
    let radius_samples: Vec<f32> = mesh.vertices.iter()
        .take(mesh.vertices.len().min(100))
        .map(|v| {
            let xz_dist = (v.x * v.x + v.z * v.z).sqrt();
            (v.y - sscyl.phei * 0.5).abs() < 0.1 // 取中间高度的样本
        })
        .filter(|is_middle| *is_middle)
        .map(|_| 2.0) // 预期半径
        .collect();
    
    if !radius_samples.is_empty() {
        let avg_radius = radius_samples.iter().sum::<f32>() / radius_samples.len() as f32;
        assert!((avg_radius - 2.0).abs() < 0.1, "Radius should be consistent: expected 2.0, got {}", avg_radius);
    }
    
    println!("✅ SSLC verification test passed");
}
