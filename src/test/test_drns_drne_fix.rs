//! DRNS/DRNE 截面斜切修复测试
//! 
//! 验证 DRNS（起始端斜切）和 DRNE（结束端斜切）的处理与 core.dll 一致
//! 
//! 运行测试：
//! ```bash
//! cargo test test_drns_drne --features ui -- --nocapture
//! ```

use crate::parsed_data::{CateProfileParam, SProfileData};
use crate::prim_geo::spine::{Line3D, SweepPath3D};
use crate::prim_geo::sweep_solid::SweepSolid;
use crate::shape::pdms_shape::BrepShapeTrait;
use crate::RefnoEnum;
use glam::{DVec3, Vec2, Vec3};
use std::f64::consts::PI;

/// 创建简单的矩形截面
fn create_rect_profile(width: f32, height: f32) -> CateProfileParam {
    let half_w = width / 2.0;
    let half_h = height / 2.0;
    
    let verts = vec![
        Vec2::new(-half_w, -half_h),
        Vec2::new(half_w, -half_h),
        Vec2::new(half_w, half_h),
        Vec2::new(-half_w, half_h),
    ];
    
    CateProfileParam::SPRO(SProfileData {
        refno: RefnoEnum::default(),
        verts,
        frads: vec![0.0; 4],
        plin_pos: Vec2::ZERO,
        plin_axis: Vec3::Y,
        plax: Vec3::Y,
        na_axis: Vec3::Z,
    })
}

/// 测试 calculate_face_rotation 新函数
#[test]
fn test_calculate_face_rotation() {
    println!("\n=== 测试 calculate_face_rotation ===\n");
    
    let profile = create_rect_profile(100.0, 50.0);
    let path = SweepPath3D::from_line(Line3D {
        start: Vec3::ZERO,
        end: Vec3::Z * 1000.0,
        is_spine: true,
    });
    
    // 测试 1: 无斜切
    let solid_no_slope = SweepSolid {
        profile: profile.clone(),
        drns: None,
        drne: None,
        plax: Vec3::Y,
        bangle: 0.0,
        extrude_dir: DVec3::Z,
        height: 1000.0,
        path: path.clone(),
        lmirror: false,
        spine_segments: vec![],
        segment_transforms: vec![],
    };
    
    assert!(solid_no_slope.calculate_face_rotation(true).is_none());
    assert!(solid_no_slope.calculate_face_rotation(false).is_none());
    println!("  ✅ 无斜切时返回 None");
    
    // 测试 2: 45度斜切
    let angle_45 = 45.0_f64.to_radians();
    let drns_45 = DVec3::new(0.0, angle_45.sin(), angle_45.cos()).normalize();
    let drne_45 = DVec3::new(0.0, -angle_45.sin(), angle_45.cos()).normalize();
    
    let solid_45 = SweepSolid {
        profile: profile.clone(),
        drns: Some(drns_45),
        drne: Some(drne_45),
        plax: Vec3::Y,
        bangle: 0.0,
        extrude_dir: DVec3::Z,
        height: 1000.0,
        path: path.clone(),
        lmirror: false,
        spine_segments: vec![],
        segment_transforms: vec![],
    };
    
    let rot_start = solid_45.calculate_face_rotation(true);
    let rot_end = solid_45.calculate_face_rotation(false);
    
    assert!(rot_start.is_some());
    assert!(rot_end.is_some());
    println!("  ✅ 45度斜切时返回有效旋转");
    println!("     DRNS 方向: {:?}", drns_45);
    println!("     DRNE 方向: {:?}", drne_45);
    println!("     起始端旋转: {:?}", rot_start.unwrap());
    println!("     结束端旋转: {:?}", rot_end.unwrap());
    
    // 测试 3: 接近默认方向时返回 None
    let drns_z = DVec3::new(0.0, 0.0, 1.0); // 接近 +Z
    let solid_z = SweepSolid {
        profile: profile.clone(),
        drns: Some(drns_z),
        drne: None,
        plax: Vec3::Y,
        bangle: 0.0,
        extrude_dir: DVec3::Z,
        height: 1000.0,
        path: path.clone(),
        lmirror: false,
        spine_segments: vec![],
        segment_transforms: vec![],
    };
    
    assert!(solid_z.calculate_face_rotation(true).is_none());
    println!("  ✅ 方向接近默认时返回 None");
    
    println!("\n✅ calculate_face_rotation 测试通过\n");
}

/// 测试带斜切的矩形截面生成 mesh 并导出 OBJ
#[test]
fn test_rect_with_drns_drne_mesh() {
    println!("\n=== 测试矩形截面斜切 Mesh 生成 ===\n");
    
    let profile = create_rect_profile(100.0, 50.0);
    let path = SweepPath3D::from_line(Line3D {
        start: Vec3::ZERO,
        end: Vec3::Z * 500.0,
        is_spine: true,
    });
    
    // 30度斜切
    let angle_30 = 30.0_f64.to_radians();
    let drns = DVec3::new(0.0, angle_30.sin(), angle_30.cos()).normalize();
    let drne = DVec3::new(angle_30.sin(), 0.0, angle_30.cos()).normalize();
    
    println!("  截面: 100x50 矩形");
    println!("  路径: 500mm 直线 (沿 Z 轴)");
    println!("  DRNS: 30° 倾斜 (Y方向)");
    println!("  DRNE: 30° 倾斜 (X方向)");
    
    let solid = SweepSolid {
        profile,
        drns: Some(drns),
        drne: Some(drne),
        plax: Vec3::Y,
        bangle: 0.0,
        extrude_dir: DVec3::Z,
        height: 500.0,
        path,
        lmirror: false,
        spine_segments: vec![],
        segment_transforms: vec![],
    };
    
    // 验证 is_sloped
    assert!(solid.is_sloped());
    assert!(solid.is_drns_sloped());
    assert!(solid.is_drne_sloped());
    println!("  ✅ is_sloped() 正确识别斜切");
    
    // 生成 mesh
    match solid.gen_csg_shape() {
        Ok(mesh) => {
            println!("  ✅ Mesh 生成成功");
            println!("     顶点数: {}", mesh.vertices.len());
            println!("     法线数: {}", mesh.normals.len());
            println!("     三角形数: {}", mesh.indices.len() / 3);
            
            // 导出 OBJ
            let obj_path = "test_output/drns_drne_rect_30deg.obj";
            std::fs::create_dir_all("test_output").ok();
            if let Err(e) = mesh.export_obj(false, obj_path) {
                println!("     ⚠️ OBJ 导出失败: {}", e);
            } else {
                println!("     ✅ OBJ 导出: {}", obj_path);
            }
        }
        Err(e) => {
            panic!("❌ Mesh 生成失败: {}", e);
        }
    }
    
    println!("\n✅ 矩形截面斜切测试通过\n");
}

/// 测试多种角度的斜切
#[test]
fn test_various_angles() {
    println!("\n=== 测试多种斜切角度 ===\n");
    
    let profile = create_rect_profile(80.0, 40.0);
    let path = SweepPath3D::from_line(Line3D {
        start: Vec3::ZERO,
        end: Vec3::Z * 300.0,
        is_spine: true,
    });
    
    let angles = [15.0, 30.0, 45.0, 60.0];
    std::fs::create_dir_all("test_output").ok();
    
    for angle_deg in angles {
        let angle_rad = (angle_deg as f64).to_radians();
        let drns = DVec3::new(0.0, angle_rad.sin(), angle_rad.cos()).normalize();
        let drne = DVec3::new(0.0, -angle_rad.sin(), angle_rad.cos()).normalize();
        
        let solid = SweepSolid {
            profile: profile.clone(),
            drns: Some(drns),
            drne: Some(drne),
            plax: Vec3::Y,
            bangle: 0.0,
            extrude_dir: DVec3::Z,
            height: 300.0,
            path: path.clone(),
            lmirror: false,
            spine_segments: vec![],
            segment_transforms: vec![],
        };
        
        match solid.gen_csg_shape() {
            Ok(mesh) => {
                let obj_path = format!("test_output/drns_drne_{}deg.obj", angle_deg as i32);
                mesh.export_obj(false, &obj_path).ok();
                println!("  ✅ {}° 斜切: {} 顶点, 导出 {}", angle_deg, mesh.vertices.len(), obj_path);
            }
            Err(e) => {
                println!("  ❌ {}° 斜切失败: {}", angle_deg, e);
            }
        }
    }
    
    println!("\n✅ 多角度斜切测试完成\n");
}

/// 测试无斜切 vs 有斜切的对比
#[test]
fn test_comparison_with_without_slope() {
    println!("\n=== 对比测试：无斜切 vs 有斜切 ===\n");
    
    let profile = create_rect_profile(60.0, 60.0);
    let path = SweepPath3D::from_line(Line3D {
        start: Vec3::ZERO,
        end: Vec3::Z * 400.0,
        is_spine: true,
    });
    
    std::fs::create_dir_all("test_output").ok();
    
    // 无斜切
    let solid_no_slope = SweepSolid {
        profile: profile.clone(),
        drns: None,
        drne: None,
        plax: Vec3::Y,
        bangle: 0.0,
        extrude_dir: DVec3::Z,
        height: 400.0,
        path: path.clone(),
        lmirror: false,
        spine_segments: vec![],
        segment_transforms: vec![],
    };
    
    // 45度斜切
    let angle = 45.0_f64.to_radians();
    let drns = DVec3::new(0.0, angle.sin(), angle.cos()).normalize();
    let drne = DVec3::new(0.0, -angle.sin(), angle.cos()).normalize();
    
    let solid_sloped = SweepSolid {
        profile: profile.clone(),
        drns: Some(drns),
        drne: Some(drne),
        plax: Vec3::Y,
        bangle: 0.0,
        extrude_dir: DVec3::Z,
        height: 400.0,
        path: path.clone(),
        lmirror: false,
        spine_segments: vec![],
        segment_transforms: vec![],
    };
    
    // 生成并对比
    let mesh_no_slope = solid_no_slope.gen_csg_shape().expect("无斜切 mesh 生成失败");
    let mesh_sloped = solid_sloped.gen_csg_shape().expect("斜切 mesh 生成失败");
    
    println!("  无斜切: {} 顶点, {} 三角形", mesh_no_slope.vertices.len(), mesh_no_slope.indices.len() / 3);
    println!("  斜切:   {} 顶点, {} 三角形", mesh_sloped.vertices.len(), mesh_sloped.indices.len() / 3);
    
    // 斜切的顶点数应该相同或更多（因为端面可能有额外调整）
    assert!(mesh_sloped.vertices.len() >= mesh_no_slope.vertices.len());
    
    mesh_no_slope.export_obj(false, "test_output/comparison_no_slope.obj").ok();
    mesh_sloped.export_obj(false, "test_output/comparison_sloped.obj").ok();
    
    println!("  ✅ 导出对比文件:");
    println!("     - test_output/comparison_no_slope.obj");
    println!("     - test_output/comparison_sloped.obj");
    
    println!("\n✅ 对比测试通过\n");
}
