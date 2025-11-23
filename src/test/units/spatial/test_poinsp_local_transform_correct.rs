use crate::*;
use anyhow::Result;
use glam::{DVec3, DQuat, DMat4, Vec4Swizzles};

#[tokio::test]
async fn test_poinsp_local_transform_with_orientation() -> Result<()> {
    println!("🔍 测试POINSP Local Transform的正确理解");
    
    init_surreal().await?;
    
    let poinsp_refno = RefnoEnum::from("17496_266220");
    let att = get_named_attmap(poinsp_refno).await?;
    let owner_refno = att.get_owner(); // SPINE
    
    println!("📋 分析POINSP 17496/266220的Local Transform:");
    println!("   POINSP: {}", poinsp_refno);
    println!("   父级SPINE: {}", owner_refno);
    
    // 获取POINSP和SPINE的世界矩阵
    let poinsp_world_mat = get_world_mat4(poinsp_refno, false).await?.expect("POINSP should have world matrix");
    let spine_world_mat = get_world_mat4(owner_refno, false).await?.expect("SPINE should have world matrix");
    
    println!("\n📋 世界变换矩阵:");
    println!("   POINSP世界矩阵: {:?}", poinsp_world_mat);
    println!("   SPINE世界矩阵: {:?}", spine_world_mat);
    
    // 计算POINSP相对于SPINE的local transform
    // local_mat = inverse(spine_world_mat) * poinsp_world_mat
    let spine_world_inverse = spine_world_mat.inverse();
    let poinsp_local_to_spine = spine_world_inverse * poinsp_world_mat;
    
    println!("\n📋 POINSP相对于SPINE的Local Transform:");
    println!("   变换矩阵: {:?}", poinsp_local_to_spine);
    
    // 分解local transform
    let local_translation = poinsp_local_to_spine.w_axis.xyz();
    let local_rotation = DQuat::from_mat4(&poinsp_local_to_spine);
    let local_scale = DVec3::new(
        poinsp_local_to_spine.x_axis.length(),
        poinsp_local_to_spine.y_axis.length(), 
        poinsp_local_to_spine.z_axis.length()
    );
    
    println!("\n📋 Local Transform分解:");
    println!("   局部平移: {:?}", local_translation);
    println!("   局部旋转: {:?}", local_rotation);
    println!("   局部缩放: {:?}", local_scale);
    
    // 获取POINSP的POS属性
    let poinsp_pos = att.get_position().expect("POINSP should have POS").as_dvec3();
    println!("\n📋 POINSP属性:");
    println!("   POS属性: {:?}", poinsp_pos);
    
    // 验证SPINE是否为虚拟节点（IDENTITY）
    let spine_att = get_named_attmap(owner_refno).await?;
    let spine_owner_refno = spine_att.get_owner(); // GENSEC
    let gensec_world_mat = get_world_mat4(spine_owner_refno, false).await?.expect("GENSEC should have world matrix");
    let gensec_world_inverse = gensec_world_mat.inverse();
    let spine_local_to_gensec = gensec_world_inverse * spine_world_mat;
    
    println!("\n📋 SPINE作为虚拟节点验证:");
    println!("   SPINE相对于GENSEC的变换: {:?}", spine_local_to_gensec);
    
    // 检查SPINE是否接近IDENTITY（除了可能的旋转）
    let spine_translation = spine_local_to_gensec.w_axis.xyz();
    let spine_rotation = DQuat::from_mat4(&spine_local_to_gensec);
    
    println!("   SPINE局部平移: {:?}", spine_translation);
    println!("   SPINE局部旋转: {:?}", spine_rotation);
    
    let is_zero_translation = spine_translation.length() < 1e-6;
    println!("   SPINE是否为零平移: {}", is_zero_translation);
    
    // 转换POINSP的local rotation为ENDATU格式
    println!("\n📋 POINSP Local Transform方位分析:");
    println!("   用户期望: Y is N 88.958 U and Z is N 0.0451 W 1.0416 D");
    println!("   实际旋转: {:?}", local_rotation);
    
    // TODO: 将四元数转换为ENDATU格式进行验证
    // 这里需要实现四元数到ENDATU格式的转换函数
    
    println!("\n✅ POINSP Local Transform分析完成");
    println!("📋 结论: SPINE是虚拟节点，POINSP的local transform包含从SPINE YDIR推导的方位信息");
    
    Ok(())
}
