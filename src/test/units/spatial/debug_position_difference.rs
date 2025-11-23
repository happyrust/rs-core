use crate::*;
use anyhow::Result;

#[tokio::test]
async fn debug_position_difference() -> Result<()> {
    init_surreal().await?;
    
    let poinsp_refno = RefnoEnum::from("17496_266220");
    println!("🔍 调试位置差异分析");
    
    // 方法1：通用测试的方法（直接调用 get_world_mat4(poinsp_refno)）
    println!("\n--- 方法1：通用测试方法 ---");
    if let Some(world_matrix_direct) = get_world_mat4(poinsp_refno, false).await? {
        let pos_direct = world_matrix_direct.transform_point3(glam::DVec3::ZERO);
        println!("直接 get_world_mat4(poinsp_refno): {:?}", pos_direct);
    }
    
    // 方法2：专门测试的方法（通过 GENSEC 计算）
    println!("\n--- 方法2：专门测试方法 ---");
    let att = get_named_attmap(poinsp_refno).await?;
    let local_pos = att.get_position().expect("POINSP should have POS").as_dvec3();
    println!("POINSP 本地位置: {:?}", local_pos);
    
    let owner_refno = att.get_owner();
    let owner_att = get_named_attmap(owner_refno).await?;
    let owner_type = owner_att.get_type_str();
    println!("所有者类型: {}", owner_type);
    println!("所有者 RefNo: {:?}", owner_refno);
    
    let gensec_refno = if owner_type == "SPINE" {
        owner_att.get_owner()
    } else {
        owner_refno
    };
    
    println!("GENSEC RefNo: {:?}", gensec_refno);
    
    if let Some(gensec_mat) = get_world_mat4(gensec_refno, false).await? {
        let pos_via_gensec = gensec_mat.transform_point3(local_pos);
        println!("通过 GENSEC 计算的位置: {:?}", pos_via_gensec);
    }
    
    // 检查两个矩阵
    println!("\n--- 矩阵对比 ---");
    if let Some(poinsp_matrix) = get_world_mat4(poinsp_refno, false).await? {
        println!("POINSP 世界矩阵:");
        println!("  位置: {:?}", poinsp_matrix.transform_point3(glam::DVec3::ZERO));
        println!("  X轴: {:?}", poinsp_matrix.transform_vector3(glam::DVec3::X));
        println!("  Y轴: {:?}", poinsp_matrix.transform_vector3(glam::DVec3::Y));
        println!("  Z轴: {:?}", poinsp_matrix.transform_vector3(glam::DVec3::Z));
    }
    
    if let Some(gensec_matrix) = get_world_mat4(gensec_refno, false).await? {
        println!("\nGENSEC 世界矩阵:");
        println!("  位置: {:?}", gensec_matrix.transform_point3(glam::DVec3::ZERO));
        println!("  X轴: {:?}", gensec_matrix.transform_vector3(glam::DVec3::X));
        println!("  Y轴: {:?}", gensec_matrix.transform_vector3(glam::DVec3::Y));
        println!("  Z轴: {:?}", gensec_matrix.transform_vector3(glam::DVec3::Z));
    }
    
    Ok(())
}
