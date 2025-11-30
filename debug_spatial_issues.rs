use aios_core::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化数据库连接
    aios_core::init_surreal().await?;
    
    println!("🔍 调试空间计算问题");
    
    // 有问题的参考号
    let problem_refnos = vec![
        "17496/266220",
        "25688/7960",
    ];
    
    for refno_str in problem_refnos {
        println!("\n" + "=".repeat(60).as_str());
        println!("🧪 分析参考号: {}", refno_str);
        println!("=".repeat(60));
        
        let refno = RefnoEnum::from(refno_str);
        
        // 获取属性映射
        match get_named_attmap(refno).await {
            Ok(att) => {
                println!("✅ 成功获取属性映射");
                
                // 基本信息
                let noun = att.get_type_str();
                let owner = att.get_owner();
                println!("📋 类型: {}", noun);
                println!("👤 父级: {}", owner);
                
                // 位置相关属性
                if let Some(npos) = att.get_dvec3("NPOS") {
                    println!("📍 NPOS: {:?}", npos);
                }
                if let Some(qpos) = att.get_dvec3("QPOS") {
                    println!("📍 QPOS: {:?}", qpos);
                }
                if let Some(xpos) = att.get_dvec3("XPOS") {
                    println!("📍 XPOS: {:?}", xpos);
                }
                
                // 方向相关属性
                if let Some(ydir) = att.get_dvec3("YDIR") {
                    println!("🧭 YDIR: {:?}", ydir);
                }
                if let Some(xdir) = att.get_dvec3("XDIR") {
                    println!("🧭 XDIR: {:?}", xdir);
                }
                if let Some(zdir) = att.get_dvec3("ZDIR") {
                    println!("🧭 ZDIR: {:?}", zdir);
                }
                
                // 旋转相关属性
                if let Some(bang) = att.get_f32("BANG") {
                    println!("🔄 BANG: {}°", bang);
                }
                
                // 特殊属性
                if let Some(zdis) = att.get_f32("ZDIS") {
                    println!("📏 ZDIS: {}", zdis);
                }
                if let Some(posl) = att.get_str("POSL") {
                    println!("📏 POSL: '{}'", posl);
                }
                
                // 获取变换矩阵
                println!("\n🔢 变换矩阵计算:");
                match aios_core::transform::get_world_mat4(refno).await {
                    Some(matrix) => {
                        let translation = matrix.project_point3(glam::DVec3::ZERO);
                        println!("🌍 计算位置: {:?}", translation);
                        
                        // 提取旋转
                        let rotation = glam::DQuat::from_mat4(&matrix);
                        let y_axis = rotation * glam::DVec3::Y;
                        let z_axis = rotation * glam::DVec3::Z;
                        println!("🧭 计算Y轴: {:?}", y_axis);
                        println!("🧭 计算Z轴: {:?}", z_axis);
                    }
                    None => {
                        println!("❌ 无法计算变换矩阵");
                    }
                }
                
                // 分析父级
                if let Ok(parent_att) = get_named_attmap(owner).await {
                    println!("\n👤 父级属性分析:");
                    let parent_noun = parent_att.get_type_str();
                    println!("📋 父级类型: {}", parent_noun);
                    
                    if let Some(parent_npos) = parent_att.get_dvec3("NPOS") {
                        println!("📍 父级NPOS: {:?}", parent_npos);
                    }
                }
                
            }
            Err(e) => {
                println!("❌ 获取属性映射失败: {}", e);
            }
        }
    }
    
    Ok(())
}
