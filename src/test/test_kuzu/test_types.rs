//! Kuzu 类型转换测试

#[cfg(feature = "kuzu")]
use crate::rs_kuzu::types::*;
#[cfg(feature = "kuzu")]
use crate::types::*;
#[cfg(feature = "kuzu")]
use glam::Vec3;

#[test]
#[cfg(feature = "kuzu")]
fn test_attr_to_kuzu_value() {
    // 测试整数类型
    let attr = NamedAttrValue::IntegerType(42);
    let kuzu_val = named_attr_to_kuzu_value(&attr);
    assert!(kuzu_val.is_ok());
    println!("✓ 整数类型转换成功");

    // 测试字符串类型
    let attr = NamedAttrValue::StringType("test".to_string());
    let kuzu_val = named_attr_to_kuzu_value(&attr);
    assert!(kuzu_val.is_ok());
    println!("✓ 字符串类型转换成功");

    // 测试浮点类型
    let attr = NamedAttrValue::F32Type(3.14);
    let kuzu_val = named_attr_to_kuzu_value(&attr);
    assert!(kuzu_val.is_ok());
    println!("✓ 浮点类型转换成功");

    // 测试布尔类型
    let attr = NamedAttrValue::BoolType(true);
    let kuzu_val = named_attr_to_kuzu_value(&attr);
    assert!(kuzu_val.is_ok());
    println!("✓ 布尔类型转换成功");

    // 测试 RefU64 类型
    let attr = NamedAttrValue::RefU64Type(RefU64(12345));
    let kuzu_val = named_attr_to_kuzu_value(&attr);
    assert!(kuzu_val.is_ok());
    println!("✓ RefU64 类型转换成功");

    // 测试 Vec3 类型
    let attr = NamedAttrValue::Vec3Type(Vec3::new(1.0, 2.0, 3.0));
    let kuzu_val = named_attr_to_kuzu_value(&attr);
    assert!(kuzu_val.is_ok());
    println!("✓ Vec3 类型转换成功");
}

#[test]
#[cfg(feature = "kuzu")]
fn test_kuzu_value_to_attr() {
    use kuzu::Value as KuzuValue;

    // 测试整数类型
    let kuzu_val = KuzuValue::Int64(42);
    let attr = kuzu_value_to_named_attr(&kuzu_val, "INT");
    assert!(attr.is_ok());
    if let Ok(NamedAttrValue::IntegerType(i)) = attr {
        assert_eq!(i, 42);
        println!("✓ Kuzu Int64 -> Integer 转换成功");
    }

    // 测试字符串类型
    let kuzu_val = KuzuValue::String("test".to_string());
    let attr = kuzu_value_to_named_attr(&kuzu_val, "STRING");
    assert!(attr.is_ok());
    if let Ok(NamedAttrValue::StringType(s)) = attr {
        assert_eq!(s, "test");
        println!("✓ Kuzu String -> String 转换成功");
    }

    // 测试浮点类型
    let kuzu_val = KuzuValue::Double(3.14);
    let attr = kuzu_value_to_named_attr(&kuzu_val, "FLOAT");
    assert!(attr.is_ok());
    if let Ok(NamedAttrValue::F32Type(f)) = attr {
        assert!((f - 3.14).abs() < 0.01);
        println!("✓ Kuzu Double -> F32 转换成功");
    }

    // 测试布尔类型
    let kuzu_val = KuzuValue::Bool(true);
    let attr = kuzu_value_to_named_attr(&kuzu_val, "BOOL");
    assert!(attr.is_ok());
    if let Ok(NamedAttrValue::BoolType(b)) = attr {
        assert!(b);
        println!("✓ Kuzu Bool -> Bool 转换成功");
    }
}

#[test]
#[cfg(feature = "kuzu")]
fn test_vec3_round_trip() {
    // 测试 Vec3 的往返转换
    let original = NamedAttrValue::Vec3Type(Vec3::new(1.5, 2.5, 3.5));

    // 转换为 Kuzu Value
    let kuzu_val = named_attr_to_kuzu_value(&original).unwrap();

    // 转换回 NamedAttrValue
    let result = kuzu_value_to_named_attr(&kuzu_val, "VEC3").unwrap();

    if let NamedAttrValue::Vec3Type(v) = result {
        assert!((v.x - 1.5).abs() < 0.01);
        assert!((v.y - 2.5).abs() < 0.01);
        assert!((v.z - 3.5).abs() < 0.01);
        println!("✓ Vec3 往返转换成功");
    } else {
        panic!("Vec3 往返转换失败");
    }
}

#[test]
#[cfg(feature = "kuzu")]
fn test_array_conversion() {
    // 测试 I32Array
    let attr = NamedAttrValue::I32Array(vec![1, 2, 3, 4, 5]);
    let kuzu_val = named_attr_to_kuzu_value(&attr);
    assert!(kuzu_val.is_ok());
    println!("✓ I32Array 转换成功");

    // 测试 F32Array
    let attr = NamedAttrValue::F32Array(vec![1.1, 2.2, 3.3]);
    let kuzu_val = named_attr_to_kuzu_value(&attr);
    assert!(kuzu_val.is_ok());
    println!("✓ F32Array 转换成功");

    // 测试 StringArray
    let attr = NamedAttrValue::StringArray(vec!["a".to_string(), "b".to_string()]);
    let kuzu_val = named_attr_to_kuzu_value(&attr);
    assert!(kuzu_val.is_ok());
    println!("✓ StringArray 转换成功");
}

#[test]
#[cfg(feature = "kuzu")]
fn test_logical_type() {
    let attr_int = NamedAttrValue::IntegerType(42);
    let logical_type = get_kuzu_logical_type(&attr_int);
    println!("✓ Integer 逻辑类型: {:?}", logical_type);

    let attr_str = NamedAttrValue::StringType("test".to_string());
    let logical_type = get_kuzu_logical_type(&attr_str);
    println!("✓ String 逻辑类型: {:?}", logical_type);

    let attr_bool = NamedAttrValue::BoolType(true);
    let logical_type = get_kuzu_logical_type(&attr_bool);
    println!("✓ Bool 逻辑类型: {:?}", logical_type);
}