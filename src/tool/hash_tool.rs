use std::hash::{Hash, Hasher};
use glam::Vec3;
use fixed::types::I24F8;

pub fn hash_vec3<T: Hasher>(v: &Vec3, hasher: &mut T){
    I24F8::from_num(v[0]).hash(hasher);
    I24F8::from_num(v[1]).hash(hasher);
    I24F8::from_num(v[2]).hash(hasher);
}

//三位有效数字的精度
pub fn hash_f32<T: Hasher>(v: &f32, hasher: &mut T){
    I24F8::from_num(*v).hash(hasher);
}
