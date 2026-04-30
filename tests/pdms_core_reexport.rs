//! 验证 PDMS 基础工具拆分后，新 crate 与 `aios_core::types::*` 旧路径都可用。

#[test]
fn pdms_hash_paths_are_stable() {
    let name = "SCTN";

    let from_core = aios_core::types::pdms_hash::db1_hash(name);
    let from_new_crate = aios_pdms_core::pdms_hash::db1_hash(name);

    assert_eq!(from_core, from_new_crate);
    assert_eq!(aios_core::types::pdms_hash::db1_hash_const(name), from_core);
    assert_eq!(aios_core::types::pdms_hash::db1_dehash(from_core), name);
    assert_eq!(aios_pdms_core::pdms_hash::db1_dehash(from_new_crate), name);
}

#[test]
fn float_util_paths_are_stable() {
    assert_eq!(aios_core::types::float_util::f32_round_3(1.23456), 1.235);
    assert_eq!(aios_pdms_core::float_util::f64_round_3(1.23456), 1.235);
}
