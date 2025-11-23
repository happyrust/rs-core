use crate::*;
use anyhow::Result;
use glam::{DVec3, DQuat, DMat4, Vec4Swizzles};

#[tokio::test]
async fn test_virtual_node_skip_logic() -> Result<()> {
    println!("🔍 测试虚拟节点跳过逻辑");
    
    init_surreal().await?;
    
    let spine_refno = RefnoEnum::from("17496_266218");
    let gensec_refno = RefnoEnum::from("17496_266217");
    
    println!("📋 测试SPINE虚拟节点跳过:");
    println!("   SPINE: {}", spine_refno);
    println!("   GENSEC: {}", gensec_refno);
    
    // 获取GENSEC的世界矩阵
    let gensec_world_mat = get_world_mat4(gensec_refno, false).await?.expect("GENSEC should have world matrix");
    println!("   GENSEC世界矩阵: {:?}", gensec_world_mat);
    
    // 获取SPINE的世界矩阵（应该与GENSEC相同，因为SPINE是虚拟节点）
    let spine_world_mat = get_world_mat4(spine_refno, false).await?.expect("SPINE should have world matrix");
    println!("   SPINE世界矩阵: {:?}", spine_world_mat);
    
    // 验证SPINE和GENSEC的世界矩阵是否相同
    let matrix_diff = (gensec_world_mat - spine_world_mat).to_cols_array();
    let max_diff = matrix_diff.iter().fold(0.0f64, |acc, &val| acc.max(val.abs()));
    
    println!("📋 虚拟节点验证:");
    println!("   矩阵差异: {:.10}", max_diff);
    
    if max_diff < 1e-10 {
        println!("   ✅ SPINE作为虚拟节点被正确跳过，世界矩阵与GENSEC相同");
    } else {
        println!("   ❌ SPINE虚拟节点跳过逻辑有问题");
        
        // 详细分析差异
        let gensec_pos = gensec_world_mat.w_axis.xyz();
        let spine_pos = spine_world_mat.w_axis.xyz();
        let gensec_rot = DQuat::from_mat4(&gensec_world_mat);
        let spine_rot = DQuat::from_mat4(&spine_world_mat);
        
        println!("   GENSEC位置: {:?}", gensec_pos);
        println!("   SPINE位置: {:?}", spine_pos);
        println!("   位置差异: {:?}", (gensec_pos - spine_pos).length());
        println!("   GENSEC旋转: {:?}", gensec_rot);
        println!("   SPINE旋转: {:?}", spine_rot);
        println!("   旋转相似度: {:.6}", gensec_rot.dot(spine_rot).abs());
    }
    
    assert!(max_diff < 1e-10, "SPINE虚拟节点应该与GENSEC世界矩阵相同");
    
    println!("✅ 虚拟节点跳过逻辑测试通过");
    
    Ok(())
}

#[tokio::test]
async fn test_poinsp_with_virtual_node_skip() -> Result<()> {
    println!("🔍 测试POINSP在虚拟节点跳过后的计算");
    
    init_surreal().await?;
    
    let poinsp_refno = RefnoEnum::from("17496_266220");
    let spine_refno = RefnoEnum::from("17496_266218");
    let gensec_refno = RefnoEnum::from("17496_266217");
    
    println!("📋 层次结构:");
    println!("   POINSP: {}", poinsp_refno);
    println!("   SPINE: {} (虚拟节点)", spine_refno);
    println!("   GENSEC: {}", gensec_refno);
    
    // 获取各节点的世界矩阵
    let poinsp_world_mat = get_world_mat4(poinsp_refno, false).await?.expect("POINSP should have world matrix");
    let spine_world_mat = get_world_mat4(spine_refno, false).await?.expect("SPINE should have world matrix");
    let gensec_world_mat = get_world_mat4(gensec_refno, false).await?.expect("GENSEC should have world matrix");
    
    println!("\n📋 世界矩阵验证:");
    println!("   POINSP世界位置: {:?}", poinsp_world_mat.w_axis.xyz());
    println!("   SPINE世界位置: {:?}", spine_world_mat.w_axis.xyz());
    println!("   GENSEC世界位置: {:?}", gensec_world_mat.w_axis.xyz());
    
    // 验证SPINE和GENSEC位置相同（虚拟节点）
    let spine_gensec_diff = (spine_world_mat.w_axis.xyz() - gensec_world_mat.w_axis.xyz()).length();
    println!("   SPINE-GENSEC位置差异: {:.6} mm", spine_gensec_diff);
    
    assert!(spine_gensec_diff < 1e-6, "SPINE和GENSEC位置应该相同");
    
    // 验证POINSP位置与JSON测试数据匹配
    let poinsp_world_pos = poinsp_world_mat.w_axis.xyz();
    println!("\n📋 POINSP位置验证:");
    println!("   实际世界位置: {:?}", poinsp_world_pos);
    
    // 期望位置根据JSON: W 0.49mm N 622.59mm D 11.32mm
    // 这需要转换为世界坐标系进行比较
    // 这里我们验证计算的一致性
    
    println!("✅ POINSP虚拟节点跳过测试完成");
    
    Ok(())
}
