use crate::pdms_types::NounHash;
use crate::tool::db_tool::db1_hash;
pub const UNSET_STR: &'static str = "unset";

pub const ATT_WORLD : u32 = 0xBEB83u32;
pub const ATT_NUMBDB : u32 = 31404443;
pub const ATT_STYP: u32 = 865153;
pub const ATT_SITE: u32 = 0x9D65A;
pub const ATT_TYPE : u32 = 0x9CCA7;
pub const ATT_CURD: u32 = 623865;

pub const TYPE_HASH: NounHash = NounHash(db1_hash("TYPE"));
pub const BOX_HASH: NounHash = NounHash(db1_hash("BOX"));
pub const NAME_HASH: NounHash = NounHash(db1_hash("NAME"));
pub const REFNO_HASH: NounHash = NounHash(db1_hash("REFNO"));
pub const OWNER_HASH: NounHash = NounHash(db1_hash("OWNER"));
pub const CYLI_HASH: NounHash = NounHash(db1_hash("CYLI"));
pub const SPHE_HASH: NounHash = NounHash(db1_hash("SPHE"));
pub const CONE_HASH: NounHash = NounHash(db1_hash("CONE"));
pub const DISH_HASH: NounHash = NounHash(db1_hash("DISH"));
pub const CTOR_HASH: NounHash = NounHash(db1_hash("CTOR"));
pub const RTOR_HASH: NounHash = NounHash(db1_hash("RTOR"));
pub const PYRA_HASH: NounHash = NounHash(db1_hash("PYRA"));
pub const LOOP_HASH: NounHash = NounHash(db1_hash("LOOP"));
pub const PLOO_HASH: NounHash = NounHash(db1_hash("PLOO"));
pub const SPINE_HASH: NounHash = NounHash(db1_hash("SPINE"));
pub const GENSEC_HASH: NounHash = NounHash(db1_hash("GENSEC"));
pub const POHE_HASH: NounHash = NounHash(db1_hash("POHE"));    //多边形的处理
pub const REVO_HASH: NounHash = NounHash(db1_hash("REVO"));
pub const NREV_HASH: NounHash = NounHash(db1_hash("NREV"));   //todo 负实体，后面需要加入