use crate::geometry::csg::{generate_scylinder_mesh, orthonormal_basis};
use crate::mesh_precision::LodMeshSettings;
use crate::prim_geo::cylinder::SCylinder;
use glam::Vec3;

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

    // 验证端面落在对应倾斜平面上（需要与生成代码使用相同的坐标系转换）
    let dir = sscyl.paxi_dir.normalize();
    let (basis_u, basis_v) = orthonormal_basis(dir);
    let btm_x = sscyl.btm_shear_angles[0].to_radians();
    let btm_y = sscyl.btm_shear_angles[1].to_radians();
    let top_x = sscyl.top_shear_angles[0].to_radians();
    let top_y = sscyl.top_shear_angles[1].to_radians();
    let nb_local = Vec3::new(btm_x.sin(), btm_y.sin(), btm_x.cos() * btm_y.cos()).normalize();
    let nt_local = Vec3::new(top_x.sin(), top_y.sin(), top_x.cos() * top_y.cos()).normalize();
    let nb = (basis_u * nb_local.x + basis_v * nb_local.y + dir * nb_local.z).normalize();
    let nt = (basis_u * nt_local.x + basis_v * nt_local.y + dir * nt_local.z).normalize();
    let bottom_center = sscyl.paxi_pt;
    let top_center = bottom_center + dir * sscyl.phei;

    let mut max_bottom_err = 0.0f32;
    let mut max_top_err = 0.0f32;
    let mut bottom_cnt = 0;
    let mut top_cnt = 0;
    for (v, n) in mesh.vertices.iter().zip(mesh.normals.iter()) {
        if n.dot(nb) > 0.99 {
            max_bottom_err = max_bottom_err.max(((*v - bottom_center).dot(nb)).abs());
            bottom_cnt += 1;
        } else if n.dot(nt) > 0.99 {
            max_top_err = max_top_err.max(((*v - top_center).dot(nt)).abs());
            top_cnt += 1;
        }
    }

    assert!(bottom_cnt > 0 && top_cnt > 0, "cap vertices should exist");
    assert!(max_bottom_err < 1e-3, "bottom cap vertices should lie on plane, max err {}", max_bottom_err);
    assert!(max_top_err < 1e-3, "top cap vertices should lie on plane, max err {}", max_top_err);

    // 导出 OBJ 文件
    std::fs::create_dir_all("test_output").ok();
    let obj_path = "test_output/sslc_correctness_test.obj";
    match mesh.export_obj(false, obj_path) {
        Ok(_) => println!("✅ OBJ 已导出: {}", obj_path),
        Err(e) => println!("⚠️ OBJ 导出失败: {}", e),
    }

    println!("✅ SSLC verification test passed");
}
