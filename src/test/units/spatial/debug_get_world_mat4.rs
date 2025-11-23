use crate::*;
use anyhow::Result;

#[tokio::test]
async fn debug_get_world_mat4_internal() -> Result<()> {
    init_surreal().await?;
    
    let poinsp_refno = RefnoEnum::from("17496_266220");
    println!("🔍 调试 get_world_mat4 内部计算过程");
    
    // 获取祖先链
    let ancestors = crate::query_ancestor_refnos(poinsp_refno).await?;
    println!("祖先链: {:?}", ancestors);
    
    // 获取每个祖先的属性
    for (i, &refno) in ancestors.iter().enumerate() {
        let att = get_named_attmap(refno).await?;
        let type_str = att.get_type_str();
        let pos = att.get_position().unwrap_or_default();
        println!("{}: RefNo={:?}, Type={}, POS={:?}", i, refno, type_str, pos);
    }
    
    // 手动计算世界坐标（模拟专门测试的逻辑）
    println!("\n--- 手动计算世界坐标 ---");
    let poinsp_att = get_named_attmap(poinsp_refno).await?;
    let local_pos = poinsp_att.get_position().unwrap_or_default().as_dvec3();
    println!("POINSP 本地位置: {:?}", local_pos);
    
    let owner_refno = poinsp_att.get_owner();
    let owner_att = get_named_attmap(owner_refno).await?;
    let owner_type = owner_att.get_type_str();
    println!("所有者: {:?}, Type: {}", owner_refno, owner_type);
    
    let gensec_refno = if owner_type == "SPINE" {
        owner_att.get_owner()
    } else {
        owner_refno
    };
    
    println!("GENSEC: {:?}", gensec_refno);
    
    // 获取 GENSEC 世界矩阵
    if let Some(gensec_matrix) = get_world_mat4(gensec_refno, false).await? {
        let gensec_pos = gensec_matrix.transform_point3(glam::DVec3::ZERO);
        println!("GENSEC 世界位置: {:?}", gensec_pos);
        
        let calculated_world_pos = gensec_matrix.transform_point3(local_pos);
        println!("计算的世界位置: {:?}", calculated_world_pos);
    }
    
    // 直接获取 POINSP 世界矩阵
    if let Some(poinsp_matrix) = get_world_mat4(poinsp_refno, false).await? {
        let poinsp_world_pos = poinsp_matrix.transform_point3(glam::DVec3::ZERO);
        println!("POINSP 直接世界位置: {:?}", poinsp_world_pos);
    }
    
    Ok(())
}
