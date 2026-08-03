use aios_core::csg::manifold::ManifoldRust;
use aios_core::geometry::csg::generate_csg_mesh;
use aios_core::mesh_precision::LodMeshSettings;
use aios_core::parsed_data::geo_params_data::PdmsGeoParam;
use aios_core::prim_geo::ctorus::CTorus;
use glam::DMat4;

fn check(label: &str, torus: CTorus) {
    let param = PdmsGeoParam::PrimCTorus(torus.clone());
    let Some(generated) = generate_csg_mesh(&param, &LodMeshSettings::default(), false, true, None)
    else {
        println!("{label}: generate_csg_mesh=None torus={torus:?}");
        return;
    };

    let input_tris = generated.mesh.indices.len() / 3;
    let manifold = ManifoldRust::from_vertices_indices(
        &generated.mesh.vertices,
        &generated.mesh.indices,
        DMat4::IDENTITY,
        false,
    );
    let output = manifold.get_mesh();
    println!(
        "{label}: input_vertices={} input_tris={} output_vertices={} output_tris={} torus={torus:?}",
        generated.mesh.vertices.len(),
        input_tris,
        output.vertices.len() / 3,
        output.indices.len() / 3,
    );

    let mut flipped = generated.mesh.indices.clone();
    for tri in flipped.chunks_exact_mut(3) {
        tri.swap(1, 2);
    }
    let flipped_manifold = ManifoldRust::from_vertices_indices(
        &generated.mesh.vertices,
        &flipped,
        DMat4::IDENTITY,
        false,
    );
    let flipped_output = flipped_manifold.get_mesh();
    println!(
        "{label} flipped: output_vertices={} output_tris={}",
        flipped_output.vertices.len() / 3,
        flipped_output.indices.len() / 3,
    );
}

fn main() {
    check(
        "half-default",
        CTorus {
            rins: 0.61165047,
            rout: 1.0,
            angle: 180.0,
        },
    );
    check(
        "quarter-default",
        CTorus {
            rins: 0.5,
            rout: 1.0,
            angle: 90.0,
        },
    );
    check(
        "full-default",
        CTorus {
            rins: 0.5,
            rout: 1.0,
            angle: 360.0,
        },
    );
}
