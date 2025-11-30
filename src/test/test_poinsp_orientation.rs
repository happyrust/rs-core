use crate::{
    init_test_surreal, get_pe, get_named_attmap, query_filter_deep_children,
    RefU64, RefnoEnum, SUL_DB,
};
use crate::rs_surreal::spatial::{
    get_world_transform, cal_ori_by_z_axis_ref_y,
};
use crate::utils::svg_generator::SpineSvgGenerator;
use serde_json::Value;
use std::collections::HashMap;
use glam::{DVec3, DQuat, Vec3};
use approx::assert_abs_diff_eq;

#[tokio::test]
async fn test_gensec_poinsp_orientation() {
    init_test_surreal().await;

    // RefNo for the POINSP: 17496/266212
    // This POINSP is part of a GENSEC/SPINE structure
    let poinsp_refno = RefU64::from("17496/266212");
    let expected_ori_str = "Y is U and Z is W"; // From user request
    
    println!("Testing POINSP orientation calculation for: {:?}", poinsp_refno);

    // 1. Get attributes to verify it's a POINSP
    let att = get_named_attmap(poinsp_refno.into()).await.expect("Failed to get attributes");
    assert_eq!(att.get_type_str(), "POINSP");
    
    // 2. Get world transform
    let transform_opt = get_world_transform(poinsp_refno.into()).await.expect("Failed to get transform");
    assert!(transform_opt.is_some(), "Should have a transform");
    let transform = transform_opt.unwrap();
    
    let rotation = transform.rotation;
    let translation = transform.translation;
    
    println!("Calculated World Transform:");
    println!("  Translation: {:?}", translation);
    println!("  Rotation (Quat): {:?}", rotation);
    
    // 3. Convert rotation to PDMS string format to verify
    // In PDMS: Y is U (Up/Z) and Z is W (West/-Y)
    // Let's verify the basis vectors
    
    // Global Up (Z) in PDMS is usually Z
    // Global North (Y) is Y
    // Global East (X) is X
    
    // If "Y is U", then local Y axis should point to Global Z (0,0,1)
    // If "Z is W", then local Z axis should point to Global -X (-1,0,0) ??? Wait, W is West?
    // PDMS Coordinates: E(X), N(Y), U(Z)
    // West is -E (-X)
    
    let local_x = rotation * Vec3::X;
    let local_y = rotation * Vec3::Y;
    let local_z = rotation * Vec3::Z;
    
    println!("Local Axes in World Space:");
    println!("  Local X: {:?}", local_x);
    println!("  Local Y: {:?}", local_y);
    println!("  Local Z: {:?}", local_z);
    
    // Expected: Y is U -> local_y should be (0,0,1)
    // Expected: Z is W -> local_z should be (-1,0,0)
    
    // Let's check if this matches the expectation
    // We allow some tolerance
    let tolerance = 0.01;
    
    // Check Y is U
    let dot_y_u = local_y.dot(Vec3::Z);
    println!("  Dot(Local Y, Global U): {}", dot_y_u);
    
    // Check Z is W
    let dot_z_w = local_z.dot(-Vec3::X);
    println!("  Dot(Local Z, Global W): {}", dot_z_w);
    
    // Note: If Y=Z and Z=-X, then X = Y cross Z = Z cross -X = (0,0,1) x (-1,0,0) = (0,-1,0) = South?
    // Let's verify the cross product
    let expected_x = Vec3::Z.cross(-Vec3::X);
    println!("  Expected X (U x W): {:?}", expected_x);
    
    // 4. Also check if we can get the local transform relative to parent
    // This exercises the GensecStrategy directly
    let parent_refno = crate::query_owner(poinsp_refno.into()).await.expect("Should have owner");
    println!("Parent RefNo: {:?}", parent_refno);
    
    let local_mat_opt = crate::transform::get_local_mat4(poinsp_refno.into(), parent_refno).await.expect("Failed to get local mat");
    if let Some(local_mat) = local_mat_opt {
        println!("Local Matrix relative to parent:");
        println!("{:?}", local_mat);
    }
}
