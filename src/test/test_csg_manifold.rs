//! CSG 网格 Manifold 流形性测试
//!
//! 本测试模块验证各种 CSG 生成函数生成的网格是否满足 Manifold 库的流形性要求。
//!
//! 流形性要求包括：
//! - 边流形性：每条边最多被 2 个三角形共享
//! - 顶点流形性：顶点周围的三角形形成连续的扇形
//! - 封闭性：网格必须是封闭的（无边界边）
//! - 方向一致性：所有三角形法向量方向一致

#[cfg(feature = "gen_model")]
use crate::csg::manifold::ManifoldRust;
use crate::geometry::csg::build_csg_mesh;
use crate::mesh_precision::LodMeshSettings;
use crate::prim_geo::*;
use crate::types::refno::RefnoEnum;
use glam::{Vec3, DMat4};
use std::path::Path;

// 直接使用 prim_geo 中的类型构建参数
use crate::parsed_data::geo_params_data::PdmsGeoParam;

/// Manifold 流形性验证结果
#[derive(Debug)]
struct ManifoldValidationResult {
    /// 是否成功转换为 Manifold
    success: bool,
    /// 输入顶点数
    input_vertices: usize,
    /// 输入三角形数
    input_triangles: usize,
    /// 输出三角形数
    output_triangles: usize,
    /// 错误信息
    error_message: Option<String>,
}

impl ManifoldValidationResult {
    fn success(input_vertices: usize, input_triangles: usize, output_triangles: usize) -> Self {
        Self {
            success: true,
            input_vertices,
            input_triangles,
            output_triangles,
            error_message: None,
        }
    }

    fn failure(input_vertices: usize, input_triangles: usize, error: String) -> Self {
        Self {
            success: false,
            input_vertices,
            input_triangles,
            output_triangles: 0,
            error_message: Some(error),
        }
    }
}

/// 验证 PlantMesh 通过 GLB 文件加载后是否能成功转换为 Manifold
///
/// 这个测试模拟真实的布尔运算流程：
/// 1. 将 PlantMesh 导出为 GLB 文件
/// 2. 从 GLB 文件加载
/// 3. 转换为 Manifold
fn validate_mesh_via_glb(mesh: &crate::shape::pdms_shape::PlantMesh, test_name: &str) -> ManifoldValidationResult {
    use crate::fast_model::export_model::export_glb::export_single_mesh_to_glb;

    let input_vertices = mesh.vertices.len();
    let input_triangles = mesh.indices.len() / 3;

    if mesh.vertices.is_empty() || mesh.indices.is_empty() {
        return ManifoldValidationResult::failure(
            input_vertices,
            input_triangles,
            "网格为空".to_string(),
        );
    }

    // 1. 导出为 GLB 文件
    let temp_dir = std::env::temp_dir();
    let glb_path = temp_dir.join(format!("test_csg_{}.glb", test_name));

    if let Err(e) = export_single_mesh_to_glb(mesh, &glb_path) {
        return ManifoldValidationResult::failure(
            input_vertices,
            input_triangles,
            format!("导出 GLB 失败: {}", e),
        );
    }

    // 2. 从 GLB 文件加载并转换为 Manifold
    let result = ManifoldRust::import_glb_to_manifold(&glb_path, DMat4::IDENTITY, false);

    // 清理临时文件
    let _ = std::fs::remove_file(&glb_path);

    match result {
        Ok(manifold) => {
            let result_mesh = manifold.get_mesh();
            let output_triangles = result_mesh.indices.len() / 3;

            if output_triangles == 0 {
                ManifoldValidationResult::failure(
                    input_vertices,
                    input_triangles,
                    "Manifold 转换失败：输出 0 个三角形".to_string(),
                )
            } else {
                ManifoldValidationResult::success(input_vertices, input_triangles, output_triangles)
            }
        }
        Err(e) => ManifoldValidationResult::failure(
            input_vertices,
            input_triangles,
            format!("从 GLB 加载失败: {}", e),
        ),
    }
}
// ============================================================================
// 圆柱体（Cylinder）测试
// ============================================================================

#[test]
fn test_scylinder_manifold() {
    let settings = LodMeshSettings::default();

    // 测试标准圆柱体
    let cyl = SCylinder {
        pdia: 100.0,  // 直径 100mm
        phei: 200.0,  // 高度 200mm
        ..Default::default()
    };

    let param = PdmsGeoParam::PrimSCylinder(cyl);
    let result = build_csg_mesh(&param, &settings, false, RefnoEnum::default());

    assert!(result.is_some(), "圆柱体网格生成失败");
    let mesh = result.unwrap().mesh;

    let validation = validate_mesh_via_glb(&mesh, "scylinder");
    println!("圆柱体 Manifold 验证: {:?}", validation);

    assert!(
        validation.success,
        "圆柱体网格不满足 Manifold 流形性要求: {:?}",
        validation.error_message
    );
}

#[test]
fn test_lcylinder_manifold() {
    let settings = LodMeshSettings::default();

    // 测试长圆柱体
    let cyl = LCylinder {
        paxi_expr: String::new(),
        paxi_pt: Vec3::ZERO,
        paxi_dir: Vec3::Z,
        pbdi: 0.0,      // 底部距离
        ptdi: 300.0,    // 顶部距离（高度）
        pdia: 50.0,     // 直径
        negative: false,
        centre_line_flag: false,
    };

    let param = PdmsGeoParam::PrimLCylinder(cyl);
    let result = build_csg_mesh(&param, &settings, false, RefnoEnum::default());

    assert!(result.is_some(), "长圆柱体网格生成失败");
    let mesh = result.unwrap().mesh;

    let validation = validate_mesh_via_glb(&mesh, "lcylinder");
    println!("长圆柱体 Manifold 验证: {:?}", validation);

    assert!(
        validation.success,
        "长圆柱体网格不满足 Manifold 流形性要求: {:?}",
        validation.error_message
    );
}

// ============================================================================
// 圆台（Snout）测试
// ============================================================================

#[test]
fn test_snout_manifold() {
    let settings = LodMeshSettings::default();

    // 测试圆台（底部直径 > 顶部直径）
    let snout = LSnout {
        pbdm: 100.0,  // 底部直径
        ptdm: 50.0,   // 顶部直径
        paax_pt: Vec3::ZERO,
        paax_dir: Vec3::Z,
        paax_expr: String::new(),
        pbax_pt: Vec3::ZERO,
        pbax_dir: Vec3::X,
        pbax_expr: String::new(),
        pbdi: 0.0,
        ptdi: 200.0,
        poff: 0.0,
        btm_on_top: false,
    };

    let param = PdmsGeoParam::PrimLSnout(snout);
    let result = build_csg_mesh(&param, &settings, false, RefnoEnum::default());

    assert!(result.is_some(), "圆台网格生成失败");
    let mesh = result.unwrap().mesh;

    let validation = validate_mesh_via_glb(&mesh, "lcylinder");
    println!("圆台 Manifold 验证: {:?}", validation);

    assert!(
        validation.success,
        "圆台网格不满足 Manifold 流形性要求: {:?}",
        validation.error_message
    );
}

#[test]
fn test_cone_manifold() {
    let settings = LodMeshSettings::default();

    // 测试圆锥（顶部直径为 0）
    let cone = LSnout {
        pbdm: 100.0,  // 底部直径
        ptdm: 0.0,    // 顶部直径为 0（圆锥）
        paax_pt: Vec3::ZERO,
        paax_dir: Vec3::Z,
        paax_expr: String::new(),
        pbax_pt: Vec3::ZERO,
        pbax_dir: Vec3::X,
        pbax_expr: String::new(),
        pbdi: 0.0,
        ptdi: 150.0,
        poff: 0.0,
        btm_on_top: false,
    };

    let param = PdmsGeoParam::PrimLSnout(cone);
    let result = build_csg_mesh(&param, &settings, false, RefnoEnum::default());

    assert!(result.is_some(), "圆锥网格生成失败");
    let mesh = result.unwrap().mesh;

    let validation = validate_mesh_via_glb(&mesh, "lcylinder");
    println!("圆锥 Manifold 验证: {:?}", validation);

    assert!(
        validation.success,
        "圆锥网格不满足 Manifold 流形性要求: {:?}",
        validation.error_message
    );
}

// ============================================================================
// 旋转体（Revolution）测试
// ============================================================================

#[test]
fn test_revolution_manifold() {
    let settings = LodMeshSettings::default();

    // 测试简单旋转体（矩形轮廓）
    let rev = Revolution {
        verts: vec![vec![
            Vec3::new(0.0, 20.0, 0.0),   // 内径 20mm
            Vec3::new(0.0, 50.0, 0.0),   // 外径 50mm
            Vec3::new(100.0, 50.0, 0.0), // 高度 100mm
            Vec3::new(100.0, 20.0, 0.0),
        ]],
        angle: 360.0,  // 完整旋转
        ..Default::default()
    };

    let param = PdmsGeoParam::PrimRevolution(rev);
    let result = build_csg_mesh(&param, &settings, false, RefnoEnum::default());

    assert!(result.is_some(), "旋转体网格生成失败");
    let mesh = result.unwrap().mesh;

    let validation = validate_mesh_via_glb(&mesh, "lcylinder");
    println!("旋转体 Manifold 验证: {:?}", validation);

    assert!(
        validation.success,
        "旋转体网格不满足 Manifold 流形性要求: {:?}",
        validation.error_message
    );
}

// ============================================================================
// 拉伸体（Extrusion）测试
// ============================================================================

#[test]
fn test_extrusion_manifold() {
    let settings = LodMeshSettings::default();

    // 测试矩形拉伸体
    let extrusion = Extrusion {
        verts: vec![vec![
            Vec3::new(-50.0, -50.0, 0.0),
            Vec3::new(50.0, -50.0, 0.0),
            Vec3::new(50.0, 50.0, 0.0),
            Vec3::new(-50.0, 50.0, 0.0),
        ]],
        height: 100.0,
        cur_type: crate::prim_geo::wire::CurveType::Fill,
    };

    let param = PdmsGeoParam::PrimExtrusion(extrusion);
    let result = build_csg_mesh(&param, &settings, false, RefnoEnum::default());

    assert!(result.is_some(), "拉伸体网格生成失败");
    let mesh = result.unwrap().mesh;

    let validation = validate_mesh_via_glb(&mesh, "lcylinder");
    println!("拉伸体 Manifold 验证: {:?}", validation);

    assert!(
        validation.success,
        "拉伸体网格不满足 Manifold 流形性要求: {:?}",
        validation.error_message
    );
}
