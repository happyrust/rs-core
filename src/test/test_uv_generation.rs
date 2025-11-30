use crate::geometry::csg::{unit_box_mesh, unit_cylinder_mesh, unit_sphere_mesh};
use crate::mesh_precision::LodMeshSettings;
use crate::shape::pdms_shape::PlantMesh;
use glam::Vec3;

fn assert_uvs_normalized(mesh: &PlantMesh) {
    assert_eq!(mesh.uvs.len(), mesh.vertices.len());
    for uv in &mesh.uvs {
        assert!(uv[0].is_finite());
        assert!(uv[1].is_finite());
        assert!(uv[0] >= -1.0e-3 && uv[0] <= 1.0 + 1.0e-3);
        assert!(uv[1] >= -1.0e-3 && uv[1] <= 1.0 + 1.0e-3);
    }
}

#[test]
fn test_generate_auto_uvs_basic_projection() {
    let mut mesh = PlantMesh {
        indices: Vec::new(),
        vertices: vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(2.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ],
        normals: Vec::new(),
        uvs: Vec::new(),
        wire_vertices: Vec::new(),
        edges: Vec::new(),
        aabb: None,
    };

    mesh.generate_auto_uvs();

    assert_eq!(mesh.uvs.len(), mesh.vertices.len());
    let eps = 1.0e-6;
    assert!((mesh.uvs[0][0] - 0.0).abs() < eps);
    assert!((mesh.uvs[0][1] - 0.0).abs() < eps);
    assert!((mesh.uvs[1][0] - 1.0).abs() < eps);
    assert!((mesh.uvs[1][1] - 0.0).abs() < eps);
    assert!((mesh.uvs[2][0] - 0.0).abs() < eps);
    assert!((mesh.uvs[2][1] - 1.0).abs() < eps);
}

#[test]
fn test_unit_box_mesh_has_uvs() {
    let mesh = unit_box_mesh();
    assert_uvs_normalized(&mesh);
}

#[test]
fn test_unit_sphere_mesh_has_uvs() {
    let mesh = unit_sphere_mesh();
    assert_uvs_normalized(&mesh);
}

#[test]
fn test_unit_cylinder_mesh_has_uvs() {
    let settings = LodMeshSettings::default();
    let mesh = unit_cylinder_mesh(&settings, false);
    assert_uvs_normalized(&mesh);
}
