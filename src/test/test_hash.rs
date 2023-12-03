use crate::{tool::db_tool::db1_dehash, RefU64};

#[test]
fn test_dehash_uda() {
    let hash = 642952055;
    dbg!(db1_dehash(hash));
    let hash = 413837091u32;
    dbg!(db1_dehash(hash));
    let hash = 430614307;
    dbg!(db1_dehash(hash));
    let hash = 738252150;
    dbg!(db1_dehash(hash));
}
