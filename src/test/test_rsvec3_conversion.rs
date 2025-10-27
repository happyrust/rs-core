use crate::shape::pdms_shape::RsVec3;
use glam::Vec3;

// === 基础转换测试 ===

#[test]
fn test_rsvec3_into_vec3() {
    let rs_vec = RsVec3(Vec3::new(1.0, 2.0, 3.0));
    let vec3: Vec3 = rs_vec.into();

    assert_eq!(vec3.x, 1.0);
    assert_eq!(vec3.y, 2.0);
    assert_eq!(vec3.z, 3.0);
    println!("✓ RsVec3 转换为 Vec3 成功");
}

#[test]
fn test_vec3_into_rsvec3() {
    let vec3 = Vec3::new(4.0, 5.0, 6.0);
    let rs_vec: RsVec3 = vec3.into();

    assert_eq!(rs_vec.0.x, 4.0);
    assert_eq!(rs_vec.0.y, 5.0);
    assert_eq!(rs_vec.0.z, 6.0);
    println!("✓ Vec3 转换为 RsVec3 成功");
}

#[test]
fn test_rsvec3_round_trip_conversion() {
    let original = Vec3::new(10.0, 20.0, 30.0);
    let rs_vec: RsVec3 = original.into();
    let back: Vec3 = rs_vec.into();

    assert_eq!(original, back);
    println!("✓ Vec3 <-> RsVec3 往返转换成功");
}

// === Deref 自动解引用测试 ===

#[test]
fn test_rsvec3_deref_methods() {
    let rs_vec = RsVec3(Vec3::new(3.0, 4.0, 0.0));

    // 可以直接调用 Vec3 的方法，无需转换！
    let length = rs_vec.length();
    assert_eq!(length, 5.0);

    let normalized = rs_vec.normalize();
    println!(
        "✓ 直接调用 Vec3 方法: length={}, normalized={:?}",
        length, normalized
    );
}

#[test]
fn test_rsvec3_field_access() {
    let rs_vec = RsVec3(Vec3::new(1.0, 2.0, 3.0));

    // 可以直接访问 Vec3 的字段
    assert_eq!(rs_vec.x, 1.0);
    assert_eq!(rs_vec.y, 2.0);
    assert_eq!(rs_vec.z, 3.0);
    println!(
        "✓ 直接访问字段: x={}, y={}, z={}",
        rs_vec.x, rs_vec.y, rs_vec.z
    );
}

// === 运算符重载测试 ===

#[test]
fn test_rsvec3_mul_scalar() {
    let v = RsVec3(Vec3::new(1.0, 2.0, 3.0));

    // RsVec3 * f32
    let result = &v * 2.0;
    assert_eq!(result.x, 2.0);
    assert_eq!(result.y, 4.0);
    assert_eq!(result.z, 6.0);

    // f32 * RsVec3
    let result2 = 3.0 * &v;
    assert_eq!(result2.x, 3.0);
    assert_eq!(result2.y, 6.0);
    assert_eq!(result2.z, 9.0);

    println!("✓ RsVec3 标量乘法: v*2={:?}, 3*v={:?}", result, result2);
}

#[test]
fn test_rsvec3_add_sub() {
    let v1 = RsVec3(Vec3::new(1.0, 2.0, 3.0));
    let v2 = RsVec3(Vec3::new(4.0, 5.0, 6.0));

    // RsVec3 + RsVec3
    let sum = &v1 + &v2;
    assert_eq!(sum.x, 5.0);
    assert_eq!(sum.y, 7.0);
    assert_eq!(sum.z, 9.0);

    // RsVec3 - RsVec3
    let diff = &v2 - &v1;
    assert_eq!(diff.x, 3.0);
    assert_eq!(diff.y, 3.0);
    assert_eq!(diff.z, 3.0);

    println!("✓ RsVec3 加减法: sum={:?}, diff={:?}", sum, diff);
}

#[test]
fn test_rsvec3_neg() {
    let v = RsVec3(Vec3::new(1.0, -2.0, 3.0));
    let neg = -&v;

    assert_eq!(neg.x, -1.0);
    assert_eq!(neg.y, 2.0);
    assert_eq!(neg.z, -3.0);

    println!("✓ RsVec3 取负: -{:?} = {:?}", v, neg);
}

// === 实际使用场景测试 ===

#[test]
fn test_rsvec3_practical_usage() {
    // 场景：计算两点间的方向向量并归一化
    let start = RsVec3(Vec3::new(0.0, 0.0, 0.0));
    let end = RsVec3(Vec3::new(10.0, 0.0, 0.0));

    // 完全像使用 Vec3 一样使用 RsVec3！
    let direction = (&end - &start).normalize();
    assert_eq!(direction.x, 1.0);
    assert_eq!(direction.y, 0.0);

    // 缩放向量
    let scaled = direction * 5.0;
    assert_eq!(scaled.x, 5.0);

    println!("✓ 实际场景: direction={:?}, scaled={:?}", direction, scaled);
}

#[test]
fn test_rsvec3_with_transform() {
    let pos = RsVec3(Vec3::new(1.0, 2.0, 3.0));
    let offset = RsVec3(Vec3::new(10.0, 20.0, 30.0));

    // 位置变换
    let new_pos = &pos + &offset;

    // 缩放
    let scaled_pos = new_pos * 0.5;

    assert_eq!(scaled_pos.x, 5.5);
    assert_eq!(scaled_pos.y, 11.0);
    assert_eq!(scaled_pos.z, 16.5);

    println!(
        "✓ 变换场景: original={:?}, offset={:?}, result={:?}",
        pos, offset, scaled_pos
    );
}

// === RsVec3 与 Vec3 混合运算测试 ===

#[test]
fn test_rsvec3_add_vec3() {
    let rs_vec = RsVec3(Vec3::new(1.0, 2.0, 3.0));
    let vec3 = Vec3::new(4.0, 5.0, 6.0);

    // RsVec3 + Vec3
    let result1 = &rs_vec + vec3;
    assert_eq!(result1.x, 5.0);
    assert_eq!(result1.y, 7.0);
    assert_eq!(result1.z, 9.0);

    // Vec3 + RsVec3
    let result2 = vec3 + &rs_vec;
    assert_eq!(result2.x, 5.0);
    assert_eq!(result2.y, 7.0);
    assert_eq!(result2.z, 9.0);

    println!("✓ RsVec3 与 Vec3 加法: {:?}, {:?}", result1, result2);
}

#[test]
fn test_rsvec3_sub_vec3() {
    let rs_vec = RsVec3(Vec3::new(10.0, 20.0, 30.0));
    let vec3 = Vec3::new(1.0, 2.0, 3.0);

    // RsVec3 - Vec3
    let result1 = &rs_vec - vec3;
    assert_eq!(result1.x, 9.0);
    assert_eq!(result1.y, 18.0);
    assert_eq!(result1.z, 27.0);

    // Vec3 - RsVec3
    let result2 = vec3 - &rs_vec;
    assert_eq!(result2.x, -9.0);
    assert_eq!(result2.y, -18.0);
    assert_eq!(result2.z, -27.0);

    println!("✓ RsVec3 与 Vec3 减法: {:?}, {:?}", result1, result2);
}

#[test]
fn test_rsvec3_vec3_reference_ops() {
    let rs_vec = RsVec3(Vec3::new(1.0, 2.0, 3.0));
    let vec3 = Vec3::new(10.0, 20.0, 30.0);

    // 使用引用运算，不消耗原值
    let result1 = &rs_vec + &vec3;
    let result2 = &vec3 - &rs_vec;

    // 原值仍然可用
    assert_eq!(rs_vec.x, 1.0);
    assert_eq!(vec3.x, 10.0);

    assert_eq!(result1.x, 11.0);
    assert_eq!(result2.x, 9.0);

    println!(
        "✓ 引用运算: 原值保留，result1={:?}, result2={:?}",
        result1, result2
    );
}

#[test]
fn test_mixed_operations_complex() {
    let rs_vec1 = RsVec3(Vec3::new(1.0, 0.0, 0.0));
    let rs_vec2 = RsVec3(Vec3::new(0.0, 1.0, 0.0));
    let vec3 = Vec3::new(0.0, 0.0, 1.0);

    // 复杂混合运算
    let result = &rs_vec1 + &rs_vec2 + vec3;
    assert_eq!(result.x, 1.0);
    assert_eq!(result.y, 1.0);
    assert_eq!(result.z, 1.0);

    // 混合运算与标量
    let scaled = (&rs_vec1 + vec3) * 2.0;
    assert_eq!(scaled.x, 2.0);
    assert_eq!(scaled.z, 2.0);

    println!("✓ 复杂混合运算成功");
}

#[test]
fn test_practical_mixed_usage() {
    // 实际场景：计算从 Vec3 点到 RsVec3 点的方向
    let start = Vec3::new(0.0, 0.0, 0.0);
    let end = RsVec3(Vec3::new(10.0, 0.0, 0.0));

    // 直接混合运算！
    let direction = (&end - start).normalize();
    assert_eq!(direction.x, 1.0);

    // 反向
    let reverse_dir = (start - &end).normalize();
    assert_eq!(reverse_dir.x, -1.0);

    println!(
        "✓ 实际混合场景: direction={:?}, reverse={:?}",
        direction, reverse_dir
    );
}

#[test]
fn test_position_offset_mixed() {
    // 场景：RsVec3 位置 + Vec3 偏移
    let position = RsVec3(Vec3::new(100.0, 200.0, 300.0));
    let offset = Vec3::new(10.0, 20.0, 30.0);

    let new_position = &position + offset;
    assert_eq!(new_position.x, 110.0);
    assert_eq!(new_position.y, 220.0);
    assert_eq!(new_position.z, 330.0);

    // 也可以反过来
    let new_position2 = offset + &position;
    assert_eq!(new_position2.x, 110.0);

    println!("✓ 位置偏移混合运算成功");
}

// === AsRef/AsMut 测试 ===

#[test]
fn test_rsvec3_as_ref() {
    let rs_vec = RsVec3(Vec3::new(1.0, 2.0, 3.0));

    // 可以作为 Vec3 的引用传递
    let vec_ref: &Vec3 = rs_vec.as_ref();
    assert_eq!(vec_ref.x, 1.0);

    println!("✓ AsRef 转换成功");
}

#[test]
fn test_rsvec3_as_mut() {
    let mut rs_vec = RsVec3(Vec3::new(1.0, 2.0, 3.0));

    // 可以获取可变引用并修改
    let vec_mut: &mut Vec3 = rs_vec.as_mut();
    vec_mut.x = 10.0;

    assert_eq!(rs_vec.x, 10.0);

    println!("✓ AsMut 转换和修改成功");
}
