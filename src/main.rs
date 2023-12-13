use aios_core::tool::db_tool::{db1_dehash, db1_hash};
use anyhow::Ok;

fn main() -> anyhow::Result<()>{

    dbg!(db1_dehash(0xADF11));
    dbg!(db1_dehash(0x04D852B8));
    dbg!(db1_dehash(0x9CCA7));
    dbg!(db1_dehash(0xAE18D));
    dbg!(db1_dehash(0x000C7983));
    dbg!(db1_dehash(0x9E770));
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

