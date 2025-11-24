// 临时调试脚本：检查 FITT 构件的属性
use aios_core::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化数据库连接
    crate::init_db().await?;
    
    let refno = RefnoEnum::from_str("25688/7959").unwrap();
    
    println!("🔍 检查 FITT 构件属性: {:?}", refno);
    
    // 获取 FITT 的属性
    let fitt_att = get_named_attmap(refno).await?;
    
    println!("\n=== FITT 基本属性 ===");
    println!("类型: {}", fitt_att.get_type_str());
    println!("名称: {}", fitt_att.get_str("NAME").unwrap_or("N/A"));
    
    println!("\n=== 位置相关属性 ===");
    if let Some(pos) = fitt_att.get_position() {
        println!("POS: {:?}", pos.as_dvec3());
    } else {
        println!("POS: 无");
    }
    
    if let Some(npos) = fitt_att.get_dvec3("NPOS") {
        println!("NPOS: {:?}", npos);
    } else {
        println!("NPOS: 无");
    }
    
    if let Some(zdis) = fitt_att.get_f64("ZDIS") {
        println!("ZDIS: {}", zdis);
    } else {
        println!("ZDIS: 无");
    }
    
    println!("\n=== POSL 相关属性 ===");
    println!("POSL: {}", fitt_att.get_str("POSL").unwrap_or("无"));
    
    println!("\n=== 方向相关属性 ===");
    if let Some(ydir) = fitt_att.get_dvec3("YDIR") {
        println!("YDIR: {:?}", ydir);
    } else {
        println!("YDIR: 无");
    }
    
    if let Some(bang) = fitt_att.get_f64("BANG") {
        println!("BANG: {}", bang);
    } else {
        println!("BANG: 无");
    }
    
    // 获取父节点信息
    if let Some(owner) = fitt_att.get_owner() {
        println!("\n=== 父节点信息 ===");
        println!("Owner: {:?}", owner);
        
        let owner_att = get_named_attmap(owner).await?;
        println!("Owner 类型: {}", owner_att.get_type_str());
        
        if let Some(owner_pos) = owner_att.get_position() {
            println!("Owner POS: {:?}", owner_pos.as_dvec3());
        }
        
        if let Some(owner_ydir) = owner_att.get_dvec3("YDIR") {
            println!("Owner YDIR: {:?}", owner_ydir);
        }
    }
    
    Ok(())
}
