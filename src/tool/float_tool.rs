use std::hash::{Hash, Hasher};
use glam::Vec3;
// use fixed::types::I24F8;
use ordered_float::*;

#[inline]
pub fn hash_vec3<T: Hasher>(v: &Vec3, hasher: &mut T) {
    hash_f32(v[0], hasher);
    hash_f32(v[1], hasher);
    hash_f32(v[2], hasher);
}

//三位有效数字的精度
#[inline]
pub fn hash_f32<T: Hasher>(v: f32, hasher: &mut T) {
    OrderedFloat(f32_round_3(v)).hash(hasher);
}

#[inline]
pub fn vec3_round_3(v: Vec3) -> Vec3 {
    Vec3::new(f32_round_3(v.x), f32_round_3(v.y), f32_round_3(v.z))
}

#[inline]
pub fn vec3_round_2(v: Vec3) -> Vec3 {
    Vec3::new(f32_round_2(v.x), f32_round_2(v.y), f32_round_2(v.z))
}

#[inline]
pub fn f32_round_3(v: f32) -> f32 {
    ((v as f64 * 1000.0).round() / 1000.0f64) as f32    //以防止溢出
}

#[inline]
pub fn f32_round_2(v: f32) -> f32 {
    ((v as f64 * 100.0).round() / 100.0f64) as f32    //以防止溢出
}

#[inline]
pub fn f32_round_1(v: f32) -> f32 {
    ((v as f64 * 10.0).round() / 10.0f64) as f32    //以防止溢出
}


#[inline]
pub fn f64_round_3(v: f64) -> f64 {
    (v * 1000.0).round() / 1000.0f64    //以防止溢出
}

#[inline]
pub fn f64_round_2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0f64    //以防止溢出
}

#[inline]
pub fn f64_round_1(v: f64) -> f64 {
    (v * 10.0).round() / 10.0f64    //以防止溢出
}

