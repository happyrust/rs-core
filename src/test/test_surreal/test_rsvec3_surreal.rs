use crate::shape::pdms_shape::RsVec3;
use glam::Vec3;
use surrealdb::types::SurrealValue;

#[test]
fn test_rsvec3_into_value() {
    let vec = RsVec3(Vec3::new(1.0, 2.0, 3.0));
    let value = vec.into_value();
    
    if let surrealdb::types::Value::Array(arr) = value {
        assert_eq!(arr.len(), 3);
        println!("RsVec3 转换为 SurrealDB Array 成功: {:?}", arr);
    } else {
        panic!("转换失败: 不是数组类型");
    }
}

#[test]
fn test_rsvec3_from_value() {
    let value = surrealdb::types::Value::Array(surrealdb::types::Array::from(vec![
        surrealdb::types::Value::Number(surrealdb::types::Number::Float(1.5)),
        surrealdb::types::Value::Number(surrealdb::types::Number::Float(2.5)),
        surrealdb::types::Value::Number(surrealdb::types::Number::Float(3.5)),
    ]));
    
    let result = RsVec3::from_value(value);
    assert!(result.is_ok());
    
    let vec = result.unwrap();
    assert_eq!(vec.0.x, 1.5);
    assert_eq!(vec.0.y, 2.5);
    assert_eq!(vec.0.z, 3.5);
    println!("从 SurrealDB Array 转换为 RsVec3 成功: {:?}", vec);
}

#[test]
fn test_rsvec3_round_trip() {
    let original = RsVec3(Vec3::new(10.0, 20.0, 30.0));
    let value = original.clone().into_value();
    let restored = RsVec3::from_value(value).unwrap();
    
    assert_eq!(original.0.x, restored.0.x);
    assert_eq!(original.0.y, restored.0.y);
    assert_eq!(original.0.z, restored.0.z);
    println!("RsVec3 往返转换成功");
}

#[test]
fn test_rsvec3_from_invalid_value() {
    // 测试错误的数组长度
    let value = surrealdb::types::Value::Array(surrealdb::types::Array::from(vec![
        surrealdb::types::Value::Number(surrealdb::types::Number::Float(1.0)),
        surrealdb::types::Value::Number(surrealdb::types::Number::Float(2.0)),
    ]));
    
    let result = RsVec3::from_value(value);
    assert!(result.is_err());
    println!("错误处理测试通过: {:?}", result.unwrap_err());
}
