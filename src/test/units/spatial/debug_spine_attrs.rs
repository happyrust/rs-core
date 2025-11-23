use crate::*;
use anyhow::Result;

#[tokio::test]
async fn debug_spine_attrs() -> Result<()> {
    init_surreal().await?;
    
    let spine_refno = RefnoEnum::from("17496_266218");
    println!("🔍 调试 SPINE 属性");
    
    let att = get_named_attmap(spine_refno).await?;
    
    println!("SPINE 所有属性:");
    let attrs = ["POS", "YDIR", "XDIR", "ZDIR", "BANG", "POSL", "DELP", "OPDI"];
    for &attr in &attrs {
        if att.contains_key(attr) {
            match attr {
                "POS" => println!("  {}: {:?}", attr, att.get_position()),
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
    
    println!("Rotation: {:?}", att.get_rotation());
    
    Ok(())
}
