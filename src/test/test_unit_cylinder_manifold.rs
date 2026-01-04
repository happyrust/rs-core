//! 测试单位圆柱体网格的流形性
//!
//! 这个测试用于验证 unit_cylinder_mesh 生成的网格是否满足 Manifold 要求

#[cfg(feature = "gen_model")]
use crate::csg::manifold::ManifoldRust;
use crate::geometry::csg::{unit_box_mesh, unit_cylinder_mesh, unit_sphere_mesh};
use crate::mesh_precision::LodMeshSettings;
use glam::{DMat4, Vec3};

#[test]
fn test_unit_cylinder_topology() {
    let settings = LodMeshSettings::default();
    let mesh = unit_cylinder_mesh(&settings, false);

    println!("单位圆柱体网格统计:");
    println!("  顶点数: {}", mesh.vertices.len());
    println!("  三角形数: {}", mesh.indices.len() / 3);
    println!("  AABB: {:?}", mesh.aabb);

    // 导出为 OBJ 文件用于手动检查
    let obj_path = "test_output/unit_cylinder.obj";
    std::fs::create_dir_all("test_output").ok();
    if let Err(e) = mesh.export_obj(false, obj_path) {
        eprintln!("导出 OBJ 失败: {}", e);
    } else {
        println!("✅ 导出单位圆柱体 OBJ: {}", obj_path);
    }

    // 检查是否有重复顶点
    let mut unique_vertices = std::collections::HashSet::new();
    let mut duplicate_count = 0;
    for v in &mesh.vertices {
        let key = (
            (v.x * 1000000.0) as i64,
            (v.y * 1000000.0) as i64,
            (v.z * 1000000.0) as i64,
        );
        if !unique_vertices.insert(key) {
            duplicate_count += 1;
        }
    }
    println!("  重复顶点数: {}", duplicate_count);

    // 检查三角形是否退化
    let mut degenerate_count = 0;
    for i in (0..mesh.indices.len()).step_by(3) {
        let i0 = mesh.indices[i] as usize;
        let i1 = mesh.indices[i + 1] as usize;
        let i2 = mesh.indices[i + 2] as usize;

        if i0 == i1 || i1 == i2 || i2 == i0 {
            degenerate_count += 1;
            continue;
        }

        let v0 = mesh.vertices[i0];
        let v1 = mesh.vertices[i1];
        let v2 = mesh.vertices[i2];

        let edge1 = v1 - v0;
        let edge2 = v2 - v0;
        let area = edge1.cross(edge2).length() * 0.5;

        if area < 1e-6 {
            degenerate_count += 1;
        }
    }
    println!("  退化三角形数: {}", degenerate_count);

    // 检查三角形法向量一致性（是否都指向外部）
    let center = Vec3::new(0.0, 0.0, 0.5); // 圆柱体中心
    let mut inward_count = 0;
    let mut outward_count = 0;

    for i in (0..mesh.indices.len()).step_by(3) {
        let i0 = mesh.indices[i] as usize;
        let i1 = mesh.indices[i + 1] as usize;
        let i2 = mesh.indices[i + 2] as usize;

        let v0 = mesh.vertices[i0];
        let v1 = mesh.vertices[i1];
        let v2 = mesh.vertices[i2];

        // 计算三角形法向量（通过叉积）
        let edge1 = v1 - v0;
        let edge2 = v2 - v0;
        let normal = edge1.cross(edge2);

        // 计算三角形中心到圆柱体中心的向量
        let tri_center = (v0 + v1 + v2) / 3.0;
        let to_center = center - tri_center;

        // 如果法向量与指向中心的向量同向，说明法向量指向内部
        if normal.dot(to_center) > 0.0 {
            inward_count += 1;
        } else {
            outward_count += 1;
        }
    }
    println!("  法向量指向外部: {}, 指向内部: {}", outward_count, inward_count);
}

#[test]
#[cfg(feature = "gen_model")]
fn test_unit_cylinder_manifold_conversion() {
    use crate::fast_model::export_model::export_glb::export_single_mesh_to_glb;

    let settings = LodMeshSettings::default();
    let mesh = unit_cylinder_mesh(&settings, false);

    // 导出为 GLB
    let temp_dir = std::env::temp_dir();
    let glb_path = temp_dir.join("test_unit_cylinder.glb");

    if let Err(e) = export_single_mesh_to_glb(&mesh, &glb_path) {
        panic!("导出 GLB 失败: {}", e);
    }

    // 从 GLB 加载并转换为 Manifold
    let result = ManifoldRust::import_glb_to_manifold(&glb_path, DMat4::IDENTITY, false);

    // 清理临时文件
    let _ = std::fs::remove_file(&glb_path);

    match result {
        Ok(manifold) => {
            let result_mesh = manifold.get_mesh();
            let output_triangles = result_mesh.indices.len() / 3;

            println!("单位圆柱体 Manifold 转换结果:");
            println!("  输入: {} 顶点, {} 三角形", mesh.vertices.len(), mesh.indices.len() / 3);
            println!("  输出: {} 三角形", output_triangles);

            if output_triangles == 0 {
                panic!("Manifold 转换失败：输出 0 个三角形");
            }
        }
        Err(e) => {
            panic!("从 GLB 加载失败: {}", e);
        }
    }
}

#[test]
fn test_unit_sphere_topology() {
    let mesh = unit_sphere_mesh();

    println!("单位球体网格统计:");
    println!("  顶点数: {}", mesh.vertices.len());
    println!("  三角形数: {}", mesh.indices.len() / 3);

    // 检查重复顶点
    let mut unique_vertices = std::collections::HashSet::new();
    let mut duplicate_count = 0;
    for v in &mesh.vertices {
        let key = (
            (v.x * 1000000.0) as i64,
            (v.y * 1000000.0) as i64,
            (v.z * 1000000.0) as i64,
        );
        if !unique_vertices.insert(key) {
            duplicate_count += 1;
        }
    }
    println!("  重复顶点数: {}", duplicate_count);

    // 检查法向量一致性
    let center = Vec3::ZERO;
    let mut inward_count = 0;
    let mut outward_count = 0;

    for i in (0..mesh.indices.len()).step_by(3) {
        let v0 = mesh.vertices[mesh.indices[i] as usize];
        let v1 = mesh.vertices[mesh.indices[i + 1] as usize];
        let v2 = mesh.vertices[mesh.indices[i + 2] as usize];

        let edge1 = v1 - v0;
        let edge2 = v2 - v0;
        let normal = edge1.cross(edge2);

        let tri_center = (v0 + v1 + v2) / 3.0;
        let to_center = center - tri_center;

        if normal.dot(to_center) > 0.0 {
            inward_count += 1;
        } else {
            outward_count += 1;
        }
    }
    println!("  法向量指向外部: {}, 指向内部: {}", outward_count, inward_count);
}

#[test]
fn test_unit_box_topology() {
    let mesh = unit_box_mesh();

    println!("单位盒子网格统计:");
    println!("  顶点数: {}", mesh.vertices.len());
    println!("  三角形数: {}", mesh.indices.len() / 3);

    // 检查重复顶点
    let mut unique_vertices = std::collections::HashSet::new();
    let mut duplicate_count = 0;
    for v in &mesh.vertices {
        let key = (
            (v.x * 1000000.0) as i64,
            (v.y * 1000000.0) as i64,
            (v.z * 1000000.0) as i64,
        );
        if !unique_vertices.insert(key) {
            duplicate_count += 1;
        }
    }
    println!("  重复顶点数: {}", duplicate_count);

    // 检查法向量一致性
    let center = Vec3::ZERO;
    let mut inward_count = 0;
    let mut outward_count = 0;

    for i in (0..mesh.indices.len()).step_by(3) {
        let v0 = mesh.vertices[mesh.indices[i] as usize];
        let v1 = mesh.vertices[mesh.indices[i + 1] as usize];
        let v2 = mesh.vertices[mesh.indices[i + 2] as usize];

        let edge1 = v1 - v0;
        let edge2 = v2 - v0;
        let normal = edge1.cross(edge2);

        let tri_center = (v0 + v1 + v2) / 3.0;
        let to_center = center - tri_center;

        if normal.dot(to_center) > 0.0 {
            inward_count += 1;
        } else {
            outward_count += 1;
        }
    }
    println!("  法向量指向外部: {}, 指向内部: {}", outward_count, inward_count);
}

#[test]
#[cfg(feature = "gen_model")]
fn test_unit_sphere_manifold_conversion() {
    use crate::fast_model::export_model::export_glb::export_single_mesh_to_glb;

    let mesh = unit_sphere_mesh();
    let temp_dir = std::env::temp_dir();
    let glb_path = temp_dir.join("test_unit_sphere.glb");

    export_single_mesh_to_glb(&mesh, &glb_path).expect("导出 GLB 失败");
    let result = ManifoldRust::import_glb_to_manifold(&glb_path, DMat4::IDENTITY, false);
    let _ = std::fs::remove_file(&glb_path);

    match result {
        Ok(manifold) => {
            let result_mesh = manifold.get_mesh();
            let output_triangles = result_mesh.indices.len() / 3;
            println!("单位球体 Manifold: {} -> {}", mesh.indices.len() / 3, output_triangles);
            assert!(output_triangles > 0, "Manifold 转换失败");
        }
        Err(e) => panic!("从 GLB 加载失败: {}", e),
    }
}

#[test]
#[cfg(feature = "gen_model")]
fn test_unit_box_manifold_conversion() {
    use crate::fast_model::export_model::export_glb::export_single_mesh_to_glb;

    let mesh = unit_box_mesh();
    let temp_dir = std::env::temp_dir();
    let glb_path = temp_dir.join("test_unit_box.glb");

    export_single_mesh_to_glb(&mesh, &glb_path).expect("导出 GLB 失败");
    let result = ManifoldRust::import_glb_to_manifold(&glb_path, DMat4::IDENTITY, false);
    let _ = std::fs::remove_file(&glb_path);

    match result {
        Ok(manifold) => {
            let result_mesh = manifold.get_mesh();
            let output_triangles = result_mesh.indices.len() / 3;
            println!("单位盒子 Manifold: {} -> {}", mesh.indices.len() / 3, output_triangles);
            assert!(output_triangles > 0, "Manifold 转换失败");
        }
        Err(e) => panic!("从 GLB 加载失败: {}", e),
    }
}
