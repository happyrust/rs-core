use aios_core::tool::{db_tool::{db1_dehash, db1_hash}, dir_tool::parse_ori_str_to_quat};
use anyhow::Ok;
use glam::{Mat3, Quat};
use aios_core::tool::math_tool::*;

fn main() -> anyhow::Result<()>{

    let ori = parse_ori_str_to_quat("Y is Y 5 -X and Z is -Z").unwrap_or(Quat::IDENTITY);
    dbg!(quat_to_pdms_ori_xyz_str(&ori));
    dbg!(Mat3::from_quat(ori));

    dbg!(db1_dehash(0x96B9B));
    
    dbg!(db1_dehash(0xADF11));
    dbg!(db1_dehash(0xDEAE7));
    dbg!(db1_dehash(0x882F1));
    dbg!(db1_dehash(0xE0C71));
    dbg!(db1_dehash(0xE0C56));
    dbg!(db1_dehash(0xDD914));
    dbg!(db1_dehash(0x04D852B8));
    dbg!(db1_dehash(0x9CCA7));
    dbg!(db1_dehash(0xAE18D));
    dbg!(db1_dehash(0x000C7983));
    dbg!(db1_dehash(0x9E770));
    
    dbg!(db1_dehash(0xDD914));
    dbg!(db1_dehash(0xDD92F));
    dbg!(db1_dehash(0xB146F));
    println!("{:#4X}", db1_hash("DESP"));
    println!("{:#4X}", db1_hash("DBLS"));
    // let type_att_info = generate_att_info_json(Some("att_APPLDW"))?;
    // let type_att_info = generate_att_info_json(Some("att_NSEX"))?;
    // // dbg!(&type_att_info);
    // let mut pdms_database_info = get_default_pdms_db_info();
    // pdms_database_info.merge(&type_att_info);
    // // pdms_database_info.save(Some("new_all_attr_info.json"))?;
    // pdms_database_info.save(None)?;

    Ok(())
}

