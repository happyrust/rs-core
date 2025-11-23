use aios_core::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化数据库连接
    aios_core::init_surreal().await?;
    
    println!("🔍 深度分析特定参考号的变换计算");
    
    // 重点分析 25688/7960 (FITT类型)
    let refno_str = "25688/7960";
    println!("\n🧪 详细分析: {}", refno_str);
    
    let refno = RefnoEnum::from(refno_str);
    
    // 获取属性映射
    let att = get_named_attmap(refno).await?;
    let noun = att.get_type_str();
    let owner = att.get_owner();
    
    println!("📋 基本信息:");
    println!("   类型: {}", noun);
    println!("   父级: {}", owner);
    
    println!("\n📍 位置相关属性:");
    if let Some(npos) = att.get_dvec3("NPOS") {
        println!("   NPOS: {:?}", npos);
    } else {
        println!("   NPOS: None");
    }
    
    if let Some(qpos) = att.get_dvec3("QPOS") {
        println!("   QPOS: {:?}", qpos);
    } else {
        println!("   QPOS: None");
    }
    
    println!("\n🧭 方向相关属性:");
    if let Some(ydir) = att.get_dvec3("YDIR") {
        println!("   YDIR: {:?}", ydir);
    } else {
        println!("   YDIR: None");
    }
    
    if let Some(xdir) = att.get_dvec3("XDIR") {
        println!("   XDIR: {:?}", xdir);
    } else {
        println!("   XDIR: None");
    }
    
    if let Some(zdir) = att.get_dvec3("ZDIR") {
        println!("   ZDIR: {:?}", zdir);
    } else {
        println!("   ZDIR: None");
    }
    
    println!("\n🔄 旋转相关属性:");
    if let Some(bang) = att.get_f32("BANG") {
        println!("   BANG: {}°", bang);
    } else {
        println!("   BANG: None");
    }
    
    println!("\n📏 偏移相关属性:");
    if let Some(zdis) = att.get_f32("ZDIS") {
        println!("   ZDIS: {}", zdis);
    } else {
        println!("   ZDIS: None");
    }
    
    if let Some(pkdi) = att.get_f32("PKDI") {
        println!("   PKDI: {}", pkdi);
    } else {
        println!("   PKDI: None");
    }
    
    println!("\n👤 父级分析:");
    let parent_att = get_named_attmap(owner).await?;
    let parent_noun = parent_att.get_type_str();
    println!("   父级类型: {}", parent_noun);
    
    if let Some(parent_npos) = parent_att.get_dvec3("NPOS") {
        println!("   父级NPOS: {:?}", parent_npos);
    }
    
    // 获取父级变换矩阵
    if let Some(parent_matrix) = aios_core::transform::get_world_mat4(owner).await? {
        let parent_translation = parent_matrix.project_point3(glam::DVec3::ZERO);
        println!("   父级世界位置: {:?}", parent_translation);
    }
    
    println!("\n🎯 策略分析:");
    let strategy = aios_core::transform::strategies::TransformStrategyFactory::get_strategy(noun);
    println!("   使用策略: {:?}", std::any::type_name_of_val(&strategy));
    
    // 手动调用策略计算
    match strategy.get_local_transform(refno, owner, &att, &parent_att).await {
        Ok(Some(local_matrix)) => {
            let local_translation = local_matrix.project_point3(glam::DVec3::ZERO);
            println!("   局部变换位置: {:?}", local_translation);
        }
        Ok(None) => {
            println!("   局部变换: None");
        }
        Err(e) => {
            println!("   局部变换错误: {}", e);
        }
    }
    
    // 获取最终世界变换
    if let Some(world_matrix) = aios_core::transform::get_world_mat4(refno).await? {
        let world_translation = world_matrix.project_point3(glam::DVec3::ZERO);
        println!("   最终世界位置: {:?}", world_translation);
        
        // 分析变换矩阵
        let rotation = glam::DQuat::from_mat4(&world_matrix);
        let y_axis = rotation * glam::DVec3::Y;
        let z_axis = rotation * glam::DVec3::Z;
        println!("   世界Y轴: {:?}", y_axis);
        println!("   世界Z轴: {:?}", z_axis);
    }
    
    Ok(())
}
