use crate::geometry::csg::{generate_scylinder_mesh, orthonormal_basis};
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
    
    // 验证2: 端面落在对应平面上（需要与生成代码使用相同的坐标系转换）
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
    for (v, n) in generated_mesh.vertices.iter().zip(generated_mesh.normals.iter()) {
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

    // 验证3: 侧面法向为径向
    let mut max_side_ang = 0.0f32;
    let mut side_cnt = 0;
    for (v, n) in generated_mesh.vertices.iter().zip(generated_mesh.normals.iter()) {
        // 过滤掉盖子（法向接近 Nb/Nt 的已经统计过），这里取与 dir 夹角接近 90° 的点
        if n.dot(dir).abs() < 0.2 {
            let proj = v - dir * v.dot(dir);
            if proj.length_squared() > 1e-6 {
                let radial = proj.normalize();
                let ang = n.angle_between(radial);
                max_side_ang = max_side_ang.max(ang);
                side_cnt += 1;
            }
        }
    }
    assert!(side_cnt > 0, "side vertices should exist");
    assert!(max_side_ang < 1e-2, "side normals should be radial, max angle {}", max_side_ang);

    // 验证4: AABB 有效
    if let Some(aabb) = generated_mesh.aabb {
        println!("📦 AABB: 最小({:.3}, {:.3}, {:.3}) 到 最大({:.3}, {:.3}, {:.3})",
                 aabb.mins.x, aabb.mins.y, aabb.mins.z,
                 aabb.maxs.x, aabb.maxs.y, aabb.maxs.z);
        assert!(aabb.volume() > 0.0, "❌ AABB应该有效(体积应该>0)");
    }

    // 导出 OBJ 文件
    std::fs::create_dir_all("test_output").ok();
    let obj_path = "test_output/sslc_shear_15_10_5_20.obj";
    match generated_mesh.export_obj(false, obj_path) {
        Ok(_) => println!("✅ OBJ 已导出: {}", obj_path),
        Err(e) => println!("⚠️ OBJ 导出失败: {}", e),
    }

    println!("🎉 所有验证通过！SSLC几何生成符合文档定义");
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
    
    let mesh = result.unwrap().mesh;
    std::fs::create_dir_all("test_output").ok();
    let obj_path = "test_output/sslc_no_shear.obj";
    match mesh.export_obj(false, obj_path) {
        Ok(_) => println!("✅ OBJ 已导出: {}", obj_path),
        Err(e) => println!("⚠️ OBJ 导出失败: {}", e),
    }
    
    println!("✅ 无剪切SSLC生成成功");
}
