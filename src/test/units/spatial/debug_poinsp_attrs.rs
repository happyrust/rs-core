use crate::*;
use anyhow::Result;

#[tokio::test]
async fn debug_poinsp_attrs() -> Result<()> {
    init_surreal().await?;
    
    let poinsp_refno = RefnoEnum::from("17496_266220");
    println!("🔍 调试 POINSP 属性");
    
    let att = get_named_attmap(poinsp_refno).await?;
    
    println!("POINSP 所有属性:");
    // 打印所有关键属性
    let attrs = ["POS", "ZDIS", "PKDI", "YDIR", "XDIR", "ZDIR", "BANG", "POSL", "DELP", "OPDI"];
    for &attr in &attrs {
        if att.contains_key(attr) {
            match attr {
                "POS" => println!("  {}: {:?}", attr, att.get_position()),
                "ZDIS" => println!("  {}: {:?}", attr, att.get_f32("ZDIS")),
                "PKDI" => println!("  {}: {:?}", attr, att.get_f32("PKDI")),
                "YDIR" => println!("  {}: {:?}", attr, att.get_dvec3("YDIR")),
                "XDIR" => println!("  {}: {:?}", attr, att.get_dvec3("XDIR")),
                "ZDIR" => println!("  {}: {:?}", attr, att.get_dvec3("ZDIR")),
                "BANG" => println!("  {}: {:?}", attr, att.get_f32("BANG")),
                "POSL" => println!("  {}: {:?}", attr, att.get_str("POSL")),
                "DELP" => println!("  {}: {:?}", attr, att.get_dvec3("DELP")),
                "OPDI" => println!("  {}: {:?}", attr, att.get_dvec3("OPDI")),
                _ => println!("  {}: 存在", attr),
            }
        }
    }
    
    // 检查是否有旋转
    println!("Rotation: {:?}", att.get_rotation());
    
    Ok(())
}
