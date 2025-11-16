/// 测试斜切圆柱 (SSCL) 的端面剪切角度控制
///
/// 验证四个剪切参数的效果：
/// - btm_shear_angles[0]: 底面 X 方向剪切角
/// - btm_shear_angles[1]: 底面 Y 方向剪切角
/// - top_shear_angles[0]: 顶面 X 方向剪切角
/// - top_shear_angles[1]: 顶面 Y 方向剪切角
use crate::prim_geo::cylinder::SCylinder;
use crate::shape::pdms_shape::BrepShapeTrait;
use glam::Vec3;

#[test]
fn test_scylinder_with_45_degree_shear() {
    println!("\n=== 测试斜切圆柱：45度单向剪切 ===");

    let cylinder = SCylinder {
        paxi_expr: "Z".to_string(),
        paxi_pt: Vec3::ZERO,
        paxi_dir: Vec3::Z,
        phei: 1000.0,                  // 高度 1000mm
        pdia: 200.0,                   // 直径 200mm
        btm_shear_angles: [45.0, 0.0], // 底面 X 方向 45 度
        top_shear_angles: [0.0, 0.0],  // 顶面垂直
        negative: false,
        center_in_mid: false,
    };

    println!("  圆柱参数:");
    println!("    直径: {} mm", cylinder.pdia);
    println!("    高度: {} mm", cylinder.phei);
    println!(
        "    底面剪切角: X={:.0}°, Y={:.0}°",
        cylinder.btm_shear_angles[0], cylinder.btm_shear_angles[1]
    );
    println!(
        "    顶面剪切角: X={:.0}°, Y={:.0}°",
        cylinder.top_shear_angles[0], cylinder.top_shear_angles[1]
    );

    // 验证是否识别为斜切圆柱
    assert!(cylinder.is_sscl(), "应该识别为 SSCL");

    // 生成 CSG mesh
    match cylinder.gen_csg_shape() {
        Ok(csg_mesh) => {
            println!("  ✅ CSG Mesh 生成成功！");
            println!("    顶点数: {}", csg_mesh.vertices.len());
            println!("    法线数: {}", csg_mesh.normals.len());
            println!("    三角形数: {}", csg_mesh.indices.len() / 3);

            // 导出为 OBJ 文件
            if let Err(e) = csg_mesh.export_obj(false, "test_output/scylinder_45deg_x_shear.obj") {
                println!("    ⚠️  OBJ文件导出失败: {}", e);
            } else {
                println!("    ✅ OBJ文件已导出: test_output/scylinder_45deg_x_shear.obj");
            }
        }
        Err(e) => {
            println!("  ❌ CSG Mesh生成失败: {}", e);
            panic!("斜切圆柱mesh生成失败");
        }
    }

    println!("✅ 45度单向剪切测试通过");
}

#[test]
fn test_scylinder_with_opposite_shear() {
    println!("\n=== 测试斜切圆柱：两端相反剪切 ===");

    let cylinder = SCylinder {
        paxi_expr: "Z".to_string(),
        paxi_pt: Vec3::ZERO,
        paxi_dir: Vec3::Z,
        phei: 800.0,
        pdia: 150.0,
        btm_shear_angles: [30.0, 0.0],  // 底面向右倾斜30度
        top_shear_angles: [-30.0, 0.0], // 顶面向左倾斜30度
        negative: false,
        center_in_mid: false,
    };

    println!("  圆柱参数:");
    println!("    直径: {} mm", cylinder.pdia);
    println!("    高度: {} mm", cylinder.phei);
    println!("    底面剪切角: X={:.0}°", cylinder.btm_shear_angles[0]);
    println!("    顶面剪切角: X={:.0}°", cylinder.top_shear_angles[0]);

    assert!(cylinder.is_sscl(), "应该识别为 SSCL");

    match cylinder.gen_csg_shape() {
        Ok(csg_mesh) => {
            println!("  ✅ CSG Mesh 生成成功！");
            println!("    顶点数: {}", csg_mesh.vertices.len());
            println!("    三角形数: {}", csg_mesh.indices.len() / 3);

            if let Err(e) = csg_mesh.export_obj(false, "test_output/scylinder_opposite_shear.obj") {
                println!("    ⚠️  OBJ文件导出失败: {}", e);
            } else {
                println!("    ✅ OBJ文件已导出: test_output/scylinder_opposite_shear.obj");
            }
        }
        Err(e) => {
            println!("  ❌ CSG Mesh生成失败: {}", e);
            panic!("斜切圆柱mesh生成失败");
        }
    }

    println!("✅ 两端相反剪切测试通过");
}

#[test]
fn test_scylinder_with_xy_shear() {
    println!("\n=== 测试斜切圆柱：XY双向剪切 ===");

    let cylinder = SCylinder {
        paxi_expr: "Z".to_string(),
        paxi_pt: Vec3::ZERO,
        paxi_dir: Vec3::Z,
        phei: 1000.0,
        pdia: 200.0,
        btm_shear_angles: [20.0, 20.0],  // 底面 X/Y 各 20 度
        top_shear_angles: [15.0, -15.0], // 顶面 X=15°, Y=-15°
        negative: false,
        center_in_mid: false,
    };

    println!("  圆柱参数:");
    println!("    直径: {} mm", cylinder.pdia);
    println!("    高度: {} mm", cylinder.phei);
    println!(
        "    底面剪切角: X={:.0}°, Y={:.0}°",
        cylinder.btm_shear_angles[0], cylinder.btm_shear_angles[1]
    );
    println!(
        "    顶面剪切角: X={:.0}°, Y={:.0}°",
        cylinder.top_shear_angles[0], cylinder.top_shear_angles[1]
    );

    assert!(cylinder.is_sscl(), "应该识别为 SSCL");

    match cylinder.gen_csg_shape() {
        Ok(csg_mesh) => {
            println!("  ✅ CSG Mesh 生成成功！");
            println!("    顶点数: {}", csg_mesh.vertices.len());
            println!("    三角形数: {}", csg_mesh.indices.len() / 3);

            if let Err(e) = csg_mesh.export_obj(false, "test_output/scylinder_xy_shear.obj") {
                println!("    ⚠️  OBJ文件导出失败: {}", e);
            } else {
                println!("    ✅ OBJ文件已导出: test_output/scylinder_xy_shear.obj");
            }
        }
        Err(e) => {
            println!("  ❌ CSG Mesh生成失败: {}", e);
            panic!("斜切圆柱mesh生成失败");
        }
    }

    println!("✅ XY双向剪切测试通过");
}

#[test]
fn test_scylinder_normal_vs_sheared() {
    println!("\n=== 测试普通圆柱 vs 斜切圆柱对比 ===");

    // 普通圆柱
    let normal_cylinder = SCylinder {
        paxi_expr: "Z".to_string(),
        paxi_pt: Vec3::ZERO,
        paxi_dir: Vec3::Z,
        phei: 500.0,
        pdia: 100.0,
        btm_shear_angles: [0.0, 0.0],
        top_shear_angles: [0.0, 0.0],
        negative: false,
        center_in_mid: false,
    };

    println!("  普通圆柱:");
    assert!(!normal_cylinder.is_sscl(), "应该不是 SSCL");

    if let Ok(csg_mesh) = normal_cylinder.gen_csg_shape() {
        println!("    顶点数: {}", csg_mesh.vertices.len());
        let _ = csg_mesh.export_obj(false, "test_output/scylinder_normal.obj");
    }

    // 斜切圆柱
    let sheared_cylinder = SCylinder {
        phei: 500.0,
        pdia: 100.0,
        btm_shear_angles: [30.0, 0.0],
        top_shear_angles: [30.0, 0.0],
        ..normal_cylinder
    };

    println!("  斜切圆柱:");
    assert!(sheared_cylinder.is_sscl(), "应该是 SSCL");

    if let Ok(csg_mesh) = sheared_cylinder.gen_csg_shape() {
        println!("    顶点数: {}", csg_mesh.vertices.len());
        let _ = csg_mesh.export_obj(false, "test_output/scylinder_sheared.obj");
    }

    println!("✅ 普通圆柱 vs 斜切圆柱对比测试通过");
}

#[test]
fn test_scylinder_extreme_shear() {
    println!("\n=== 测试斜切圆柱：极端剪切角度 ===");

    let cylinder = SCylinder {
        paxi_expr: "Z".to_string(),
        paxi_pt: Vec3::ZERO,
        paxi_dir: Vec3::Z,
        phei: 600.0,
        pdia: 120.0,
        btm_shear_angles: [60.0, 0.0], // 底面 60 度
        top_shear_angles: [0.0, 60.0], // 顶面 Y 方向 60 度
        negative: false,
        center_in_mid: false,
    };

    println!("  圆柱参数:");
    println!("    底面剪切角: X={:.0}°", cylinder.btm_shear_angles[0]);
    println!("    顶面剪切角: Y={:.0}°", cylinder.top_shear_angles[1]);

    assert!(cylinder.is_sscl(), "应该识别为 SSCL");

    match cylinder.gen_csg_shape() {
        Ok(csg_mesh) => {
            println!("  ✅ CSG Mesh 生成成功！");
            println!("    顶点数: {}", csg_mesh.vertices.len());
            println!("    三角形数: {}", csg_mesh.indices.len() / 3);

            if let Err(e) = csg_mesh.export_obj(false, "test_output/scylinder_extreme_shear.obj") {
                println!("    ⚠️  OBJ文件导出失败: {}", e);
            } else {
                println!("    ✅ OBJ文件已导出: test_output/scylinder_extreme_shear.obj");
            }
        }
        Err(e) => {
            println!("  ❌ CSG Mesh生成失败: {}", e);
            panic!("极端剪切角度mesh生成失败");
        }
    }

    println!("✅ 极端剪切角度测试通过");
}
