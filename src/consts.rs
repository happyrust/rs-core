use crate::tool::db_tool::db1_hash;
pub const UNSET_STR: &'static str = "unset";

pub const ATT_WORLD : u32 = 0xBEB83u32;
pub const ATT_NUMBDB : u32 = 31404443;
pub const ATT_STYP: u32 = 865153;
pub const ATT_SITE: u32 = 0x9D65A;
pub const ATT_TYPE : u32 = 0x9CCA7;
pub const ATT_CURD: u32 = 623865;


pub const BOX_NOUN: u32 = db1_hash("BOX");
pub const NAME_NOUN: u32 = db1_hash("NAME");
pub const REFNO_NOUN: u32 = db1_hash("REFNO");
pub const OWNER_NOUN: u32 = db1_hash("OWNER");
pub const CYLI_NOUN: u32 = db1_hash("CYLI");
pub const SPHE_NOUN: u32 = db1_hash("SPHE");
pub const CONE_NOUN: u32 = db1_hash("CONE");
pub const DISH_NOUN: u32 = db1_hash("DISH");
pub const CTOR_NOUN: u32 = db1_hash("CTOR");
pub const RTOR_NOUN: u32 = db1_hash("RTOR");
pub const PYRA_NOUN: u32 = db1_hash("PYRA");
pub const LOOP_NOUN: u32 = db1_hash("LOOP");
pub const PLOO_NOUN: u32 = db1_hash("PLOO");
pub const SPINE_NOUN: u32 = db1_hash("SPINE");
pub const GENSEC_NOUN: u32 = db1_hash("GENSEC");
pub const POHE_NOUN: u32 = db1_hash("POHE");    //多边形的处理
pub const REVO_NOUN: u32 = db1_hash("REVO");
pub const NREV_NOUN: u32 = db1_hash("NREV");   //todo 负实体，后面需要加入