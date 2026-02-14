use crate::RefnoEnum;
use crate::parsed_data::{CateProfileParam, SProfileData};
/// 验证 normalize_spine_segments 重构后的功能
///
/// 验证清单：
/// - [x] GENSEC 单段直线生成正确
/// - [x] GENSEC 多段直线生成正确
/// - [x] GENSEC 含圆弧路径生成正确
/// - [ ] WALL 弧形墙生成正确 (需要更多模拟数据)
/// - [ ] STWALL 结构墙生成正确 (需要更多模拟数据)
/// - [x] DRNS/DRNE 端面倾斜生成正确
/// - [ ] 布尔运算结果正确 (需要集成测试)
/// - [ ] LOD 多级精度正确 (需要集成测试)
/// - [ ] 缓存复用率统计无下降 (需要运行时测试)
use crate::prim_geo::spine::{Arc3D, Line3D, SegmentPath, SweepPath3D};
use crate::prim_geo::sweep_solid::SweepSolid;
use glam::{DVec3, Vec2, Vec3};
use std::f32::consts::PI;

/// 创建一个简单的圆形截面用于测试（用多边形近似）
fn create_circle_profile(diameter: f32) -> CateProfileParam {
    let radius = diameter / 2.0;
    let segments = 16; // 16边形近似圆形
    let mut verts = Vec::new();

    for i in 0..segments {
        let angle = 2.0 * PI * (i as f32) / (segments as f32);
        verts.push(Vec2::new(radius * angle.cos(), radius * angle.sin()));
    }

    CateProfileParam::SPRO(SProfileData {
        refno: RefnoEnum::default(),
        verts: verts.clone(),
        frads: vec![0.0; verts.len()],
        plin_pos: Vec2::ZERO,
        plin_axis: Vec3::Y,
        plax: Vec3::Y,
        na_axis: Vec3::Z,
    })
}

/// 创建一个矩形截面用于测试
fn create_rect_profile(width: f32, height: f32) -> CateProfileParam {
    let hw = width / 2.0;
    let hh = height / 2.0;

    let verts = vec![
        Vec2::new(-hw, -hh),
        Vec2::new(hw, -hh),
        Vec2::new(hw, hh),
        Vec2::new(-hw, hh),
    ];

    CateProfileParam::SPRO(SProfileData {
        refno: RefnoEnum::default(),
        verts: verts.clone(),
        frads: vec![0.0; verts.len()],
        plin_pos: Vec2::ZERO,
        plin_axis: Vec3::Y,
        plax: Vec3::Y,
        na_axis: Vec3::Z,
    })
}

/// 创建一个 L 形截面用于测试
fn create_l_profile(width: f32, height: f32, thickness: f32) -> CateProfileParam {
    let hw = width / 2.0;
    let hh = height / 2.0;

    let verts = vec![
        Vec2::new(-hw, -hh),
        Vec2::new(hw, -hh),
        Vec2::new(hw, -hh + thickness),
        Vec2::new(-hw + thickness, -hh + thickness),
        Vec2::new(-hw + thickness, hh),
        Vec2::new(-hw, hh),
    ];

    CateProfileParam::SPRO(SProfileData {
        refno: RefnoEnum::default(),
        verts: verts.clone(),
        frads: vec![0.0; verts.len()],
        plin_pos: Vec2::ZERO,
        plin_axis: Vec3::Y,
        plax: Vec3::Y,
        na_axis: Vec3::Z,
    })
}

/// 创建一个 H 形截面用于测试
fn create_h_profile(width: f32, height: f32, web_thk: f32, flg_thk: f32) -> CateProfileParam {
    let hw = width / 2.0;
    let hh = height / 2.0;
    let hwt = web_thk / 2.0;

    let verts = vec![
        Vec2::new(-hw, -hh),
        Vec2::new(-hw, -hh + flg_thk),
        Vec2::new(-hwt, -hh + flg_thk),
        Vec2::new(-hwt, hh - flg_thk),
        Vec2::new(-hw, hh - flg_thk),
        Vec2::new(-hw, hh),
        Vec2::new(hw, hh),
        Vec2::new(hw, hh - flg_thk),
        Vec2::new(hwt, hh - flg_thk),
        Vec2::new(hwt, -hh + flg_thk),
        Vec2::new(hw, -hh + flg_thk),
        Vec2::new(hw, -hh),
    ];

    CateProfileParam::SPRO(SProfileData {
        refno: RefnoEnum::default(),
        verts: verts.clone(),
        frads: vec![0.0; verts.len()],
        plin_pos: Vec2::ZERO,
        plin_axis: Vec3::Y,
        plax: Vec3::Y,
        na_axis: Vec3::Z,
    })
}

// ============================================================================
// 测试 1: GENSEC 单段直线生成
// ============================================================================

#[test]
fn test_gensec_single_line_no_slope() {
    println!("\n=== 测试 GENSEC 单段直线（无端面倾斜） ===");

    // 创建圆形截面
    let profile = create_circle_profile(100.0);

    // 创建单段直线路径：沿 Z 轴 500mm
    let line_path = SweepPath3D::from_line(Line3D {
        start: Vec3::ZERO,
        end: Vec3::Z * 500.0,
        is_spine: true,
    });

    // 创建 SweepSolid
    let sweep_solid = SweepSolid {
        profile: profile.clone(),
        drns: None, // 无端面倾斜
        drne: None,
        plax: Vec3::Y,
        bangle: 0.0,
        extrude_dir: DVec3::Z,
        height: 500.0,
        path: line_path.clone(),
        lmirror: false,
        spine_segments: vec![],
    };

    println!("  创建 SweepSolid: 圆形截面(dia=100mm), 直���长度 500mm");
    println!("  路径段数: {}", line_path.segment_count());
    println!("  路径长度: {:.3} mm", line_path.length());

    // 验证路径属性
    assert!(line_path.is_single_segment(), "应该是单段路径");
    assert!(line_path.as_single_line().is_some(), "应该能获取直线引用");

    // 验证是否可复用（单段直线 + 无倾斜）
    assert!(!sweep_solid.is_sloped(), "不应该有端面倾斜");

    // 尝试生成 CSG mesh
    use crate::shape::pdms_shape::BrepShapeTrait;
    match sweep_solid.gen_csg_shape() {
        Ok(csg_mesh) => {
            println!("  ✅ CSG Mesh 生成成功！");
            println!("    顶点数: {}", csg_mesh.vertices.len());
            println!("    法线数: {}", csg_mesh.normals.len());
            println!("    三角形数: {}", csg_mesh.indices.len() / 3);

            // 验证顶点数量合理（圆形截面应该有一定数量的顶点）
            assert!(csg_mesh.vertices.len() > 0, "顶点数应该大于0");
            assert!(csg_mesh.indices.len() > 0, "索引数应该大于0");
        }
        Err(e) => {
            panic!("CSG Mesh 生成失败: {:?}", e);
        }
    }

    println!("✅ GENSEC 单段直线（无倾斜）测试通过");
}

#[test]
fn test_gensec_single_line_with_drns_drne() {
    println!("\n=== 测试 GENSEC 单段直线（有端面倾斜） ===");

    let profile = create_rect_profile(100.0, 50.0);

    let line_path = SweepPath3D::from_line(Line3D {
        start: Vec3::ZERO,
        end: Vec3::Z * 1000.0,
        is_spine: true,
    });

    // 端面倾斜 45 度
    let drns = Some(DVec3::new(0.707, 0.0, 0.707).normalize());
    let drne = Some(DVec3::new(-0.707, 0.0, 0.707).normalize());

    let sweep_solid = SweepSolid {
        profile: profile.clone(),
        drns,
        drne,
        plax: Vec3::Y,
        bangle: 0.0,
        extrude_dir: DVec3::Z,
        height: 1000.0,
        path: line_path.clone(),
        lmirror: false,
        spine_segments: vec![],
    };

    println!("  创建 SweepSolid: 矩形截面(100x50mm), 直线长度 1000mm");
    println!("  DRNS: {:?}", drns);
    println!("  DRNE: {:?}", drne);

    // 验证是否有端面倾斜
    assert!(sweep_solid.is_sloped(), "应该有端面倾斜");
    assert!(sweep_solid.is_drns_sloped(), "DRNS 应该是倾斜的");
    assert!(sweep_solid.is_drne_sloped(), "DRNE 应该是倾斜的");

    use crate::shape::pdms_shape::BrepShapeTrait;
    match sweep_solid.gen_csg_shape() {
        Ok(csg_mesh) => {
            println!("  ✅ CSG Mesh 生成成功！");
            println!("    顶点数: {}", csg_mesh.vertices.len());
            println!("    三角形数: {}", csg_mesh.indices.len() / 3);

            assert!(csg_mesh.vertices.len() > 0, "顶点数应该大于0");
        }
        Err(e) => {
            panic!("CSG Mesh 生成失败: {:?}", e);
        }
    }

    println!("✅ GENSEC 单段直线（有倾斜）测试通过");
}

// ============================================================================
// 测试 2: GENSEC 多段直线生成
// ============================================================================

#[test]
fn test_gensec_multi_segment_lines() {
    println!("\n=== 测试 GENSEC 多段直线路径 ===");

    let profile = create_circle_profile(80.0);

    // 创建 L 形路径：先沿 Z 轴，再沿 X 轴
    let segments = vec![
        SegmentPath::Line(Line3D {
            start: Vec3::ZERO,
            end: Vec3::Z * 300.0,
            is_spine: true,
        }),
        SegmentPath::Line(Line3D {
            start: Vec3::Z * 300.0,
            end: Vec3::new(200.0, 0.0, 300.0),
            is_spine: true,
        }),
    ];

    let path = SweepPath3D::from_segments(segments);

    println!("  创建 L 形路径: Z 方向 300mm + X 方向 200mm");
    println!("  路径段数: {}", path.segment_count());
    println!("  路径总长度: {:.3} mm", path.length());

    // 验证路径属性
    assert!(!path.is_single_segment(), "不应该是单段路径");
    assert_eq!(path.segment_count(), 2, "应该有2段");

    // 验证连续性
    let (is_continuous, discontinuity) = path.validate_continuity();
    println!(
        "  路径连续性: {} (不连续位置: {:?})",
        is_continuous, discontinuity
    );

    let sweep_solid = SweepSolid {
        profile: profile.clone(),
        drns: None,
        drne: None,
        plax: Vec3::Y,
        bangle: 0.0,
        extrude_dir: DVec3::Z,
        height: path.length(),
        path: path.clone(),
        lmirror: false,
        spine_segments: vec![],
    };

    use crate::shape::pdms_shape::BrepShapeTrait;
    match sweep_solid.gen_csg_shape() {
        Ok(csg_mesh) => {
            println!("  ✅ CSG Mesh 生成成功！");
            println!("    顶点数: {}", csg_mesh.vertices.len());
            println!("    三角形数: {}", csg_mesh.indices.len() / 3);

            assert!(csg_mesh.vertices.len() > 0, "顶点数应该大于0");
        }
        Err(e) => {
            panic!("CSG Mesh 生成失败: {:?}", e);
        }
    }

    println!("✅ GENSEC 多段直线测试通过");
}

// ============================================================================
// 测试 3: GENSEC 含圆弧路径生成
// ============================================================================

#[test]
fn test_gensec_path_with_arc() {
    println!("\n=== 测试 GENSEC 含圆弧路径 ===");

    let profile = create_circle_profile(60.0);

    // 创建包含圆弧的路径：直线 -> 90度圆弧 -> 直线
    let arc_radius = 100.0;
    let segments = vec![
        // 第一段直线：沿 Z 轴
        SegmentPath::Line(Line3D {
            start: Vec3::ZERO,
            end: Vec3::Z * 200.0,
            is_spine: true,
        }),
        // 90度圆弧：从 Z 轴转向 X 轴
        SegmentPath::Arc(Arc3D {
            center: Vec3::new(arc_radius, 0.0, 200.0),
            radius: arc_radius,
            angle: PI / 2.0,
            start_pt: Vec3::new(0.0, 0.0, 200.0),
            clock_wise: false,
            axis: Vec3::NEG_Y, // 绕 -Y 轴旋转
            pref_axis: Vec3::Y,
        }),
        // 第二段直线：沿 X 轴
        SegmentPath::Line(Line3D {
            start: Vec3::new(arc_radius, 0.0, 200.0 + arc_radius),
            end: Vec3::new(arc_radius + 200.0, 0.0, 200.0 + arc_radius),
            is_spine: true,
        }),
    ];

    let path = SweepPath3D::from_segments(segments);

    // 预期长度：200 + π*100/2 + 200 ≈ 557.08
    let expected_length = 200.0 + PI * arc_radius / 2.0 + 200.0;

    println!("  创建路径: 直线(200mm) -> 90度圆弧(R=100mm) -> 直线(200mm)");
    println!("  路径段数: {}", path.segment_count());
    println!(
        "  路径总长度: {:.3} mm (预期: {:.3})",
        path.length(),
        expected_length
    );

    // 验证路径属性
    assert_eq!(path.segment_count(), 3, "应该有3段");
    assert!(
        (path.length() - expected_length).abs() < 1.0,
        "路径长度应该接近 {:.3}，实际为 {:.3}",
        expected_length,
        path.length()
    );

    let sweep_solid = SweepSolid {
        profile: profile.clone(),
        drns: None,
        drne: None,
        plax: Vec3::Y,
        bangle: 0.0,
        extrude_dir: DVec3::Z,
        height: path.length(),
        path: path.clone(),
        lmirror: false,
        spine_segments: vec![],
    };

    use crate::shape::pdms_shape::BrepShapeTrait;
    match sweep_solid.gen_csg_shape() {
        Ok(csg_mesh) => {
            println!("  ✅ CSG Mesh 生成成功！");
            println!("    顶点数: {}", csg_mesh.vertices.len());
            println!("    三角形数: {}", csg_mesh.indices.len() / 3);

            // 含圆弧的路径应该有更多顶点（因为圆弧需要细分）
            assert!(csg_mesh.vertices.len() > 100, "含圆弧路径应该有较多顶点");
        }
        Err(e) => {
            panic!("CSG Mesh 生成失败: {:?}", e);
        }
    }

    println!("✅ GENSEC 含圆弧路径测试通过");
}

// ============================================================================
// 测试 4: 单段圆弧路径（弧形墙场景）
// ============================================================================

#[test]
fn test_single_arc_path() {
    println!("\n=== 测试单段圆弧路径（弧形墙场景） ===");

    let profile = create_rect_profile(200.0, 150.0); // 墙体截面

    // 创建 180 度圆弧
    let arc_radius = 500.0;
    let arc = Arc3D {
        center: Vec3::ZERO,
        radius: arc_radius,
        angle: PI, // 180 度
        start_pt: Vec3::X * arc_radius,
        clock_wise: false,
        axis: Vec3::Z,
        pref_axis: Vec3::Y,
    };

    let path = SweepPath3D::from_arc(arc.clone());

    // 预期长度：π * 500 ≈ 1570.8
    let expected_length = PI * arc_radius;

    println!("  创建 180 度圆弧墙: 半径=500mm");
    println!(
        "  路径长度: {:.3} mm (预期: {:.3})",
        path.length(),
        expected_length
    );

    // 验证路径属性
    assert!(path.is_single_segment(), "应该是单段路径");
    assert!(path.as_single_arc().is_some(), "应该能获取圆弧引用");

    let sweep_solid = SweepSolid {
        profile: profile.clone(),
        drns: None,
        drne: None,
        plax: Vec3::Y,
        bangle: 0.0,
        extrude_dir: DVec3::Z,
        height: path.length(),
        path: path.clone(),
        lmirror: false,
        spine_segments: vec![],
    };

    use crate::shape::pdms_shape::BrepShapeTrait;
    match sweep_solid.gen_csg_shape() {
        Ok(csg_mesh) => {
            println!("  ✅ CSG Mesh 生成成功！");
            println!("    顶点数: {}", csg_mesh.vertices.len());
            println!("    三角形数: {}", csg_mesh.indices.len() / 3);

            // 180度圆弧应该有较多顶点
            assert!(csg_mesh.vertices.len() > 50, "圆弧路径应该有较多顶点");

            // 导出为 OBJ 以便视觉验证
            if let Err(e) = csg_mesh.export_obj(false, "test_output/refactor_arc_wall.obj") {
                println!("    ⚠️  OBJ 导出失败: {}", e);
            } else {
                println!("    📁 已导出: test_output/refactor_arc_wall.obj");
            }
        }
        Err(e) => {
            panic!("CSG Mesh 生成失败: {:?}", e);
        }
    }

    println!("✅ 单段圆弧路径测试通过");
}

// ============================================================================
// 测试 5: BANGLE 旋转测试
// ============================================================================

#[test]
fn test_bangle_rotation() {
    println!("\n=== 测试 BANGLE 旋转 ===");

    let profile = create_rect_profile(100.0, 50.0); // 非对称截面

    let line_path = SweepPath3D::from_line(Line3D {
        start: Vec3::ZERO,
        end: Vec3::Z * 300.0,
        is_spine: true,
    });

    // 测试不同的 BANGLE 值
    let bangle_values = [0.0, 45.0, 90.0];

    for bangle in bangle_values {
        println!("\n  测试 BANGLE = {} 度", bangle);

        let sweep_solid = SweepSolid {
            profile: profile.clone(),
            drns: None,
            drne: None,
            plax: Vec3::Y,
            bangle,
            extrude_dir: DVec3::Z,
            height: 300.0,
            path: line_path.clone(),
            lmirror: false,
            spine_segments: vec![],
        };

        use crate::shape::pdms_shape::BrepShapeTrait;
        match sweep_solid.gen_csg_shape() {
            Ok(csg_mesh) => {
                println!(
                    "    ✅ CSG Mesh 生成成功！顶点数: {}",
                    csg_mesh.vertices.len()
                );

                // 导出不同 BANGLE 的结果
                let filename = format!("test_output/refactor_bangle_{}.obj", bangle as i32);
                if let Err(e) = csg_mesh.export_obj(false, &filename) {
                    println!("    ⚠️  OBJ 导出失败: {}", e);
                } else {
                    println!("    📁 已导出: {}", filename);
                }
            }
            Err(e) => {
                panic!("BANGLE={} 时 CSG Mesh 生成失败: {:?}", bangle, e);
            }
        }
    }

    println!("\n✅ BANGLE 旋转测试通过");
}

// ============================================================================
// 测试 6: 路径连续性验证
// ============================================================================

#[test]
fn test_path_continuity_validation() {
    println!("\n=== 测试路径连续性验证 ===");

    // 连续路径
    let continuous_segments = vec![
        SegmentPath::Line(Line3D {
            start: Vec3::ZERO,
            end: Vec3::Z * 100.0,
            is_spine: true,
        }),
        SegmentPath::Line(Line3D {
            start: Vec3::Z * 100.0, // 与上一段终点相同
            end: Vec3::new(100.0, 0.0, 100.0),
            is_spine: true,
        }),
    ];

    let continuous_path = SweepPath3D::from_segments(continuous_segments);
    let (is_continuous, _) = continuous_path.validate_continuity();
    assert!(is_continuous, "连续路径应该验证为连续");
    println!("  ✅ 连续路径验证通过");

    // 不连续路径
    let discontinuous_segments = vec![
        SegmentPath::Line(Line3D {
            start: Vec3::ZERO,
            end: Vec3::Z * 100.0,
            is_spine: true,
        }),
        SegmentPath::Line(Line3D {
            start: Vec3::Z * 150.0, // 有 50mm 间隙
            end: Vec3::new(100.0, 0.0, 150.0),
            is_spine: true,
        }),
    ];

    let discontinuous_path = SweepPath3D::from_segments(discontinuous_segments);
    let (is_continuous, discontinuity_index) = discontinuous_path.validate_continuity();
    assert!(!is_continuous, "不连续路径应该验证为不连续");
    // 返回的是第一个不连续点的索引（段 0 的终点与段 1 的起点不连续，返回 0）
    assert!(discontinuity_index.is_some(), "应该检测到不连续位置");
    println!(
        "  ✅ 不连续路径验证通过（在索引 {:?} 处发现）",
        discontinuity_index
    );

    println!("✅ 路径连续性验证测试通过");
}

// ============================================================================
// 测试 7: 镜像 (LMIRROR) 测试
// ============================================================================

#[test]
fn test_lmirror() {
    println!("\n=== 测试 LMIRROR 镜像 ===");

    // 使用非对称 L 型截面
    let profile = create_l_profile(100.0, 80.0, 10.0);

    let line_path = SweepPath3D::from_line(Line3D {
        start: Vec3::ZERO,
        end: Vec3::Z * 200.0,
        is_spine: true,
    });

    // 不镜像
    let sweep_no_mirror = SweepSolid {
        profile: profile.clone(),
        drns: None,
        drne: None,
        plax: Vec3::Y,
        bangle: 0.0,
        extrude_dir: DVec3::Z,
        height: 200.0,
        path: line_path.clone(),
        lmirror: false,
        spine_segments: vec![],
    };

    // 镜像
    let sweep_mirror = SweepSolid {
        profile: profile.clone(),
        drns: None,
        drne: None,
        plax: Vec3::Y,
        bangle: 0.0,
        extrude_dir: DVec3::Z,
        height: 200.0,
        path: line_path.clone(),
        lmirror: true,
        spine_segments: vec![],
    };

    use crate::shape::pdms_shape::BrepShapeTrait;

    match sweep_no_mirror.gen_csg_shape() {
        Ok(csg_mesh) => {
            println!(
                "  ✅ 不镜像版本生成成功！顶点数: {}",
                csg_mesh.vertices.len()
            );
            let _ = csg_mesh.export_obj(false, "test_output/refactor_lmirror_false.obj");
        }
        Err(e) => {
            panic!("不镜像版本生成失败: {:?}", e);
        }
    }

    match sweep_mirror.gen_csg_shape() {
        Ok(csg_mesh) => {
            println!("  ✅ 镜像版本生成成功！顶点数: {}", csg_mesh.vertices.len());
            let _ = csg_mesh.export_obj(false, "test_output/refactor_lmirror_true.obj");
        }
        Err(e) => {
            panic!("镜像版本生成失败: {:?}", e);
        }
    }

    println!("✅ LMIRROR 镜像测试通过");
}

// ============================================================================
// 综合测试：模拟真实 GENSEC 场景
// ============================================================================

#[test]
fn test_realistic_gensec_scenario() {
    println!("\n=== 综合测试：模拟真实 GENSEC 场景 ===");

    // 模拟一个真实的管道支架场景
    // H型钢截面
    let profile = create_h_profile(200.0, 200.0, 8.0, 12.0);

    // 创建 Z 形路径：水平 -> 垂直 -> 水平
    let segments = vec![
        SegmentPath::Line(Line3D {
            start: Vec3::ZERO,
            end: Vec3::X * 500.0,
            is_spine: true,
        }),
        SegmentPath::Line(Line3D {
            start: Vec3::X * 500.0,
            end: Vec3::new(500.0, 0.0, 300.0),
            is_spine: true,
        }),
        SegmentPath::Line(Line3D {
            start: Vec3::new(500.0, 0.0, 300.0),
            end: Vec3::new(1000.0, 0.0, 300.0),
            is_spine: true,
        }),
    ];

    let path = SweepPath3D::from_segments(segments);

    println!("  创建 Z 形管道支架:");
    println!("    截面: H型钢 200x200mm");
    println!("    路径: 水平(500mm) -> 垂直(300mm) -> 水平(500mm)");
    println!("    总长度: {:.3} mm", path.length());

    let sweep_solid = SweepSolid {
        profile: profile.clone(),
        drns: None,
        drne: None,
        plax: Vec3::Y,
        bangle: 0.0,
        extrude_dir: DVec3::Z,
        height: path.length(),
        path: path.clone(),
        lmirror: false,
        spine_segments: vec![],
    };

    use crate::shape::pdms_shape::BrepShapeTrait;
    match sweep_solid.gen_csg_shape() {
        Ok(csg_mesh) => {
            println!("\n  ✅ CSG Mesh 生成成功！");
            println!("    顶点数: {}", csg_mesh.vertices.len());
            println!("    法线数: {}", csg_mesh.normals.len());
            println!("    三角形数: {}", csg_mesh.indices.len() / 3);

            // 计算包围盒
            if csg_mesh.vertices.len() > 0 {
                let mut min = csg_mesh.vertices[0];
                let mut max = csg_mesh.vertices[0];
                for v in &csg_mesh.vertices {
                    min = min.min(*v);
                    max = max.max(*v);
                }
                println!(
                    "    包围盒: ({:.1}, {:.1}, {:.1}) -> ({:.1}, {:.1}, {:.1})",
                    min.x, min.y, min.z, max.x, max.y, max.z
                );
            }

            // 导出 OBJ
            if let Err(e) = csg_mesh.export_obj(false, "test_output/refactor_realistic_gensec.obj")
            {
                println!("    ⚠️  OBJ 导出失败: {}", e);
            } else {
                println!("    📁 已导出: test_output/refactor_realistic_gensec.obj");
            }
        }
        Err(e) => {
            panic!("CSG Mesh 生成失败: {:?}", e);
        }
    }

    println!("\n✅ 综合测试通过");
}
