use std::hash::{Hash, Hasher};
use glam::Vec3;
use ordered_float::OrderedFloat;

pub fn hash_vec3<T: Hasher>(v: &Vec3, hasher: &mut T){
    OrderedFloat(v[0]).hash(hasher);
    OrderedFloat(v[1]).hash(hasher);
    OrderedFloat(v[2]).hash(hasher);
}

//三位有效数字的精度
pub fn hash_f32<T: Hasher>(v: &f32, hasher: &mut T){
    OrderedFloat(*v).hash(hasher);
}
