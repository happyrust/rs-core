use crate::geometry::csg::generate_csg_mesh;
use crate::mesh_precision::LodMeshSettings;
use crate::parsed_data::geo_params_data::PdmsGeoParam;
use crate::parsed_data::{CateProfileParam, SProfileData};
use crate::prim_geo::profile_processor::ProfileProcessor;
use crate::prim_geo::spine::{Line3D, SegmentPath, SweepPath3D};
use crate::prim_geo::sweep_solid::SweepSolid;
use crate::types::refno::RefnoEnum;
/// 测试 bad PrimLoft 几何生成问题
///
/// 问题案例：inst_geo:⟨14275436081664024447⟩
///
/// 可能的问题：
/// 1. SPRO 截面自相交
/// 2. FRADIUS 处理失败（圆角半径过大）
/// 3. 路径与高度不匹配
/// 4. ProfileProcessor 处理失败
use glam::{DVec3, Vec2, Vec3};

/// 创建问题案例的 SPRO 截面
fn create_bad_spro_profile() -> CateProfileParam {
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

/// 创建问题案例的 SweepSolid
fn create_bad_sweep_solid() -> SweepSolid {
    let profile = create_bad_spro_profile();

    let path = SweepPath3D::from_line(Line3D {
        start: Vec3::ZERO,
        end: Vec3::new(0.0, 0.0, 10.0),
        is_spine: false,
    });

    SweepSolid {
        profile,
        drns: None,
        drne: None,
        plax: Vec3::new(0.0, 1.0, 0.0),
        bangle: 0.0,
        extrude_dir: DVec3::new(0.0, 0.0, 1.0),
        height: 803.92334,
        path,
        lmirror: false,
        spine_segments: vec![],
        segment_transforms: vec![],
    }
}

#[test]
fn test_bad_primloft_profile_only() {
    println!("\n=== 测试 1: 仅测试 SPRO 截面处理 ===");

    let profile = create_bad_spro_profile();

    if let CateProfileParam::SPRO(spro) = &profile {
        println!("SPRO 截面信息:");
        println!("  顶点数: {}", spro.verts.len());
        println!("  FRADIUS 数: {}", spro.frads.len());
        println!("  顶点: {:?}", spro.verts);
        println!("  FRADIUS: {:?}", spro.frads);

        // 检查基本几何属性
        let mut min_edge_len = f32::MAX;
        for i in 0..spro.verts.len() {
            let curr = spro.verts[i];
            let next = spro.verts[(i + 1) % spro.verts.len()];
            let edge_len = curr.distance(next);
            min_edge_len = min_edge_len.min(edge_len);
            println!(
                "  边 {}->{}: 长度 = {:.2}",
                i,
                (i + 1) % spro.verts.len(),
                edge_len
            );
        }

        println!("\n最小边长: {:.2}", min_edge_len);

        // 检查 FRADIUS 是否过大
        for (i, &frad) in spro.frads.iter().enumerate() {
            if frad > 0.0 {
                let prev_idx = if i == 0 { spro.verts.len() - 1 } else { i - 1 };
                let next_idx = (i + 1) % spro.verts.len();

                let edge1_len = spro.verts[prev_idx].distance(spro.verts[i]);
                let edge2_len = spro.verts[i].distance(spro.verts[next_idx]);

                println!("\n顶点 {} 的 FRADIUS = {:.2}", i, frad);
                println!("  前一条边长度: {:.2}", edge1_len);
                println!("  后一条边长度: {:.2}", edge2_len);

                if frad * 2.0 > edge1_len || frad * 2.0 > edge2_len {
                    println!("  ⚠️ 警告: FRADIUS 可能过大!");
                }
            }
        }

        // 尝试使用 ProfileProcessor 处理
        println!("\n尝试使用 ProfileProcessor 处理截面...");
        let verts2d = vec![spro.verts.clone()];
        let frads = vec![spro.frads.clone()];

        match ProfileProcessor::from_wires(verts2d, frads, true) {
            Ok(processor) => {
                println!("✅ ProfileProcessor 创建成功");

                match processor.process("BAD_PRIMLOFT_TEST", Some("23984_108393")) {
                    Ok(processed) => {
                        println!("✅ 截面处理成功!");
                        println!("  轮廓点数: {}", processed.contour_points.len());
                        println!("  三角形数: {}", processed.tri_indices.len() / 3);
                    }
                    Err(e) => {
                        println!("❌ 截面处理失败: {}", e);
                    }
                }
            }
            Err(e) => {
                println!("❌ ProfileProcessor 创建失败: {}", e);
            }
        }
    }
}

#[test]
fn test_bad_primloft_full_generation() {
    println!("\n=== 测试 2: 完整 PrimLoft 网格生成 ===");

    let sweep = create_bad_sweep_solid();
    let settings = LodMeshSettings::default();

    println!("SweepSolid 参数:");
    println!("  height: {}", sweep.height);
    println!("  path length: {}", sweep.path.length());
    println!("  lmirror: {}", sweep.lmirror);

    // 检查路径与高度的一致性
    if (sweep.height - sweep.path.length()).abs() > 0.01 {
        println!(
            "⚠️ 警告: height ({}) 与 path.length() ({}) 不一致!",
            sweep.height,
            sweep.path.length()
        );
    }

    println!("\n尝试生成 CSG 网格...");
    match generate_csg_mesh(&PdmsGeoParam::PrimLoft(sweep), &settings, false, None) {
        Some(generated) => {
            println!("✅ 网格生成成功!");
            println!("  顶点数: {}", generated.mesh.vertices.len());
            println!("  索引数: {}", generated.mesh.indices.len());
            println!("  三角形数: {}", generated.mesh.indices.len() / 3);
            if let Some(aabb) = generated.aabb {
                println!("  AABB: {:?}", aabb);
            }
        }
        None => {
            println!("❌ 网格生成失败 (返回 None)");
        }
    }
}
