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
