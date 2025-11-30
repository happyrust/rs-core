use crate::parsed_data::geo_params_data::PdmsGeoParam;
use crate::parsed_data::{CateProfileParam, SProfileData};
use crate::prim_geo::spine::{Line3D, SweepPath3D};
use crate::prim_geo::sweep_solid::SweepSolid;
use crate::prim_geo::wire::{gen_polyline_from_processed_vertices, process_ploop_vertices};
use crate::types::refno::RefnoEnum;
/// 简化版 bad PrimLoft 测试（避免依赖有编译错误的模块）
use glam::{Vec2, Vec3};

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

#[test]
fn test_bad_primloft_ploop_processing() {
    println!("\n=== 测试: PLOOP 处理 (ploop-rs) ===");

    let profile = create_bad_spro_profile();

    if let CateProfileParam::SPRO(spro) = &profile {
        println!("SPRO 截面信息:");
        println!("  顶点数: {}", spro.verts.len());
        println!("  FRADIUS 数: {}", spro.frads.len());

        // 分析几何属性
        println!("\n边长分析:");
        let mut min_edge_len = f32::MAX;
        for i in 0..spro.verts.len() {
            let curr = spro.verts[i];
            let next = spro.verts[(i + 1) % spro.verts.len()];
            let edge_len = curr.distance(next);
            min_edge_len = min_edge_len.min(edge_len);

            if i < 4 || spro.frads[i] > 0.0 || spro.frads[(i + 1) % spro.verts.len()] > 0.0 {
                println!(
                    "  边 {} -> {}: 长度 = {:.2} mm",
                    i,
                    (i + 1) % spro.verts.len(),
                    edge_len
                );
            }
        }

        println!("\n最小边长: {:.2} mm", min_edge_len);

        // 检查 FRADIUS 是否过大
        println!("\nFRADIUS 分析:");
        for (i, &frad) in spro.frads.iter().enumerate() {
            if frad > 0.0 {
                let prev_idx = if i == 0 { spro.verts.len() - 1 } else { i - 1 };
                let next_idx = (i + 1) % spro.verts.len();

                let edge1_len = spro.verts[prev_idx].distance(spro.verts[i]);
                let edge2_len = spro.verts[i].distance(spro.verts[next_idx]);

                println!("  顶点 {}: FRADIUS = {:.2} mm", i, frad);
                println!("    前一条边长度: {:.2} mm", edge1_len);
                println!("    后一条边长度: {:.2} mm", edge2_len);
                println!("    FRADIUS / 前边长 = {:.2}", frad / edge1_len);
                println!("    FRADIUS / 后边长 = {:.2}", frad / edge2_len);

                if frad * 2.0 > edge1_len {
                    println!(
                        "    ⚠️ 警告: FRADIUS 的 2 倍 ({:.2}) 大于前一条边 ({:.2})!",
                        frad * 2.0,
                        edge1_len
                    );
                }
                if frad * 2.0 > edge2_len {
                    println!(
                        "    ⚠️ 警告: FRADIUS 的 2 倍 ({:.2}) 大于后一条边 ({:.2})!",
                        frad * 2.0,
                        edge2_len
                    );
                }
            }
        }

        // 尝试使用 ploop-rs 处理
        println!("\n尝试使用 ploop-rs 处理截面...");
        match process_ploop_vertices(&spro.verts, &spro.frads, "BAD_PRIMLOFT_TEST") {
            Ok(processed) => {
                println!("✅ ploop-rs 处理成功!");
                println!("  处理后顶点数: {}", processed.len());

                // 尝试生成 Polyline
                println!("\n尝试生成 Polyline...");
                match gen_polyline_from_processed_vertices(&processed, None) {
                    Ok(polyline) => {
                        println!("✅ Polyline 生成成功!");
                        println!("  Polyline 顶点数: {}", polyline.vertex_data.len());

                        // 检查自相交
                        use crate::prim_geo::wire::check_wire_ok;

                        // 将 Polyline 转换为 Vec3 格式（z 分量为 fradius）
                        let wire_verts: Vec<Vec3> = polyline
                            .vertex_data
                            .iter()
                            .map(|v| Vec3::new(v.x as f32, v.y as f32, v.bulge as f32))
                            .collect();
                        let wire_frads: Vec<f32> = polyline
                            .vertex_data
                            .iter()
                            .map(|v| v.bulge as f32)
                            .collect();

                        if check_wire_ok(&wire_verts, &wire_frads) {
                            println!("✅ 轮廓无自相交");
                        } else {
                            println!("❌ 轮廓存在自相交!");
                        }
                    }
                    Err(e) => {
                        println!("❌ Polyline 生成失败: {}", e);
                    }
                }
            }
            Err(e) => {
                println!("❌ ploop-rs 处理失败: {}", e);
                println!("   这可能是导致 bad 几何的根本原因!");
            }
        }
    }
}
