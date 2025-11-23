use aios_core::*;
use anyhow::Result;
use approx::assert_relative_eq;

#[tokio::test]
async fn test_poinsp_17496_266220_orientation_detailed() -> Result<()> {
    // Initialize database connection (using test init)
    // Note: Ensure your test environment DB is reachable or mock it. 
    // For this specific "live" check, we assume SUL_DB is available as in the example.
    // In a real CI environment, you might need `init_test_surreal()`.
    init_surreal().await?;
    
    let poinsp_refno = RefnoEnum::from("17496_266220");
    println!("🔍 Testing POINSP {} orientation and position", poinsp_refno);
    
    // 1. Fetch Attributes
    let att = get_named_attmap(poinsp_refno).await?;
    
    // 2. Resolve Hierarchy
    let owner_refno = att.get_owner();
    let owner_att = get_named_attmap(owner_refno).await?;
    let owner_type = owner_att.get_type_str();
    
    let (gensec_refno, spine_refno) = if owner_type == "SPINE" {
        (owner_att.get_owner(), Some(owner_refno))
    } else if owner_type == "GENSEC" || owner_type == "WALL" {
         let gensec_children = get_children_refnos(owner_refno).await?;
         let s_ref = gensec_children.into_iter().find(|&r| {
             // Simplified check: assume we find a spine
             // In a robust test we'd check type, but for this specific case it's fine
             true 
         });
         // We need to iterate to find the one with type SPINE really
         // But let's reuse the logic that worked
         let mut real_spine = None;
         for &child in &get_children_refnos(owner_refno).await? {
             let ca = get_named_attmap(child).await?;
             if ca.get_type_str() == "SPINE" {
                 real_spine = Some(child);
                 break;
             }
         }
         (owner_refno, real_spine)
    } else {
        (owner_refno, None)
    };
    
    assert!(spine_refno.is_some(), "Should find a SPINE element");
    let spine_refno = spine_refno.unwrap();
    let spine_att = get_named_attmap(spine_refno).await?;
    
    // 3. Check YDIR
    let ydir_opt = spine_att.get_dvec3("YDIR");
    assert!(ydir_opt.is_some(), "SPINE should have YDIR");
    let ydir = ydir_opt.unwrap();
    
    // 4. Check Spine Points
    let spine_pts = get_spline_pts(gensec_refno).await?;
    assert!(spine_pts.len() >= 2, "SPINE should have at least 2 points");
    
    let spine_dir = (spine_pts[1] - spine_pts[0]).normalize();
    
    // 5. Calculate Orientation using fix
    let quat = cal_spine_orientation_basis_with_ydir(spine_dir, Some(ydir), false);
    let calculated_z = quat * glam::DVec3::Z;
    let calculated_y = quat * glam::DVec3::Y;
    
    // 6. Verify Orientation (Expected values derived from previous successful run)
    // Expected Z: ~ N 0.0451 W 1.0416 D
    // Expected Y: ~ N 88.958 U
    // From previous run:
    // Z: DVec3(-0.0008, 0.9998, -0.0182)
    // Y: DVec3(-0.0000, 0.0182, 0.9998)
    
    let expected_z = glam::DVec3::new(-0.0007869044836398384, 0.9998344368711255, -0.01817909865569267);
    let expected_y = glam::DVec3::new(-1.4307578617685256e-5, 0.01817909302541243, 0.9998347465316791);
    
    assert_relative_eq!(calculated_z.x, expected_z.x, epsilon = 1e-6);
    assert_relative_eq!(calculated_z.y, expected_z.y, epsilon = 1e-6);
    assert_relative_eq!(calculated_z.z, expected_z.z, epsilon = 1e-6);
    
    assert_relative_eq!(calculated_y.x, expected_y.x, epsilon = 1e-6);
    assert_relative_eq!(calculated_y.y, expected_y.y, epsilon = 1e-6);
    assert_relative_eq!(calculated_y.z, expected_y.z, epsilon = 1e-6);
    
    println!("✅ Orientation Verified");

    // 7. Verify Position
    // Expected: W 5375.49mm N 1771.29mm D 2607.01mm
    let expected_pos = glam::DVec3::new(-5375.49, 1771.29, -2607.01);
    
    let local_pos = att.get_position().expect("POINSP should have POS").as_dvec3();
    let gensec_mat = get_world_mat4(gensec_refno, false).await?.expect("GENSEC should have world matrix");
    
    let calculated_wpos = gensec_mat.transform_point3(local_pos);
    
    println!("Calculated WPOS: {:?}", calculated_wpos);
    println!("Expected WPOS: {:?}", expected_pos);
    
    assert_relative_eq!(calculated_wpos.x, expected_pos.x, epsilon = 0.1); // 0.1mm tolerance
    assert_relative_eq!(calculated_wpos.y, expected_pos.y, epsilon = 0.1);
    assert_relative_eq!(calculated_wpos.z, expected_pos.z, epsilon = 0.1);
    
    println!("✅ Position Verified");
    
    Ok(())
}
