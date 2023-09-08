pub mod bin_data;



use aios_core::tool::db_tool::{db1_dehash, db1_hash};
use anyhow::Ok;
use crate::bin_data::generate_att_info_json;


fn main() -> anyhow::Result<()>{

    // dbg!(db1_dehash(0x04D852B8));
    // dbg!(db1_dehash(785962));
    println!("{:#4X}", db1_hash("SPRO"));
    // generate_att_info_json()

    Ok(())
}

