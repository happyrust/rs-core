use crate::*;
use anyhow::Result;
use glam::{DMat4, DQuat, DVec3, Vec4Swizzles};

#[tokio::test]
async fn test_virtual_node_detection() -> Result<()> {
    println!("🔍 测试虚拟节点检测系统");

    // 测试SPINE类型
    assert!(is_virtual_node("SPINE"), "SPINE应该是虚拟节点");
    assert!(has_zero_local_translation("SPINE"), "SPINE应该有零局部平移");

    // 测试非虚拟节点
    assert!(!is_virtual_node("GENSEC"), "GENSEC不应该是虚拟节点");
    assert!(!is_virtual_node("POINSP"), "POINSP不应该是虚拟节点");
    assert!(!is_virtual_node("EQUI"), "EQUI不应该是虚拟节点");

    println!("✅ 虚拟节点检测测试通过");

    Ok(())
}

#[tokio::test]
async fn test_spine_virtual_node_orientation() -> Result<()> {
    println!("🔍 测试SPINE虚拟节点方向获取");

    init_surreal().await?;

    let spine_refno = RefnoEnum::from("17496_266218");

    // 获取SPINE的虚拟节点方向
    let orientation = get_virtual_node_orientation(spine_refno, "SPINE").await?;

    assert!(orientation.is_some(), "SPINE应该有方向信息");

    let spine_orientation = orientation.unwrap();
    println!("   SPINE虚拟节点方向: {:?}", spine_orientation);

    // 验证这个方向与get_world_mat4中的SPINE方向一致
    let spine_world_mat = get_world_mat4(spine_refno, false)
        .await?
        .expect("SPINE should have world matrix");
    let spine_rotation = DQuat::from_mat4(&spine_world_mat);

    println!("   get_world_mat4中的SPINE方向: {:?}", spine_rotation);

    // 比较两个方向
    let dot_product = spine_orientation.dot(spine_rotation).abs();
    println!("   方向相似度: {:.6}", dot_product);

    assert!(
        dot_product > 0.999,
        "虚拟节点方向应该与get_world_mat4方向一致"
    );

    println!("✅ SPINE虚拟节点方向测试通过");

    Ok(())
}

#[tokio::test]
async fn test_poinsp_with_virtual_node_system() -> Result<()> {
    println!("🔍 测试POINSP使用虚拟节点系统的计算");

    init_surreal().await?;

    let poinsp_refno = RefnoEnum::from("17496_266220");
    let att = get_named_attmap(poinsp_refno).await?;
    let owner_refno = att.get_owner();
    let owner_att = get_named_attmap(owner_refno).await?;
    let gensec_refno = owner_att.get_owner();

    println!("📋 层次结构:");
    println!("   POINSP: {}", poinsp_refno);
    println!("   SPINE: {} (虚拟节点)", owner_refno);
    println!("   GENSEC: {}", gensec_refno);

    // 获取各节点的世界矩阵
    let poinsp_world_mat = get_world_mat4(poinsp_refno, false)
        .await?
        .expect("POINSP should have world matrix");
    let gensec_world_mat = get_world_mat4(gensec_refno, false)
        .await?
        .expect("GENSEC should have world matrix");

    // 获取POINSP的局部位置
    let poinsp_local_pos = att
        .get_position()
        .expect("POINSP should have POS")
        .as_dvec3();
    println!("   POINSP局部位置: {:?}", poinsp_local_pos);

    // 使用虚拟节点系统计算POINSP世界位置
    // 公式: GENSEC世界矩阵 + POINSP在GENSEC坐标系中的位置
    let gensec_world_pos = gensec_world_mat.w_axis.xyz();
    let gensec_world_rotation = DQuat::from_mat4(&gensec_world_mat);
    let calculated_world_pos = gensec_world_pos + gensec_world_rotation * poinsp_local_pos;

    println!("📋 计算验证:");
    println!("   计算世界位置: {:?}", calculated_world_pos);

    let actual_world_pos = poinsp_world_mat.w_axis.xyz();
    println!("   实际世界位置: {:?}", actual_world_pos);

    let pos_diff = calculated_world_pos - actual_world_pos;
    println!("   位置差异: {:?}", pos_diff);
    println!("   差异大小: {:.6} mm", pos_diff.length());

    assert!(
        pos_diff.length() < 0.01,
        "虚拟节点系统计算应该与实际结果一致"
    );

    println!("✅ POINSP虚拟节点系统测试通过");
    println!("📋 结论: SPINE作为虚拟节点，POINSP直接使用GENSEC坐标系");

    Ok(())
}
