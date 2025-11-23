use crate::geometry::csg::generate_csg_mesh;
use crate::mesh_precision::LodMeshSettings;
use crate::parsed_data::geo_params_data::PdmsGeoParam;
use crate::parsed_data::{CateProfileParam, SProfileData};
use crate::prim_geo::spine::{Line3D, SweepPath3D};
use crate::prim_geo::sweep_solid::SweepSolid;
use crate::shape::pdms_shape::VerifiedShape;
use crate::types::refno::RefnoEnum;
/// 测试 bad PrimLoft 的修复方案
///
/// 这个测试验证在修复 ploop-rs 文件 I/O 问题后，
/// 之前失败的几何是否能够成功生成
use glam::{DVec3, Vec2, Vec3};

/// 创建问题案例的 SPRO 截面
fn create_test_spro_profile() -> CateProfileParam {
    let verts = vec![
        Vec2::new(-75.0, 75.0),
        Vec2::new(75.0, 75.0),
        Vec2::new(75.0, 65.0),
        Vec2::new(3.5, 65.0),
        Vec2::new(3.5, -65.0),
        Vec2::new(75.0, -65.0),
        Vec2::new(75.0, -75.0),
        Vec2::new(-75.0, -75.0),
        Vec2::new(-75.0, -65.0),
        Vec2::new(-3.5, -65.0),
        Vec2::new(-3.5, 65.0),
        Vec2::new(-75.0, 65.0),
    ];

    let frads = vec![0.0, 0.0, 0.0, 8.0, 8.0, 0.0, 0.0, 0.0, 0.0, 8.0, 8.0, 0.0];

    CateProfileParam::SPRO(SProfileData {
        refno: RefnoEnum::from("23984_108393"),
        verts,
        frads,
        plax: Vec3::new(0.0, 1.0, 0.0),
        plin_pos: Vec2::ZERO,
        plin_axis: Vec3::new(0.0, 1.0, 0.0),
        na_axis: Vec3::new(0.0, 1.0, 0.0),
    })
}

/// 创建测试用的 SweepSolid（修正路径长度）
fn create_test_sweep_solid() -> SweepSolid {
    let profile = create_test_spro_profile();

    // 使用与 height 一致的路径长度
    let height = 803.92334;
    let path = SweepPath3D::from_line(Line3D {
        start: Vec3::ZERO,
        end: Vec3::new(0.0, 0.0, height),
        is_spine: false,
    });

    SweepSolid {
        profile,
        drns: None,
        drne: None,
        bangle: 0.0,
        plax: Vec3::new(0.0, 1.0, 0.0),
        extrude_dir: DVec3::new(0.0, 0.0, 1.0),
        height,
        path,
        lmirror: false,
    }
}

#[test]
fn test_primloft_fix_validation() {
    println!("\n=== 测试: PrimLoft 修复验证 ===\n");

    let sweep = create_test_sweep_solid();

    // 1. 验证 SweepSolid 参数有效性
    println!("1. 验证 SweepSolid 参数:");
    println!("   height: {}", sweep.height);
    println!("   path.length(): {}", sweep.path.length());
    println!("   extrude_dir: {:?}", sweep.extrude_dir);

    assert!(sweep.check_valid(), "SweepSolid 应该是有效的");
    println!("   ✅ SweepSolid 参数有效\n");

    // 2. 检查路径与高度的一致性
    println!("2. 检查路径与高度一致性:");
    let path_len = sweep.path.length();
    let diff = (path_len - sweep.height).abs();
    let relative_diff = diff / sweep.height;
    println!("   差异: {:.6} ({:.2}%)", diff, relative_diff * 100.0);

    if relative_diff < 0.01 {
        println!("   ✅ 路径长度与高度一致\n");
    } else {
        println!("   ⚠️  路径长度与高度存在差异\n");
    }

    // 3. 尝试生成网格
    println!("3. 尝试生成 CSG 网格:");
    let settings = LodMeshSettings::default();
    let param = PdmsGeoParam::PrimLoft(sweep);

    match generate_csg_mesh(&param, &settings, false, None) {
        Some(generated) => {
            println!("   ✅ 网格生成成功!");
            println!("   顶点数: {}", generated.mesh.vertices.len());
            println!("   索引数: {}", generated.mesh.indices.len());
            println!("   三角形数: {}", generated.mesh.indices.len() / 3);

            if let Some(aabb) = generated.aabb {
                println!("   AABB: min={:?}, max={:?}", aabb.mins, aabb.maxs);
            }

            // 验证网格质量
            assert!(!generated.mesh.vertices.is_empty(), "网格应该有顶点");
            assert!(!generated.mesh.indices.is_empty(), "网格应该有索引");
            assert_eq!(generated.mesh.indices.len() % 3, 0, "索引数应该是 3 的倍数");

            println!("\n   ✅ 所有验证通过!");
        }
        None => {
            println!("   ❌ 网格生成失败!");
            panic!("网格生成应该成功");
        }
    }
}

#[test]
fn test_primloft_without_fradius() {
    println!("\n=== 测试: 无 FRADIUS 的 PrimLoft（对照组）===\n");

    // 创建一个没有 FRADIUS 的简单截面
    let verts = vec![
        Vec2::new(-50.0, 50.0),
        Vec2::new(50.0, 50.0),
        Vec2::new(50.0, -50.0),
        Vec2::new(-50.0, -50.0),
    ];
    let frads = vec![0.0, 0.0, 0.0, 0.0];

    let profile = CateProfileParam::SPRO(SProfileData {
        refno: RefnoEnum::from("test_simple"),
        verts,
        frads,
        plax: Vec3::new(0.0, 1.0, 0.0),
        plin_pos: Vec2::ZERO,
        plin_axis: Vec3::new(0.0, 1.0, 0.0),
        na_axis: Vec3::new(0.0, 1.0, 0.0),
    });

    let height = 100.0;
    let path = SweepPath3D::from_line(Line3D {
        start: Vec3::ZERO,
        end: Vec3::new(0.0, 0.0, height),
        is_spine: false,
    });

    let sweep = SweepSolid {
        profile,
        drns: None,
        drne: None,
        bangle: 0.0,
        plax: Vec3::new(0.0, 1.0, 0.0),
        extrude_dir: DVec3::new(0.0, 0.0, 1.0),
        height,
        path,
        lmirror: false,
    };

    let settings = LodMeshSettings::default();
    let param = PdmsGeoParam::PrimLoft(sweep);

    match generate_csg_mesh(&param, &settings, false, None) {
        Some(generated) => {
            println!("✅ 简单截面网格生成成功");
            println!("   顶点数: {}", generated.mesh.vertices.len());
            println!("   三角形数: {}", generated.mesh.indices.len() / 3);
        }
        None => {
            panic!("简单截面应该能够成功生成网格");
        }
    }
}
