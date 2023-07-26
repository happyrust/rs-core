pub mod bin_data;

use aios_core::prim_geo::sbox::SBox;
use aios_core::shape::pdms_shape::BrepShapeTrait;
use aios_core::tool::db_tool::{db1_dehash, db1_hash};
use crate::bin_data::generate_att_info_json;


fn main() -> anyhow::Result<()>{

    // dbg!(db1_dehash(0x04D852B8));
    dbg!(db1_dehash(581519));
    println!("{:#4X}", db1_hash("SUPPO"));
    generate_att_info_json()
}

