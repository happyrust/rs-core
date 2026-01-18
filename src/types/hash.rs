use serde::Serialize;

pub fn gen_bytes_hash<T>(v: &T) -> u64
where
    T: Serialize,
{
    use core::hash::Hasher;

    let bytes = bincode::serialize(v).unwrap();
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    std::hash::Hash::hash_slice(&bytes, &mut hasher);
    hasher.finish()
}

/// 将浮点数格式化为保留3位有效数字的字符串
fn format_f32_3digits(v: f32) -> String {
    if v.abs() < 1e-10 {
        return "0".to_string();
    }
    format!("{:.3}", v)
}

/// 生成 AABB 的稳定 hash（基于保留3位有效数字的字符串拼接）
pub fn gen_aabb_hash(aabb: &parry3d::bounding_volume::Aabb) -> u64 {
    use core::hash::Hasher;
    
    let s = format!(
        "{}_{}_{}_{}_{}_{}", 
        format_f32_3digits(aabb.mins.x),
        format_f32_3digits(aabb.mins.y),
        format_f32_3digits(aabb.mins.z),
        format_f32_3digits(aabb.maxs.x),
        format_f32_3digits(aabb.maxs.y),
        format_f32_3digits(aabb.maxs.z),
    );
    
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    std::hash::Hash::hash(&s, &mut hasher);
    hasher.finish()
}

/// 生成 Transform 的稳定 hash（基于保留3位有效数字的字符串拼接）
pub fn gen_transform_hash(trans: &crate::rs_surreal::PlantTransform) -> u64 {
    use core::hash::Hasher;
    
    let s = format!(
        "{}_{}_{}_{}_{}_{}_{}_{}_{}_{}", 
        format_f32_3digits(trans.translation.x),
        format_f32_3digits(trans.translation.y),
        format_f32_3digits(trans.translation.z),
        format_f32_3digits(trans.rotation.x),
        format_f32_3digits(trans.rotation.y),
        format_f32_3digits(trans.rotation.z),
        format_f32_3digits(trans.rotation.w),
        format_f32_3digits(trans.scale.x),
        format_f32_3digits(trans.scale.y),
        format_f32_3digits(trans.scale.z),
    );
    
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    std::hash::Hash::hash(&s, &mut hasher);
    hasher.finish()
}
