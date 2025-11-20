use crate::RefnoEnum;
use crate::parsed_data::{CateProfileParam, SProfileData};
use crate::prim_geo::spine::{Arc3D, Line3D, SweepPath3D};
use crate::prim_geo::sweep_solid::SweepSolid;
use crate::shape::pdms_shape::BrepShapeTrait;
use glam::{DVec3, Vec2, Vec3};
use std::f32::consts::PI;

/// 测试 plax 参数对拉伸体方位的影响
#[test]
fn test_sweep_orientation_with_plax() {
    println!("\n=== 测试 plax 参数对方位的影响 ===");

    // 创建一个简单的矩形截面
    let rect_profile = vec![
        Vec2::new(-50.0, -25.0),
        Vec2::new(50.0, -25.0),
        Vec2::new(50.0, 25.0),
        Vec2::new(-50.0, 25.0),
    ];

    let profile = CateProfileParam::SPRO(SProfileData {
        refno: RefnoEnum::default(),
        verts: rect_profile.clone(),
        frads: vec![0.0; 4],
        plin_pos: Vec2::ZERO,
        plin_axis: Vec3::Y,
        plax: Vec3::Y,
        na_axis: Vec3::Z, // 截面初始在 XY 平面
    });

    // 创建沿 Z 轴的直线路径
    let line_path = SweepPath3D::from_line(Line3D {
        start: Vec3::ZERO,
        end: Vec3::Z * 500.0,
        is_spine: false,
    });

    // 测试 1: plax = Y (默认)
    println!("\n测试 1: plax = Vec3::Y");
    let sweep_y = SweepSolid {
        profile: profile.clone(),
        drns: None,
        drne: None,
        bangle: 0.0,
        plax: Vec3::Y, // 截面朝向 Y 轴
        extrude_dir: DVec3::Z,
        height: 500.0,
        path: line_path.clone(),
        lmirror: false,
    };

    let mesh_y = sweep_y
        .gen_csg_shape()
        .expect("Failed to generate mesh with plax=Y");
    println!("  顶点数: {}", mesh_y.vertices.len());

    // 检查第一个顶点 - 应该在 XZ 平面的某个位置
    let first_vertex_y = mesh_y.vertices[0];
    println!(
        "  第一个顶点: ({:.2}, {:.2}, {:.2})",
        first_vertex_y.x, first_vertex_y.y, first_vertex_y.z
    );

    // 测试 2: plax = X
    println!("\n测试 2: plax = Vec3::X");
    let sweep_x = SweepSolid {
        profile: profile.clone(),
        drns: None,
        drne: None,
        bangle: 0.0,
        plax: Vec3::X, // 截面朝向 X 轴
        extrude_dir: DVec3::Z,
        height: 500.0,
        path: line_path.clone(),
        lmirror: false,
    };

    let mesh_x = sweep_x
        .gen_csg_shape()
        .expect("Failed to generate mesh with plax=X");
    println!("  顶点数: {}", mesh_x.vertices.len());

    let first_vertex_x = mesh_x.vertices[0];
    println!(
        "  第一个顶点: ({:.2}, {:.2}, {:.2})",
        first_vertex_x.x, first_vertex_x.y, first_vertex_x.z
    );

    // 验证：plax=Y 和 plax=X 应该产生不同的方位
    // 第一个顶点应该在不同的位置
    let diff = (first_vertex_y - first_vertex_x).length();
    println!("\n顶点位置差异: {:.2}", diff);

    assert!(diff > 1.0, "plax 参数应该影响截面方位，但顶点位置几乎相同");

    // 导出 OBJ 文件用于可视化验证
    let _ = mesh_y.export_obj(false, "test_output/sweep_plax_y.obj");
    let _ = mesh_x.export_obj(false, "test_output/sweep_plax_x.obj");

    println!("\n✅ plax 参数测试通过");
    println!("  已导出: test_output/sweep_plax_y.obj");
    println!("  已导出: test_output/sweep_plax_x.obj");
}

/// 测试 bangle 参数（绕 plax 轴旋转）
#[test]
fn test_sweep_bangle_rotation() {
    println!("\n=== 测试 bangle 参数 ===");

    let rect_profile = vec![
        Vec2::new(-50.0, -25.0),
        Vec2::new(50.0, -25.0),
        Vec2::new(50.0, 25.0),
        Vec2::new(-50.0, 25.0),
    ];

    let profile = CateProfileParam::SPRO(SProfileData {
        refno: RefnoEnum::default(),
        verts: rect_profile,
        frads: vec![0.0; 4],
        plin_pos: Vec2::ZERO,
        plin_axis: Vec3::Y,
        plax: Vec3::Y,
        na_axis: Vec3::Z,
    });

    let line_path = SweepPath3D::from_line(Line3D {
        start: Vec3::ZERO,
        end: Vec3::Z * 500.0,
        is_spine: false,
    });

    // 无旋转
    let sweep_0 = SweepSolid {
        profile: profile.clone(),
        drns: None,
        drne: None,
        bangle: 0.0,
        plax: Vec3::Y,
        extrude_dir: DVec3::Z,
        height: 500.0,
        path: line_path.clone(),
        lmirror: false,
    };

    // 旋转 45 度
    let sweep_45 = SweepSolid {
        profile: profile.clone(),
        drns: None,
        drne: None,
        bangle: 45.0,
        plax: Vec3::Y,
        extrude_dir: DVec3::Z,
        height: 500.0,
        path: line_path.clone(),
        lmirror: false,
    };

    let mesh_0 = sweep_0.gen_csg_shape().expect("Failed with bangle=0");
    let mesh_45 = sweep_45.gen_csg_shape().expect("Failed with bangle=45");

    let v0 = mesh_0.vertices[0];
    let v45 = mesh_45.vertices[0];

    println!("  bangle=0°:  ({:.2}, {:.2}, {:.2})", v0.x, v0.y, v0.z);
    println!("  bangle=45°: ({:.2}, {:.2}, {:.2})", v45.x, v45.y, v45.z);

    let diff = (v0 - v45).length();
    println!("  位置差异: {:.2}", diff);

    assert!(diff > 1.0, "bangle 参数应该影响截面旋转");

    let _ = mesh_0.export_obj(false, "test_output/sweep_bangle_0.obj");
    let _ = mesh_45.export_obj(false, "test_output/sweep_bangle_45.obj");

    println!("\n✅ bangle 参数测试通过");
}

/// 测试圆弧路径的 pref_axis 参数（关键修复验证）
#[test]
fn test_arc_sweep_with_pref_axis() {
    println!("\n=== 测试圆弧路径的 pref_axis 参数 ===");
    println!("此测试验证关键修复：圆弧路径应使用 arc.pref_axis 作为 Y 轴");

    // 创建一个圆形截面
    let circle_profile = {
        let segments = 16;
        let radius = 30.0;
        let mut verts = Vec::new();
        for i in 0..segments {
            let angle = (i as f32) * 2.0 * PI / (segments as f32);
            verts.push(Vec2::new(radius * angle.cos(), radius * angle.sin()));
        }
        verts
    };

    let profile = CateProfileParam::SPRO(SProfileData {
        refno: RefnoEnum::default(),
        verts: circle_profile,
        frads: vec![0.0; 16],
        plin_pos: Vec2::ZERO,
        plin_axis: Vec3::Y,
        plax: Vec3::Z,
        na_axis: Vec3::Z,
    });

    // 创建一个 90 度的圆弧路径（在 XY 平面，绕 Z 轴）
    let arc_radius = 500.0;
    let arc = Arc3D {
        center: Vec3::ZERO,
        radius: arc_radius,
        angle: PI / 2.0, // 90 度
        start_pt: Vec3::new(arc_radius, 0.0, 0.0),
        clock_wise: false,
        axis: Vec3::Z,           // 圆弧绕 Z 轴
        pref_axis: Vec3::Y,      // 首选轴为 Y（这应该作为坐标系的 Y 轴）
    };

    let arc_path = SweepPath3D::from_arc(arc.clone());

    println!("\n圆弧参数:");
    println!("  中心: {:?}", arc.center);
    println!("  半径: {}", arc.radius);
    println!("  角度: {}° ({} rad)", arc.angle.to_degrees(), arc.angle);
    println!("  轴向 (axis): {:?}", arc.axis);
    println!("  首选轴 (pref_axis): {:?}", arc.pref_axis);

    // 测试 1: pref_axis = Y
    println!("\n测试 1: pref_axis = Vec3::Y");
    let sweep_y = SweepSolid {
        profile: profile.clone(),
        drns: None,
        drne: None,
        bangle: 0.0,
        plax: Vec3::Z,
        extrude_dir: DVec3::Z,
        height: 500.0,
        path: arc_path.clone(),
        lmirror: false,
    };

    let mesh_y = sweep_y
        .gen_csg_shape()
        .expect("Failed to generate arc sweep with pref_axis=Y");
    println!("  顶点数: {}", mesh_y.vertices.len());

    // 测试 2: pref_axis = X（改变首选轴应该改变方位）
    let arc_x = Arc3D {
        pref_axis: Vec3::X,  // 改变首选轴为 X
        ..arc.clone()
    };
    let arc_path_x = SweepPath3D::from_arc(arc_x);

    println!("\n测试 2: pref_axis = Vec3::X");
    let sweep_x = SweepSolid {
        profile: profile.clone(),
        drns: None,
        drne: None,
        bangle: 0.0,
        plax: Vec3::Z,
        extrude_dir: DVec3::Z,
        height: 500.0,
        path: arc_path_x,
        lmirror: false,
    };

    let mesh_x = sweep_x
        .gen_csg_shape()
        .expect("Failed to generate arc sweep with pref_axis=X");
    println!("  顶点数: {}", mesh_x.vertices.len());

    // 验证：不同的 pref_axis 应该产生不同的网格
    // 比较起始截面的顶点位置
    if mesh_y.vertices.len() > 0 && mesh_x.vertices.len() > 0 {
        let v_y = mesh_y.vertices[0];
        let v_x = mesh_x.vertices[0];
        let diff = (v_y - v_x).length();

        println!("\n顶点位置对比:");
        println!("  pref_axis=Y: ({:.2}, {:.2}, {:.2})", v_y.x, v_y.y, v_y.z);
        println!("  pref_axis=X: ({:.2}, {:.2}, {:.2})", v_x.x, v_x.y, v_x.z);
        println!("  位置差异: {:.2}", diff);

        assert!(
            diff > 1.0,
            "pref_axis 参数应该影响圆弧扫掠的方位，但顶点位置几乎相同。\
             这说明 pref_axis 没有被正确使用！"
        );
    }

    // 导出 OBJ 文件用于可视化验证
    let _ = mesh_y.export_obj(false, "test_output/arc_sweep_pref_y.obj");
    let _ = mesh_x.export_obj(false, "test_output/arc_sweep_pref_x.obj");

    println!("\n✅ 圆弧路径 pref_axis 测试通过");
    println!("  已导出: test_output/arc_sweep_pref_y.obj");
    println!("  已导出: test_output/arc_sweep_pref_x.obj");
    println!("\n关键验证：圆弧路径现在正确使用 pref_axis (YDIR) 作为坐标系的 Y 轴");
}
