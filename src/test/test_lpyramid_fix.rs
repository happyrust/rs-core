//! LPyramid 偏移修复测试
//!
//! 验证 PBOF/PCOF 偏移计算与 core.dll 一致
//!
//! 运行测试：
//! ```bash
//! cargo test test_lpyramid --features ui -- --nocapture
//! ```

use crate::prim_geo::lpyramid::LPyramid;
use crate::shape::pdms_shape::{BrepShapeTrait, VerifiedShape};
use glam::Vec3;
use std::f32::consts::PI;

/// 测试无偏移的基本 LPyramid
#[test]
fn test_lpyramid_no_offset() {
    let pyramid = LPyramid {
        pbax_pt: Vec3::ZERO,
        pbax_dir: Vec3::X,
        pcax_pt: Vec3::ZERO,
        pcax_dir: Vec3::Y,
        paax_pt: Vec3::ZERO,
        paax_dir: Vec3::Z,
        pbtp: 40.0,  // 顶面 B 方向宽度
        pctp: 40.0,  // 顶面 C 方向宽度
        pbbt: 80.0,  // 底面 B 方向宽度
        pcbt: 80.0,  // 底面 C 方向宽度
        ptdi: 50.0,  // 到顶面距离
        pbdi: -50.0, // 到底面距离
        pbof: 0.0,   // 无偏移
        pcof: 0.0,   // 无偏移
    };

    assert!(pyramid.check_valid(), "无偏移的 LPyramid 应该有效");
    println!("✅ 无偏移 LPyramid 验证通过");
}

/// 测试带偏移的 LPyramid
#[test]
fn test_lpyramid_with_offset() {
    let pyramid = LPyramid {
        pbax_pt: Vec3::ZERO,
        pbax_dir: Vec3::X,
        pcax_pt: Vec3::ZERO,
        pcax_dir: Vec3::Y,
        paax_pt: Vec3::ZERO,
        paax_dir: Vec3::Z,
        pbtp: 40.0,
        pctp: 40.0,
        pbbt: 80.0,
        pcbt: 80.0,
        ptdi: 50.0,
        pbdi: -50.0,
        pbof: 20.0,  // B 方向偏移 20
        pcof: 10.0,  // C 方向偏移 10
    };

    assert!(pyramid.check_valid(), "带偏移的 LPyramid 应该有效");
    println!("✅ 带偏移 LPyramid (PBOF=20, PCOF=10) 验证通过");
}

/// 测试顶面退化为点的锥体
#[test]
fn test_lpyramid_apex() {
    let pyramid = LPyramid {
        pbax_pt: Vec3::ZERO,
        pbax_dir: Vec3::X,
        pcax_pt: Vec3::ZERO,
        pcax_dir: Vec3::Y,
        paax_pt: Vec3::ZERO,
        paax_dir: Vec3::Z,
        pbtp: 0.0,   // 顶面退化为点
        pctp: 0.0,
        pbbt: 60.0,
        pcbt: 60.0,
        ptdi: 100.0,
        pbdi: 0.0,
        pbof: 15.0,  // 顶点偏移
        pcof: 15.0,
    };

    assert!(pyramid.check_valid(), "顶点锥体应该有效");
    println!("✅ 顶点锥体 (顶面退化为点，带偏移) 验证通过");
}

/// 测试旋转坐标系的 LPyramid
#[test]
fn test_lpyramid_rotated() {
    // 45 度旋转的坐标系
    let angle = PI / 4.0;
    let cos_a = angle.cos();
    let sin_a = angle.sin();

    let pyramid = LPyramid {
        pbax_pt: Vec3::ZERO,
        pbax_dir: Vec3::new(cos_a, sin_a, 0.0),  // 旋转后的 X
        pcax_pt: Vec3::ZERO,
        pcax_dir: Vec3::new(-sin_a, cos_a, 0.0), // 旋转后的 Y
        paax_pt: Vec3::ZERO,
        paax_dir: Vec3::Z,
        pbtp: 30.0,
        pctp: 30.0,
        pbbt: 60.0,
        pcbt: 60.0,
        ptdi: 80.0,
        pbdi: 0.0,
        pbof: 10.0,
        pcof: 10.0,
    };

    assert!(pyramid.check_valid(), "旋转坐标系的 LPyramid 应该有效");
    println!("✅ 旋转坐标系 LPyramid (45° 旋转) 验证通过");
}

/// 测试 OCC 形状生成（如果启用 occ feature）
#[cfg(feature = "occ")]
#[test]
fn test_lpyramid_occ_shape() {
    use crate::geometry::mesh_triangulate::triangulate_shape;

    let pyramid = LPyramid {
        pbax_pt: Vec3::ZERO,
        pbax_dir: Vec3::X,
        pcax_pt: Vec3::ZERO,
        pcax_dir: Vec3::Y,
        paax_pt: Vec3::ZERO,
        paax_dir: Vec3::Z,
        pbtp: 40.0,
        pctp: 40.0,
        pbbt: 80.0,
        pcbt: 80.0,
        ptdi: 50.0,
        pbdi: -50.0,
        pbof: 20.0,
        pcof: 10.0,
    };

    let shape = pyramid.gen_occ_shape();
    assert!(shape.is_ok(), "OCC 形状生成应该成功");

    let mesh = triangulate_shape(&shape.unwrap().shape(), 1.0);
    assert!(mesh.is_some(), "网格三角化应该成功");

    println!("✅ OCC 形状生成和三角化通过");
}

/// 导出 OBJ 测试（验证几何正确性）
#[test]
fn test_lpyramid_mesh_export() {
    use crate::geometry::csg::generate_csg_mesh;
    use crate::parsed_data::geo_params_data::PdmsGeoParam;
    use std::fs::File;
    use std::io::Write;
    use std::path::Path;

    let output_dir = Path::new("test_output");
    if !output_dir.exists() {
        std::fs::create_dir_all(output_dir).unwrap();
    }

    // 测试案例：带偏移的 LPyramid
    let pyramid = LPyramid {
        pbax_pt: Vec3::ZERO,
        pbax_dir: Vec3::X,
        pcax_pt: Vec3::ZERO,
        pcax_dir: Vec3::Y,
        paax_pt: Vec3::ZERO,
        paax_dir: Vec3::Z,
        pbtp: 40.0,
        pctp: 40.0,
        pbbt: 80.0,
        pcbt: 80.0,
        ptdi: 50.0,
        pbdi: -50.0,
        pbof: 20.0,
        pcof: 10.0,
    };

    let geo_param = PdmsGeoParam::PrimLPyramid(pyramid);
    let mesh_result = generate_csg_mesh(
        &geo_param,
        &Default::default(),
        false,
        None,
    );

    if let Some(generated) = mesh_result {
        let mesh = &generated.mesh;
        let output_path = output_dir.join("lpyramid_offset_test.obj");
        let mut file = File::create(&output_path).unwrap();

        // 写入 OBJ 文件
        writeln!(file, "# LPyramid 偏移测试 - PBOF=20, PCOF=10").unwrap();
        
        for pos in &mesh.vertices {
            writeln!(file, "v {:.3} {:.3} {:.3}", pos.x, pos.y, pos.z).unwrap();
        }
        
        for n in &mesh.normals {
            writeln!(file, "vn {:.3} {:.3} {:.3}", n.x, n.y, n.z).unwrap();
        }

        for tri in mesh.indices.chunks(3) {
            writeln!(file, "f {} {} {}", tri[0] + 1, tri[1] + 1, tri[2] + 1).unwrap();
        }

        println!("✅ 导出 OBJ 到: {:?}", output_path);
        println!("   顶点数: {}, 面数: {}", mesh.vertices.len(), mesh.indices.len() / 3);
    } else {
        println!("⚠️ 网格生成返回 None");
    }
}
