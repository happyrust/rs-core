use crate::*;
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
    println!(
        "🔍 Testing POINSP {} orientation and position",
        poinsp_refno
    );

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

    println!("📋 Owner type: {}", owner_type);
    println!("📋 GENSEC refno: {:?}", gensec_refno);
    println!("📋 SPINE refno: {:?}", spine_refno);

    assert!(spine_refno.is_some(), "Should find a SPINE element");
    let spine_refno = spine_refno.unwrap();
    let spine_att = get_named_attmap(spine_refno).await?;

    // 3. Check YDIR
    let ydir_opt = spine_att.get_dvec3("YDIR");
    assert!(ydir_opt.is_some(), "SPINE should have YDIR");
    let ydir = ydir_opt.unwrap();
    println!("📋 YDIR: {:?}", ydir);

    // 4. Check Spine Points
    let spine_pts = get_spline_pts(gensec_refno).await?;
    assert!(spine_pts.len() >= 2, "SPINE should have at least 2 points");
    println!("📋 Spine points count: {}", spine_pts.len());
    println!("📋 First spine point: {:?}", spine_pts[0]);
    println!("📋 Second spine point: {:?}", spine_pts[1]);

    let spine_dir = (spine_pts[1] - spine_pts[0]).normalize();
    println!("📋 Spine direction: {:?}", spine_dir);

    // 5. Calculate Orientation using fix
    let quat = cal_spine_orientation_basis_with_ydir(spine_dir, Some(ydir), false);
    let calculated_z = quat * glam::DVec3::Z;
    let calculated_y = quat * glam::DVec3::Y;

    println!("📋 Calculated quaternion: {:?}", quat);
    println!("📋 Calculated Z axis: {:?}", calculated_z);
    println!("📋 Calculated Y axis: {:?}", calculated_y);

    // 6. Verify Orientation (Expected values derived from previous successful run)
    // Expected Z: ~ N 0.0451 W 1.0416 D
    // Expected Y: ~ N 88.958 U
    // From previous run:
    // Z: DVec3(-0.0008, 0.9998, -0.0182)
    // Y: DVec3(-0.0000, 0.0182, 0.9998)

    let expected_z = glam::DVec3::new(
        -0.0007869044836398384,
        0.9998344368711255,
        -0.01817909865569267,
    );
    let expected_y = glam::DVec3::new(
        -1.4307578617685256e-5,
        0.01817909302541243,
        0.9998347465316791,
    );

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

    let local_pos = att
        .get_position()
        .expect("POINSP should have POS")
        .as_dvec3();
    println!("📋 POINSP局部位置: {:?}", local_pos);

    // 检查变换链：POINSP -> SPINE -> GENSEC -> 世界
    let spine_mat = get_world_mat4(spine_refno, false)
        .await?
        .expect("SPINE should have world matrix");
    let gensec_mat = get_world_mat4(gensec_refno, false)
        .await?
        .expect("GENSEC should have world matrix");

    println!("📋 SPINE世界矩阵: {:?}", spine_mat);
    println!("📋 GENSEC世界矩阵: {:?}", gensec_mat);

    // 方法1：直接用GENSEC矩阵变换（当前方法）
    let calculated_wpos_method1 = gensec_mat.transform_point3(local_pos);
    println!("📋 方法1 - GENSEC直接变换: {:?}", calculated_wpos_method1);

    // 方法2：先变换到SPINE坐标系，再到世界坐标
    let spine_local_pos = spine_mat.transform_point3(local_pos);
    println!("📋 SPINE变换后位置: {:?}", spine_local_pos);

    // 方法3：检查是否需要考虑SPINE的YDIR旋转
    let spine_quat = cal_spine_orientation_basis_with_ydir(
        (spine_pts[1] - spine_pts[0]).normalize(),
        Some(ydir),
        false,
    );
    let spine_rotation_mat = glam::DMat4::from_rotation_translation(spine_quat, glam::DVec3::ZERO);
    println!("📋 SPINE旋转矩阵: {:?}", spine_rotation_mat);

    let pos_with_spine_rotation = spine_rotation_mat.transform_point3(local_pos);
    let calculated_wpos_method3 = gensec_mat.transform_point3(pos_with_spine_rotation);
    println!("📋 方法3 - 考虑SPINE旋转: {:?}", calculated_wpos_method3);

    let calculated_wpos = calculated_wpos_method1; // 保持原有逻辑

    println!("Calculated WPOS: {:?}", calculated_wpos);
    println!("Expected WPOS: {:?}", expected_pos);

    assert_relative_eq!(calculated_wpos.x, expected_pos.x, epsilon = 0.1); // 0.1mm tolerance
    assert_relative_eq!(calculated_wpos.y, expected_pos.y, epsilon = 0.1);
    assert_relative_eq!(calculated_wpos.z, expected_pos.z, epsilon = 0.1);

    println!("✅ Position Verified");

    Ok(())
}
