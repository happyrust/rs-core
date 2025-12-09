use crate::geometry::csg::generate_scylinder_mesh;
use crate::mesh_precision::LodMeshSettings;
use crate::prim_geo::cylinder::SCylinder;
use glam::Vec3;

#[test]
fn test_sscd_diagnose_height() {
    println!("🔍 诊断SSLC高度计算...");
    
    let sscyl = SCylinder {
        paxi_pt: Vec3::new(0.0, 0.0, 0.0),
        paxi_dir: Vec3::new(0.0, 0.0, 1.0),
        phei: 2.0,
        pdia: 4.0,
        btm_shear_angles: [0.0, 0.0],  // 无剪切，简化测试
        top_shear_angles: [0.0, 0.0],
        ..Default::default()
    };

    let result = generate_scylinder_mesh(&sscyl, &LodMeshSettings::default(), false);
    
    if let Some(generated) = result {
        let mesh = generated.mesh;
        println!("📊 顶点数量: {}", mesh.vertices.len());
        
        // 检查所有顶点的坐标范围
        let mut min_x = f32::INFINITY; let mut max_x = f32::NEG_INFINITY;
        let mut min_y = f32::INFINITY; let mut max_y = f32::NEG_INFINITY;
        let mut min_z = f32::INFINITY; let mut max_z = f32::NEG_INFINITY;
        
        for (i, vertex) in mesh.vertices.iter().enumerate() {
            min_x = min_x.min(vertex.x); max_x = max_x.max(vertex.x);
            min_y = min_y.min(vertex.y); max_y = max_y.max(vertex.y);
            min_z = min_z.min(vertex.z); max_z = max_z.max(vertex.z);
            
            if i < 10 {
                println!("顶点[{}]: ({:.3}, {:.3}, {:.3})", i, vertex.x, vertex.y, vertex.z);
            }
        }
        
        println!("📏 坐标范围:");
        println!("  X: {:.3} 到 {:.3}", min_x, max_x);
        println!("  Y: {:.3} 到 {:.3}", min_y, max_y);
        println!("  Z: {:.3} 到 {:.3}", min_z, max_z);
        
        let size_x = max_x - min_x;
        let size_y = max_y - min_y;
        let size_z = max_z - min_z;
        
        println!("📏 尺寸范围:");
        println!("  X: {:.3}", size_x);
        println!("  Y: {:.3}", size_y);
        println!("  Z: {:.3}", size_z);
        
        println!("🎯 预期参数:");
        println!("  高度: {:.3}", sscyl.phei);
        println!("  直径: {:.3}", sscyl.pdia);
        println!("  半径: {:.3}", sscyl.pdia / 2.0);
        
        // 检查哪个维度对应高度
        let height_mismatch_x = (size_x - sscyl.phei).abs();
        let height_mismatch_y = (size_y - sscyl.phei).abs();
        let height_mismatch_z = (size_z - sscyl.phei).abs();
        
        println!("🔍 高度匹配分析:");
        println!("  vs X轴差异: {:.6}", height_mismatch_x);
        println!("  vs Y轴差异: {:.6}", height_mismatch_y);
        println!("  vs Z轴差异: {:.6}", height_mismatch_z);
        
        if height_mismatch_z < 0.1 {
            println!("✅ 高度沿Z轴 - 符合预期");
        } else if height_mismatch_y < 0.1 {
            println!("✅ 高度沿Y轴 - 需要调整测试");
        } else if height_mismatch_x < 0.1 {
            println!("✅ 高度沿X轴 - 需要调整测试");
        } else {
            println!("❌ 无法确定高度方向");
        }
    }
}
