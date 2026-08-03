use parry3d::bounding_volume::Aabb;

use crate::{RefU64, gen_aabb_hash, tool::db_tool::db1_dehash};

#[test]
fn test_is_uda_fff_family_unsigned() {
    use crate::tool::db_tool::is_uda;
    // 0xFFF5AFAA as i32 is negative; signed `>` would wrongly reject it.
    let h = 0xFFF5_AFAAu32 as i32;
    assert!(h < 0);
    assert!(is_uda(h), "0xFFF family must count as UDA");
    assert!(is_uda(0x2C00_D55Au32 as i32));
    assert!(!is_uda(0x9C18E)); // NAME-ish below threshold
}

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

#[test]
fn test_gen_hash() {
    let aabb = Aabb::new([-1.0, -1.0, -1.0].into(), [1.0, 1.0, 1.0].into());
    let hash1 = gen_aabb_hash(&aabb);
    let aabb = Aabb::new([1.0, -1.0, -1.0].into(), [1.0, 1.0, 1.0].into());
    let hash2 = gen_aabb_hash(&aabb);
    assert_ne!(hash1, hash2);
}
