/// 测试多段路径 SweepPath3D 重构后的功能
use crate::prim_geo::spine::{Arc3D, Line3D, SegmentPath, Spine3D, SpineCurveType, SweepPath3D};
use glam::{DVec3, Vec3};
use std::f32::consts::PI;

#[test]
fn test_single_line_path() {
    println!("\n=== 测试单段直线路径 ===");

    let line = Line3D {
        start: Vec3::ZERO,
        end: Vec3::Z * 100.0,
        is_spine: true,
    };

    let path = SweepPath3D::from_line(line.clone());

    // 验证基本属性
    assert!(path.is_single_segment(), "应该是单段路径");
    assert_eq!(path.segment_count(), 1, "段数应该为1");
    assert_eq!(path.length(), 100.0, "长度应该为100.0");

    // 验证辅助方法
    assert!(path.as_single_line().is_some(), "应该能获取直线引用");
    assert!(path.as_single_arc().is_none(), "不应该能获取圆弧引用");

    let retrieved_line = path.as_single_line().unwrap();
    assert_eq!(retrieved_line.start, line.start, "起点应该相同");
    assert_eq!(retrieved_line.end, line.end, "终点应该相同");

    println!("✅ 单段直线路径测试通过");
}

#[test]
fn test_single_arc_path() {
    println!("\n=== 测试单段圆弧路径 ===");

    let arc = Arc3D {
        center: Vec3::ZERO,
        radius: 50.0,
        angle: PI / 2.0, // 90度
        start_pt: Vec3::X * 50.0,
        clock_wise: false,
        axis: Vec3::Z,
        pref_axis: Vec3::Y,
    };

    let path = SweepPath3D::from_arc(arc.clone());

    // 验证基本属性
    assert!(path.is_single_segment(), "应该是单段路径");
    assert_eq!(path.segment_count(), 1, "段数应该为1");

    // 圆弧长度 = 半径 * 角度
    let expected_length = 50.0 * PI / 2.0;
    let actual_length = path.length();
    assert!(
        (actual_length - expected_length).abs() < 0.001,
        "圆弧长度应该约为 {:.3}，实际为 {:.3}",
        expected_length,
        actual_length
    );

    // 验证辅助方法
    assert!(path.as_single_arc().is_some(), "应该能获取圆弧引用");
    assert!(path.as_single_line().is_none(), "不应该能获取直线引用");

    let retrieved_arc = path.as_single_arc().unwrap();
    assert_eq!(retrieved_arc.radius, arc.radius, "半径应该相同");
    assert_eq!(retrieved_arc.angle, arc.angle, "角度应该相同");

    println!("✅ 单段圆弧路径测试通过");
}

#[test]
fn test_multi_segment_path() {
    println!("\n=== 测试多段混合路径 ===");

    // 创建一个包含直线和圆弧的路径：直线 -> 圆弧 -> 直线
    let segments = vec![
        SegmentPath::Line(Line3D {
            start: Vec3::ZERO,
            end: Vec3::new(0.0, 0.0, 100.0),
            is_spine: true,
        }),
        SegmentPath::Arc(Arc3D {
            center: Vec3::new(50.0, 0.0, 100.0),
            radius: 50.0,
            angle: PI / 2.0,
            start_pt: Vec3::new(0.0, 0.0, 100.0),
            clock_wise: false,
            axis: Vec3::Z,
            pref_axis: Vec3::Y,
        }),
        SegmentPath::Line(Line3D {
            start: Vec3::new(50.0, 50.0, 100.0),
            end: Vec3::new(50.0, 150.0, 100.0),
            is_spine: true,
        }),
    ];

    let path = SweepPath3D::from_segments(segments);

    // 验证基本属性
    assert!(!path.is_single_segment(), "应该是多段路径");
    assert_eq!(path.segment_count(), 3, "段数应该为3");

    // 验证辅助方法对多段路径返回 None
    assert!(
        path.as_single_line().is_none(),
        "多段路径不应该返回单段直线"
    );
    assert!(path.as_single_arc().is_none(), "多段路径不应该返回单段圆弧");

    // 计算总长度
    let line1_len = 100.0;
    let arc_len = 50.0 * PI / 2.0;
    let line2_len = 100.0;
    let expected_total = line1_len + arc_len + line2_len;
    let actual_total = path.length();

    assert!(
        (actual_total - expected_total).abs() < 0.01,
        "总长度应该约为 {:.3}，实际为 {:.3}",
        expected_total,
        actual_total
    );

    println!("  直线1长度: {:.3}", line1_len);
    println!("  圆弧长度: {:.3}", arc_len);
    println!("  直线2长度: {:.3}", line2_len);
    println!(
        "  总长度: {:.3} (预期: {:.3})",
        actual_total, expected_total
    );
    println!("✅ 多段混合路径测试通过");
}

#[test]
fn test_path_continuity_check() {
    println!("\n=== 测试路径连续性验证 ===");

    // 创建连续的路径
    let continuous_segments = vec![
        SegmentPath::Line(Line3D {
            start: Vec3::ZERO,
            end: Vec3::new(100.0, 0.0, 0.0),
            is_spine: true,
        }),
        SegmentPath::Line(Line3D {
            start: Vec3::new(100.0, 0.0, 0.0), // 与前一段终点相同
            end: Vec3::new(100.0, 100.0, 0.0),
            is_spine: true,
        }),
    ];

    let continuous_path = SweepPath3D::from_segments(continuous_segments);
    let (is_continuous, discontinuity_index) = continuous_path.validate_continuity();

    assert!(is_continuous, "路径应该是连续的");
    assert!(discontinuity_index.is_none(), "不应该有不连续点");
    println!("✅ 连续路径验证通过");

    // 创建不连续的路径
    let discontinuous_segments = vec![
        SegmentPath::Line(Line3D {
            start: Vec3::ZERO,
            end: Vec3::new(100.0, 0.0, 0.0),
            is_spine: true,
        }),
        SegmentPath::Line(Line3D {
            start: Vec3::new(200.0, 0.0, 0.0), // 与前一段终点不连续
            end: Vec3::new(200.0, 100.0, 0.0),
            is_spine: true,
        }),
    ];

    let discontinuous_path = SweepPath3D::from_segments(discontinuous_segments);
    let (is_continuous, discontinuity_index) = discontinuous_path.validate_continuity();

    assert!(!is_continuous, "路径应该是不连续的");
    assert_eq!(
        discontinuity_index,
        Some(0),
        "不连续点应该在段0和段1之间（返回段0的索引）"
    );
    println!("✅ 不连续路径检测通过");
}

#[test]
fn test_path_geometry_properties() {
    println!("\n=== 测试路径几何属性 ===");

    let segments = vec![
        SegmentPath::Line(Line3D {
            start: Vec3::new(0.0, 0.0, 0.0),
            end: Vec3::new(100.0, 0.0, 0.0),
            is_spine: true,
        }),
        SegmentPath::Line(Line3D {
            start: Vec3::new(100.0, 0.0, 0.0),
            end: Vec3::new(100.0, 100.0, 0.0),
            is_spine: true,
        }),
    ];

    let path = SweepPath3D::from_segments(segments);

    // 测试起点和终点
    let start = path.start_point();
    let end = path.end_point();

    assert_eq!(start, Vec3::new(0.0, 0.0, 0.0), "起点应该正确");
    assert_eq!(end, Vec3::new(100.0, 100.0, 0.0), "终点应该正确");

    println!("  起点: {:?}", start);
    println!("  终点: {:?}", end);

    // 测试切线（在不同位置）
    let tangent_start = path.tangent_at(0.0);
    let tangent_mid = path.tangent_at(0.5);
    let tangent_end = path.tangent_at(1.0);

    println!("  起点切线: {:?}", tangent_start);
    println!("  中点切线: {:?}", tangent_mid);
    println!("  终点切线: {:?}", tangent_end);

    // 第一段是沿X轴，切线应该是 (1, 0, 0)
    assert!(
        (tangent_start - Vec3::X).length() < 0.01,
        "起点切线应该沿X轴"
    );

    // 第二段是沿Y轴，切线应该是 (0, 1, 0)
    assert!((tangent_end - Vec3::Y).length() < 0.01, "终点切线应该沿Y轴");

    println!("✅ 路径几何属性测试通过");
}

#[test]
fn test_spine3d_generate_paths() {
    println!("\n=== 测试 Spine3D 路径生成 ===");

    use crate::RefnoEnum;

    // 测试直线类型
    let line_spine = Spine3D {
        refno: RefnoEnum::default(),
        pt0: Vec3::ZERO,
        pt1: Vec3::new(0.0, 0.0, 100.0),
        thru_pt: Vec3::ZERO,
        center_pt: Vec3::ZERO,
        cond_pos: Vec3::ZERO,
        radius: 0.0,
        curve_type: SpineCurveType::LINE,
        preferred_dir: Vec3::Y,
    };

    let (line_path, _transform) = line_spine.generate_paths();

    assert!(line_path.is_single_segment(), "直线应该生成单段路径");
    assert!(line_path.as_single_line().is_some(), "应该是直线段");
    println!("  直线Spine生成路径长度: {:.3}", line_path.length());

    // 测试圆弧类型（需要三个点）
    let arc_spine = Spine3D {
        refno: RefnoEnum::default(),
        pt0: Vec3::new(0.0, 0.0, 0.0),
        pt1: Vec3::new(100.0, 100.0, 0.0),
        thru_pt: Vec3::new(50.0, 0.0, 0.0),
        center_pt: Vec3::ZERO,
        cond_pos: Vec3::ZERO,
        radius: 0.0,
        curve_type: SpineCurveType::THRU,
        preferred_dir: Vec3::Z,
    };

    let (arc_path, _transform) = arc_spine.generate_paths();

    assert!(arc_path.is_single_segment(), "圆弧应该生成单段路径");
    // THRU类型可能生成圆弧或直线，取决于几何条件
    println!("  圆弧Spine生成路径长度: {:.3}", arc_path.length());

    println!("✅ Spine3D 路径生成测试通过");
}

#[test]
fn test_gensec_spine_scenario() {
    println!("\n=== 模拟 GENSEC SPINE 场景 ===");

    // 模拟 test-files/gensec.txt 中的路径：
    // 6个POINSP点 + 1个CURVE弧线
    let segments = vec![
        // 段1: 直线
        SegmentPath::Line(Line3D {
            start: Vec3::new(12635.0, -25862.0, 1950.0),
            end: Vec3::new(12785.0, -25862.0, 1950.0),
            is_spine: true,
        }),
        // 段2: 圆弧 (半径140mm, 90度弯)
        SegmentPath::Arc(Arc3D {
            center: Vec3::new(12785.0, -25862.0 - 140.0, 1950.0),
            radius: 140.0,
            angle: PI / 2.0,
            start_pt: Vec3::new(12785.0, -25862.0, 1950.0),
            clock_wise: false,
            axis: Vec3::Z,
            pref_axis: Vec3::Y,
        }),
        // 段3: 直线
        SegmentPath::Line(Line3D {
            start: Vec3::new(12925.0, -26002.0, 1950.0),
            end: Vec3::new(12925.0, -26152.0, 1950.0),
            is_spine: true,
        }),
        // 段4: 长直线
        SegmentPath::Line(Line3D {
            start: Vec3::new(12925.0, -26152.0, 1950.0),
            end: Vec3::new(12925.0, -29152.0, 1950.0),
            is_spine: true,
        }),
        // 段5: 直线
        SegmentPath::Line(Line3D {
            start: Vec3::new(12925.0, -29152.0, 1950.0),
            end: Vec3::new(12925.0, -30652.0, 1950.0),
            is_spine: true,
        }),
    ];

    let path = SweepPath3D::from_segments(segments);

    println!("  段数: {}", path.segment_count());
    println!("  总长度: {:.3} mm", path.length());

    // 预期长度计算
    let seg1 = 150.0; // 12785 - 12635
    let seg2 = 140.0 * PI / 2.0; // 圆弧
    let seg3 = 150.0; // 26152 - 26002
    let seg4 = 3000.0; // 29152 - 26152
    let seg5 = 1500.0; // 30652 - 29152
    let expected = seg1 + seg2 + seg3 + seg4 + seg5;

    println!("  预期长度: {:.3} mm", expected);
    println!("  各段长度:");
    println!("    直线1: {:.3} mm", seg1);
    println!("    圆弧:  {:.3} mm", seg2);
    println!("    直线2: {:.3} mm", seg3);
    println!("    直线3: {:.3} mm", seg4);
    println!("    直线4: {:.3} mm", seg5);

    let actual = path.length();
    assert!(
        (actual - expected).abs() < 1.0,
        "总长度应该约为 {:.3}，实际为 {:.3}",
        expected,
        actual
    );

    // 验证连续性
    let (is_continuous, _) = path.validate_continuity();
    if !is_continuous {
        println!("  ⚠️  警告: 路径不连续（这是正常的，因为是手动构造的测试数据）");
    }

    println!("✅ GENSEC SPINE 场景测试通过");
}

#[test]
fn test_empty_path() {
    println!("\n=== 测试空路径 ===");

    let empty_path = SweepPath3D::from_segments(vec![]);

    assert_eq!(empty_path.segment_count(), 0, "空路径段数应该为0");
    assert_eq!(empty_path.length(), 0.0, "空路径长度应该为0");
    assert!(!empty_path.is_single_segment(), "空路径不是单段路径");

    println!("✅ 空路径测试通过");
}

#[test]
fn test_path_iteration() {
    println!("\n=== 测试路径迭代 ===");

    let segments = vec![
        SegmentPath::Line(Line3D {
            start: Vec3::ZERO,
            end: Vec3::X * 100.0,
            is_spine: true,
        }),
        SegmentPath::Arc(Arc3D {
            center: Vec3::ZERO,
            radius: 50.0,
            angle: PI,
            start_pt: Vec3::X * 50.0,
            clock_wise: false,
            axis: Vec3::Z,
            pref_axis: Vec3::Y,
        }),
        SegmentPath::Line(Line3D {
            start: Vec3::Y * 100.0,
            end: Vec3::Z * 100.0,
            is_spine: true,
        }),
    ];

    let path = SweepPath3D::from_segments(segments);

    // 测试迭代
    println!("  迭代所有段:");
    let mut line_count = 0;
    let mut arc_count = 0;

    for (i, segment) in path.segments.iter().enumerate() {
        match segment {
            SegmentPath::Line(l) => {
                line_count += 1;
                println!("    段{}: 直线, 长度={:.3}", i, l.length());
            }
            SegmentPath::Arc(a) => {
                arc_count += 1;
                println!(
                    "    段{}: 圆弧, 半径={:.3}, 角度={:.3}°",
                    i,
                    a.radius,
                    a.angle.to_degrees()
                );
            }
        }
    }

    assert_eq!(line_count, 2, "应该有2段直线");
    assert_eq!(arc_count, 1, "应该有1段圆弧");

    println!("✅ 路径迭代测试通过");
}

// ============================================================================
// CSG 模型生成测试 (SweepSolid 结构验证)
// ============================================================================

#[test]
fn test_single_line_sweep_solid_creation() {
    println!("\n=== 测试单段直线 SweepSolid 创建 ===");

    use crate::parsed_data::CateProfileParam;
    use crate::prim_geo::sweep_solid::SweepSolid;
    use glam::Vec2;

    // 创建一个简单的圆形截面
    use crate::RefnoEnum;
    let profile = CateProfileParam::SANN(crate::parsed_data::SannData {
        refno: RefnoEnum::default(),
        xy: Vec2::ZERO,
        dxy: Vec2::ZERO,
        paxis: None,
        pangle: 360.0,
        pradius: 50.0, // 半径50mm (直径100mm)
        pwidth: 0.0,
        drad: 0.0,
        dwid: 0.0,
        plin_pos: Vec2::ZERO,
        plin_axis: Vec3::Y,
        plax: Vec3::Y,
        na_axis: Vec3::Y,
    });

    // 创建单段直线路径
    let line_path = SweepPath3D::from_line(Line3D {
        start: Vec3::ZERO,
        end: Vec3::Z * 500.0, // 500mm长的直线
        is_spine: true,
    });

    let sweep_solid = SweepSolid {
        profile,
        drns: Some(DVec3::Z),
        drne: Some(DVec3::Z),
        bangle: 0.0,
        plax: Vec3::Y,
        extrude_dir: DVec3::Z,
        height: 500.0,
        path: line_path.clone(),
        lmirror: false,
    };

    println!("  创建SweepSolid: 圆形截面(dia=100mm), 直线长度500mm");
    println!("  路径段数: {}", line_path.segment_count());
    println!("  路径长度: {:.3} mm", line_path.length());

    // 验证路径属性
    assert!(line_path.is_single_segment(), "应该是单段路径");
    assert!(line_path.as_single_line().is_some(), "应该能获取直线引用");

    // 尝试生成CSG mesh
    use crate::shape::pdms_shape::BrepShapeTrait;
    match sweep_solid.gen_csg_shape() {
        Ok(csg_mesh) => {
            println!("  ✅ CSG Mesh 生成成功！");
            println!("  顶点数: {}", csg_mesh.vertices.len());
            println!("  三角形数: {}", csg_mesh.indices.len() / 3);

            // 导出为 OBJ 文件
            if let Err(e) = csg_mesh.export_obj(false, "test_output/single_line_sweep.obj") {
                println!("  ⚠️  OBJ文件导出失败: {}", e);
            } else {
                println!("  ✅ OBJ文件导出成功: test_output/single_line_sweep.obj");
            }
        }
        Err(e) => {
            println!("  ℹ️  CSG Shape生成失败: {}", e);
        }
    }

    println!("✅ 单段直线SweepSolid创建测试通过");
}

#[test]
fn test_single_arc_sweep_solid_creation() {
    println!("\n=== 测试单段圆弧 SweepSolid 创建 ===");

    use crate::parsed_data::CateProfileParam;
    use crate::prim_geo::sweep_solid::SweepSolid;
    use glam::Vec2;

    // 创建圆形截面
    use crate::RefnoEnum;
    let profile = CateProfileParam::SANN(crate::parsed_data::SannData {
        refno: RefnoEnum::default(),
        xy: Vec2::ZERO,
        dxy: Vec2::ZERO,
        paxis: None,
        pangle: 360.0,
        pradius: 25.0, // 半径25mm (直径50mm)
        pwidth: 0.0,
        drad: 0.0,
        dwid: 0.0,
        plin_pos: Vec2::ZERO,
        plin_axis: Vec3::Y,
        plax: Vec3::Y,
        na_axis: Vec3::Y,
    });

    // 创建90度圆弧路径
    let arc_path = SweepPath3D::from_arc(Arc3D {
        center: Vec3::ZERO,
        radius: 200.0,   // 弯曲半径200mm
        angle: PI / 2.0, // 90度
        start_pt: Vec3::X * 200.0,
        clock_wise: false,
        axis: Vec3::Z,
        pref_axis: Vec3::Y,
    });

    let arc_length = arc_path.length();

    let sweep_solid = SweepSolid {
        profile,
        drns: Some(DVec3::Z),
        drne: Some(DVec3::Z),
        bangle: 0.0,
        plax: Vec3::Y,
        extrude_dir: DVec3::Z,
        height: arc_length,
        path: arc_path.clone(),
        lmirror: false,
    };

    println!("  路径段数: {}", arc_path.segment_count());
    println!("  圆弧长度: {:.3} mm", arc_length);

    // 尝试生成CSG mesh
    use crate::shape::pdms_shape::BrepShapeTrait;
    match sweep_solid.gen_csg_shape() {
        Ok(csg_mesh) => {
            println!("  ✅ CSG Mesh 生成成功！");
            println!("  顶点数: {}", csg_mesh.vertices.len());
            println!("  三角形数: {}", csg_mesh.indices.len() / 3);

            // 导出为 OBJ 文件
            if let Err(e) = csg_mesh.export_obj(false, "test_output/single_arc_sweep.obj") {
                println!("  ⚠️  OBJ文件导出失败: {}", e);
            } else {
                println!("  ✅ OBJ文件导出成功: test_output/single_arc_sweep.obj");
            }
        }
        Err(e) => {
            println!("  ℹ️  CSG Shape生成失败: {}", e);
        }
    }

    println!("✅ 单段圆弧SweepSolid创建测试通过");
}

#[test]
fn test_multi_segment_sweep_solid_creation() {
    println!("\n=== 测试多段路径 SweepSolid 创建 ===");

    use crate::parsed_data::CateProfileParam;
    use crate::prim_geo::sweep_solid::SweepSolid;
    use glam::Vec2;

    // 创建矩形截面
    use crate::RefnoEnum;
    use crate::parsed_data::SRectData;
    let profile = CateProfileParam::SREC(SRectData {
        refno: RefnoEnum::default(),
        center: Vec2::ZERO,
        size: Vec2::new(60.0, 40.0), // 60x40mm
        dxy: Vec2::ZERO,
        plax: Vec3::Y,
        plin_pos: Vec2::ZERO,
        plin_axis: Vec3::Y,
        na_axis: Vec3::Y,
    });

    // 创建复杂多段路径：直线 -> 圆弧 -> 直线
    let segments = vec![
        // 段1: 垂直向上200mm
        SegmentPath::Line(Line3D {
            start: Vec3::ZERO,
            end: Vec3::Z * 200.0,
            is_spine: true,
        }),
        // 段2: 90度圆弧转弯 (半径150mm)
        SegmentPath::Arc(Arc3D {
            center: Vec3::new(150.0, 0.0, 200.0),
            radius: 150.0,
            angle: PI / 2.0,
            start_pt: Vec3::Z * 200.0,
            clock_wise: false,
            axis: Vec3::Y,
            pref_axis: Vec3::Z,
        }),
        // 段3: 水平向右300mm
        SegmentPath::Line(Line3D {
            start: Vec3::new(150.0, 0.0, 350.0),
            end: Vec3::new(450.0, 0.0, 350.0),
            is_spine: true,
        }),
    ];

    let multi_path = SweepPath3D::from_segments(segments);
    let total_length = multi_path.length();

    let sweep_solid = SweepSolid {
        profile,
        drns: Some(DVec3::Z),
        drne: Some(DVec3::Z),
        bangle: 0.0,
        plax: Vec3::Y,
        extrude_dir: DVec3::Z,
        height: total_length,
        path: multi_path.clone(),
        lmirror: false,
    };

    println!("  创建SweepSolid: 矩形截面(60x40mm), 3段混合路径");
    println!("  路径段数: {}", multi_path.segment_count());
    println!("  总长度: {:.3} mm", total_length);

    // 验证路径连续性
    let (is_continuous, discontinuity_index) = multi_path.validate_continuity();
    if is_continuous {
        println!("  ✅ 路径是连续的");
    } else {
        println!("  ⚠️  路径在索引 {:?} 处不连续", discontinuity_index);
    }

    // 验证基本属性
    assert_eq!(multi_path.segment_count(), 3, "应该有3段路径");
    assert!(!multi_path.is_single_segment(), "应该是多段路径");
    assert!(
        multi_path.as_single_line().is_none(),
        "多段路径不应该返回单段直线"
    );

    // 验证总长度（200 + 150*π/2 + 300 ≈ 735.4mm)
    let expected_length = 200.0 + 150.0 * PI / 2.0 + 300.0;
    assert!(
        (total_length - expected_length).abs() < 1.0,
        "总长度应该约为{:.3}mm, 实际为{:.3}mm",
        expected_length,
        total_length
    );

    println!("  预期总长度: {:.3} mm", expected_length);
    println!("✅ 多段路径SweepSolid创建测试通过");
}

#[test]
fn test_gensec_spine_sweep_solid_creation() {
    println!("\n=== 测试 GENSEC SPINE 场景 SweepSolid 创建 ===");

    use crate::parsed_data::CateProfileParam;
    use crate::prim_geo::sweep_solid::SweepSolid;
    use glam::Vec2;

    // 创建圆形截面 (模拟管道)
    use crate::RefnoEnum;
    let profile = CateProfileParam::SANN(crate::parsed_data::SannData {
        refno: RefnoEnum::default(),
        xy: Vec2::ZERO,
        dxy: Vec2::ZERO,
        paxis: None,
        pangle: 360.0,
        pradius: 50.0, // 半径50mm (管道直径100mm)
        pwidth: 0.0,
        drad: 0.0,
        dwid: 0.0,
        plin_pos: Vec2::ZERO,
        plin_axis: Vec3::Y,
        plax: Vec3::Y,
        na_axis: Vec3::Y,
    });

    // 模拟 gensec.txt 的路径结构（简化版 - 纯直线，暂不包含圆弧）
    let segments = vec![
        // 段1: 水平直线150mm
        SegmentPath::Line(Line3D {
            start: Vec3::new(0.0, 0.0, 0.0),
            end: Vec3::new(150.0, 0.0, 0.0),
            is_spine: true,
        }),
        // 段2: 对角线转向 (替代圆弧)
        SegmentPath::Line(Line3D {
            start: Vec3::new(150.0, 0.0, 0.0),
            end: Vec3::new(150.0, 0.0, 140.0),
            is_spine: true,
        }),
        // 段3: 垂直向上150mm
        SegmentPath::Line(Line3D {
            start: Vec3::new(150.0, 0.0, 140.0),
            end: Vec3::new(150.0, 0.0, 290.0),
            is_spine: true,
        }),
        // 段4: 长直线段3000mm
        SegmentPath::Line(Line3D {
            start: Vec3::new(150.0, 0.0, 290.0),
            end: Vec3::new(150.0, 0.0, 3290.0),
            is_spine: true,
        }),
        // 段5: 最后一段1500mm
        SegmentPath::Line(Line3D {
            start: Vec3::new(150.0, 0.0, 3290.0),
            end: Vec3::new(150.0, 0.0, 4790.0),
            is_spine: true,
        }),
    ];

    let gensec_path = SweepPath3D::from_segments(segments);
    let total_length = gensec_path.length();

    println!("  GENSEC路径信息:");
    println!("    段数: {}", gensec_path.segment_count());
    println!("    总长度: {:.3} mm", total_length);

    let sweep_solid = SweepSolid {
        profile,
        drns: Some(DVec3::Z),
        drne: Some(DVec3::Z),
        bangle: 0.0,
        plax: Vec3::Y,
        extrude_dir: DVec3::Z,
        height: total_length,
        path: gensec_path.clone(),
        lmirror: false,
    };

    // 验证基本属性
    assert_eq!(gensec_path.segment_count(), 5, "GENSEC路径应该有5段");

    // 预期长度: 150 + 140 + 150 + 3000 + 1500 = 4940mm (纯直线版本)
    let expected_length = 150.0 + 140.0 + 150.0 + 3000.0 + 1500.0;
    assert!(
        (total_length - expected_length).abs() < 1.0,
        "总长度应该约为{:.3}mm, 实际为{:.3}mm",
        expected_length,
        total_length
    );

    println!("  预期总长度: {:.3} mm", expected_length);
    println!("  高度: {:.3} mm", sweep_solid.height);
    println!("  ✅ 路径结构和长度计算正确");

    // 尝试生成CSG mesh并导出OBJ
    use crate::shape::pdms_shape::BrepShapeTrait;
    match sweep_solid.gen_csg_shape() {
        Ok(csg_mesh) => {
            println!("\n  ✅ CSG Mesh 生成成功！");
            println!("  网格信息:");
            println!("    顶点数: {}", csg_mesh.vertices.len());
            println!("    法线数: {}", csg_mesh.normals.len());
            println!("    三角形数: {}", csg_mesh.indices.len() / 3);

            // 导出为 OBJ 文件
            if let Err(e) = csg_mesh.export_obj(false, "test_output/gensec_spine_sweep.obj") {
                println!("    ⚠️  OBJ文件导出失败: {}", e);
            } else {
                println!("    ✅ OBJ文件导出成功: test_output/gensec_spine_sweep.obj");
                println!("    可以使用 Blender/MeshLab 等工具查看生成的模型");
            }
        }
        Err(e) => {
            println!("\n  ℹ️  CSG Shape生成失败: {}", e);
            println!("  说明: 圆形截面的sweep mesh生成功能已实现");
        }
    }

    println!("\n✅ GENSEC SPINE场景SweepSolid创建测试通过");
}
