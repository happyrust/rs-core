use ordered_float::OrderedFloat;
use std::hash::{Hash, Hasher};

#[inline]
pub fn f32_round_3(v: f32) -> f32 {
    ((v as f64 * 1000.0).round() / 1000.0f64) as f32
}

#[inline]
pub fn f64_round_3(v: f64) -> f64 {
    (v * 1000.0).round() / 1000.0f64
}

#[inline]
pub fn hash_f32<T: Hasher>(v: f32, hasher: &mut T) {
    OrderedFloat(f32_round_3(v)).hash(hasher);
}

#[inline]
pub fn hash_f64<T: Hasher>(v: f64, hasher: &mut T) {
    OrderedFloat(f64_round_3(v)).hash(hasher);
}

#[inline]
pub fn hash_f64_slice<T: Hasher>(a: &[f64], hasher: &mut T) {
    for v in a {
        hash_f64(*v, hasher);
    }
}

#[inline]
pub fn hash_f32_slice<T: Hasher>(a: &[f32], hasher: &mut T) {
    for v in a {
        hash_f32(*v, hasher);
    }
}
