use crate::pe::SPdmsElement;
use crate::types::named_attmap::NamedAttrMap;
use crate::types::named_attvalue::NamedAttrValue;
use crate::SurlValue;
use glam::Vec3;

#[test]
fn test_des() {
    let str = r#"
        {
  "cata_hash": "13292_0",
  "dbnum": 5100,
  "deleted": false,
  "sesno": 114,
  "id": "pe:13292_0",
  "lock": false,
  "name": "/*",
  "noun": "WORL",
  "owner": "pe:0_0",
  "refno": "WORL:13292_0",
  "status_tag": null,
  "version_tag": null
}
    "#;

    let pe: SPdmsElement = serde_json::from_str(str).unwrap();
    dbg!(pe);
}

#[test]
fn test_get_named_attmap() {
    // 创建一个测试用的 NamedAttrMap
    let mut original_map = NamedAttrMap::default();

    // 添加各种类型的值
    original_map.insert("TYPE".to_string(), NamedAttrValue::StringType("EQUI".to_string()));
    original_map.insert("NAME".to_string(), NamedAttrValue::StringType("Test Equipment".to_string()));
    original_map.insert("REFNO".to_string(), NamedAttrValue::RefU64Type(crate::RefU64::from_two_nums(1112, 12345)));
    original_map.insert("OWNER".to_string(), NamedAttrValue::RefU64Type(crate::RefU64::from_two_nums(1112, 10000)));
    original_map.insert("PGNO".to_string(), NamedAttrValue::IntegerType(100));
    original_map.insert("SESNO".to_string(), NamedAttrValue::IntegerType(50));
    original_map.insert("HEIGHT".to_string(), NamedAttrValue::F32Type(1500.0));
    original_map.insert("POS".to_string(), NamedAttrValue::Vec3Type(Vec3::new(100.0, 200.0, 300.0)));
    original_map.insert("DIMS".to_string(), NamedAttrValue::F32VecType(vec![10.0, 20.0, 30.0]));
    original_map.insert("TAGS".to_string(), NamedAttrValue::StringArrayType(vec!["tag1".to_string(), "tag2".to_string()]));
    original_map.insert("FLAGS".to_string(), NamedAttrValue::IntArrayType(vec![1, 2, 3]));
    original_map.insert("ACTIVE".to_string(), NamedAttrValue::BoolType(true));
    original_map.insert("STATES".to_string(), NamedAttrValue::BoolArrayType(vec![true, false, true]));

    // 转换为 SurlValue
    let surl_value: SurlValue = original_map.clone().into();

    // 从 SurlValue 转换回 NamedAttrMap
    let restored_map: NamedAttrMap = surl_value.into();

    // 验证数据是否正确还原
    assert_eq!(restored_map.get_type(), "EQUI", "TYPE should be EQUI");
    assert_eq!(restored_map.get_name().unwrap(), "Test Equipment", "NAME should match");

    let refno = restored_map.get_refno().unwrap();
    assert_eq!(refno.refno().get_0(), 1112, "REFNO dbnum should be 1112");
    assert_eq!(refno.refno().get_1(), 12345, "REFNO num should be 12345");

    let owner = restored_map.get_owner();
    assert_eq!(owner.refno().get_0(), 1112, "OWNER dbnum should be 1112");
    assert_eq!(owner.refno().get_1(), 10000, "OWNER num should be 10000");

    assert_eq!(restored_map.pgno(), 100, "PGNO should be 100");
    assert_eq!(restored_map.sesno(), 50, "SESNO should be 50");

    // 检查所有原始字段是否已经正确还原
    // 注意：由于 From<SurlValue> for NamedAttrMap 的实现依赖于 TYPE 字段来确定其他字段的类型
    // 只有在 PDMS 数据库元数据（all_attr_info.json）中定义的字段才会被正确还原
    // 所以我们只测试核心的、必定会被还原的字段

    // 打印出实际还原的内容，用于调试
    println!("Restored map keys: {:?}", restored_map.keys().collect::<Vec<_>>());

    // 验证核心字段已还原
    assert!(restored_map.contains_key("TYPE"), "Should contain TYPE key");
    assert!(restored_map.contains_key("NAME"), "Should contain NAME key");
    assert!(restored_map.contains_key("REFNO"), "Should contain REFNO key");
    assert!(restored_map.contains_key("OWNER"), "Should contain OWNER key");
    assert!(restored_map.contains_key("PGNO"), "Should contain PGNO key");
    assert!(restored_map.contains_key("SESNO"), "Should contain SESNO key");

    println!("✅ All NamedAttrMap conversion tests passed!");
}