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

/// 生成 PlantAabb 的稳定 hash（基于保留3位有效数字的字符串拼接）
pub fn gen_plant_aabb_hash(aabb: &crate::types::PlantAabb) -> u64 {
    gen_aabb_hash(&aabb.0)
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

/// 将浮点数格式化为保留3位有效数字的字符串（f64 版本）
fn format_f64_3digits(v: f64) -> String {
    if v.abs() < 1e-10 {
        return "0".to_string();
    }
    format!("{:.3}", v)
}

/// 生成 DMat4 的稳定 hash（基于保留3位有效数字的字符串拼接）
pub fn gen_dmat4_hash(mat: &glam::DMat4) -> u64 {
    use core::hash::Hasher;
    
    let cols = mat.to_cols_array();
    let s = cols.iter()
        .map(|&v| format_f64_3digits(v))
        .collect::<Vec<_>>()
        .join("_");
    
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    std::hash::Hash::hash(&s, &mut hasher);
    hasher.finish()
}

/// 生成 bevy Transform 的稳定 hash（基于保留3位有效数字的字符串拼接）
pub fn gen_bevy_transform_hash(trans: &bevy_transform::prelude::Transform) -> u64 {
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

/// 生成字符串的 hash
pub fn gen_string_hash(s: &str) -> u64 {
    use core::hash::Hasher;
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    std::hash::Hash::hash(s, &mut hasher);
    hasher.finish()
}
