use aios_core::prim_geo::SCTorus;
use glam::Vec3;

fn main() {
    let elbow = SCTorus {
        paax_pt: Vec3::new(0.0, -151.33, 0.0),
        paax_dir: Vec3::new(0.0, -1.0, 0.0),
        pbax_pt: Vec3::ZERO,
        pbax_dir: Vec3::new(0.99999034, 0.004415669, 0.0),
        pdia: 114.0,
    };

    let (torus, transform) = elbow
        .convert_to_ctorus()
        .expect("7997 ELBO SCTO must produce a torus");
    let major_radius = (torus.rins + torus.rout) / 2.0;

    assert!(
        (major_radius - 152.0).abs() < 0.02,
        "major radius: expected 152, got {major_radius}"
    );
    assert!(
        ((torus.rout - torus.rins) - 114.0).abs() < 0.02,
        "diameter: expected 114, got {}",
        torus.rout - torus.rins
    );
    assert!(
        (transform.scale.x - torus.rout).abs() < f32::EPSILON,
        "unit-mesh scale must preserve the outer radius"
    );

    let unit_major_radius = major_radius / torus.rout;
    let angle = torus.angle.to_radians();
    let start = transform.transform_point(Vec3::new(unit_major_radius, 0.0, 0.0));
    let end = transform.transform_point(Vec3::new(
        unit_major_radius * angle.cos(),
        unit_major_radius * angle.sin(),
        0.0,
    ));
    let expected_start = elbow.pbax_pt + elbow.pbax_dir.normalize() * 151.33;
    let (pa_tangent, pb_tangent) = elbow
        .tangent_points()
        .expect("7997 ELBO must expose physical tubing tangents");
    assert!(pa_tangent.distance(elbow.paax_pt) < 0.02);
    assert!(pb_tangent.distance(expected_start) < 0.02);

    assert!(
        start.distance(expected_start) < 0.02,
        "derived B tangent: expected {expected_start:?}, got {start:?}"
    );
    assert!(
        end.distance(elbow.paax_pt) < 0.02,
        "A tangent: expected {:?}, got {end:?}",
        elbow.paax_pt
    );

    println!(
        "ELBO_OK angle={:.6} major_radius={major_radius:.6} rins={:.6} rout={:.6} scale={:.6}",
        torus.angle, torus.rins, torus.rout, transform.scale.x
    );

    let shared_elbow = SCTorus {
        paax_pt: Vec3::new(0.0, -151.243, 0.0),
        paax_dir: Vec3::new(0.0, -1.0, 0.0),
        pbax_pt: Vec3::ZERO,
        pbax_dir: Vec3::new(0.99998754, 0.004991621, 0.0),
        pdia: 114.0,
    };
    let (shared_torus, _) = shared_elbow
        .convert_to_ctorus()
        .expect("shared 7997 ELBO SCTO must produce a torus");
    let shared_major_radius = (shared_torus.rins + shared_torus.rout) / 2.0;
    assert!(
        (shared_major_radius - 152.0).abs() < 0.02,
        "shared ELBO major radius: expected 152, got {shared_major_radius}"
    );
}
