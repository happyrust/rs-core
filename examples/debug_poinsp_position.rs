use aios_core::*;
use anyhow::Result;
use glam::DVec3;

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化数据库连接
    init_surreal().await?;
    
    let poinsp_refno = RefnoEnum::from("17496_266220");
    
    println!("🔍 深度分析POINSP {} 的位置计算问题", poinsp_refno);
    
    // 1. 获取POINSP基本信息
    let att = get_named_attmap(poinsp_refno).await?;
    println!("📋 POINSP基本信息:");
    println!("  类型: {}", att.get_type_str());
    println!("  所有者: {:?}", att.get_owner());
    
    // 2. 检查POINSP的局部位置属性
    println!("\n📍 POINSP局部位置属性检查:");
    if let Some(pos) = att.get_position() {
        println!("  POS: {:?}", pos);
        println!("  POS (DVec3): {:?}", pos.as_dvec3());
    } else {
        println!("  ❌ 没有POS属性");
    }
    
    if let Some(poss) = att.get_dvec3("POSS") {
        println!("  POSS: {:?}", poss);
    } else {
        println!("  ❌ 没有POSS属性");
    }
    
    if let Some(pose) = att.get_dvec3("POSE") {
        println!("  POSE: {:?}", pose);
    } else {
        println!("  ❌ 没有POSE属性");
    }
    
    // 3. 获取父级GENSEC信息
    let gensec_refno = att.get_owner();
    let gensec_att = get_named_attmap(gensec_refno).await?;
    println!("\n📐 GENSEC {} 信息:", gensec_refno);
    println!("  类型: {}", gensec_att.get_type_str());
    println!("  所有者: {:?}", gensec_att.get_owner());
    
    // 4. 检查GENSEC的位置属性
    println!("\n📍 GENSEC位置属性检查:");
    if let Some(gensec_pos) = gensec_att.get_position() {
        println!("  GENSEC POS: {:?}", gensec_pos);
    } else {
        println!("  ❌ GENSEC没有POS属性");
    }
    
    // 5. 检查GENSEC的世界变换矩阵
    println!("\n🌍 GENSEC世界变换矩阵分析:");
    if let Some(gensec_mat) = get_world_mat4(gensec_refno, false).await? {
        println!("  GENSEC世界矩阵:");
        println!("    平移部分: {:?}", gensec_mat.w_axis.truncate());
        println!("    旋转部分:");
        println!("      X轴: {:?}", gensec_mat.x_axis.truncate());
        println!("      Y轴: {:?}", gensec_mat.y_axis.truncate());
        println!("      Z轴: {:?}", gensec_mat.z_axis.truncate());
        
        // 检查GENSEC的世界位置
        let gensec_world_pos = gensec_mat.w_axis.truncate();
        println!("    GENSEC世界位置: {:?}", gensec_world_pos);
        
    } else {
        println!("  ❌ 无法获取GENSEC的世界变换矩阵");
    }
    
    // 6. 检查POINSP的世界变换矩阵
    println!("\n🌍 POINSP世界变换矩阵分析:");
    if let Some(poinsp_mat) = get_world_mat4(poinsp_refno, false).await? {
        println!("  POINSP世界矩阵:");
        println!("    平移部分: {:?}", poinsp_mat.w_axis.truncate());
        println!("    旋转部分:");
        println!("      X轴: {:?}", poinsp_mat.x_axis.truncate());
        println!("      Y轴: {:?}", poinsp_mat.y_axis.truncate());
        println!("      Z轴: {:?}", poinsp_mat.z_axis.truncate());
        
        let poinsp_world_pos = poinsp_mat.w_axis.truncate();
        println!("    POINSP世界位置: {:?}", poinsp_world_pos);
        
        // 7. 与期望位置对比
        println!("\n🎯 位置对比分析:");
        let expected_wpos = DVec3::new(-5375.49, 1771.29, -2607.01); // W 5375.49mm N 1771.29mm D 2607.01mm
        println!("  期望位置(W 5375.49 N 1771.29 D 2607.01): {:?}", expected_wpos);
        println!("  计算位置: {:?}", poinsp_world_pos);
        
        let diff = poinsp_world_pos - expected_wpos;
        println!("  位置差异: {:?}", diff);
        println!("  距离误差: {:.4} mm", diff.length());
        
        // 分析各轴误差
        println!("  各轴误差分析:");
        println!("    X轴(东西): {:.4} mm (正值=东，负值=西)", diff.x);
        println!("    Y轴(南北): {:.4} mm (正值=北，负值=南)", diff.y);
        println!("    Z轴(上下): {:.4} mm (正值=上，负值=下)", diff.z);
        
    } else {
        println!("  ❌ 无法获取POINSP的世界变换矩阵");
    }
    
    // 8. 检查层级关系
    println!("\n🔗 层级关系分析:");
    // 简化版本：直接检查父级关系
    let mut current_refno = poinsp_refno;
    let mut level = 0;
    
    while level < 10 { // 限制层级深度避免无限循环
        let current_att = get_named_attmap(current_refno).await?;
        let owner_refno = current_att.get_owner();
        
        if owner_refno == current_refno {
            println!("    {}: {} (根节点)", level, current_refno);
            break;
        }
        
        let owner_att = get_named_attmap(owner_refno).await?;
        let type_name = owner_att.get_type_str();
        println!("    {}: {} ({})", level, owner_refno, type_name);
        
        // 检查每个祖先的位置
        if let Some(pos) = owner_att.get_position() {
            println!("      POS: {:?}", pos);
        }
        
        current_refno = owner_refno;
        level += 1;
    }
    
    // 9. 手动计算位置（基于GENSEC矩阵 + POINSP局部坐标）
    if let (Some(gensec_mat), Some(local_pos)) = (get_world_mat4(gensec_refno, false).await?, att.get_position()) {
        println!("\n🧮 手动位置计算验证:");
        let local_pos_d = local_pos.as_dvec3();
        println!("  GENSEC矩阵: {:?}", gensec_mat);
        println!("  POINSP局部坐标: {:?}", local_pos_d);
        
        let manual_calculated = gensec_mat.transform_point3(local_pos_d);
        println!("  手动计算结果: {:?}", manual_calculated);
        
        if let Some(poinsp_mat) = get_world_mat4(poinsp_refno, false).await? {
            let direct_calculated = poinsp_mat.w_axis.truncate();
            let manual_diff = manual_calculated - direct_calculated;
            println!("  与直接计算的差异: {:?}", manual_diff);
            println!("  差异距离: {:.6} mm", manual_diff.length());
        }
    }
    
    Ok(())
}
