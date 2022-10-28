use serde::{Serialize, Deserialize};
#[cfg(not(target_arch = "wasm32"))]
use crate::cache::mgr::BytesTrait;
use crate::pdms_types::{AttrMap, RefU64};
use crate::pdms_types::NounHash;
use crate::tool::db_tool::db1_hash;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct CachedRefBasic {
    pub owner: RefU64,
    pub table: String, //提前处理好成了table name，有关键字冲突的地方，删除最后的
}

#[cfg(not(target_arch = "wasm32"))]
impl BytesTrait for CachedRefBasic {
    fn to_bytes(&self) -> Vec<u8> {
        bincode::serialize(&self).unwrap().into()
    }

    fn from_bytes(bytes: &[u8]) -> Self {
        bincode::deserialize(bytes).unwrap()
    }
}

impl CachedRefBasic{

    #[inline]
    pub fn get_type(&self) -> &str{
        if self.table.ends_with("_") {
            &self.table[..self.table.len()-1]
        }else{
            self.table.as_str()
        }
    }

    #[inline]
    pub fn get_table_name(&self) -> &str{
        self.table.as_str()
    }

    #[inline]
    pub fn get_noun_hash(&self) -> NounHash{
        db1_hash(self.get_type()).into()
    }

    #[inline]
    pub fn get_owner(&self) -> RefU64{
        self.owner
    }

}