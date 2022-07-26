use glam::Vec3;
use smol_str::SmolStr;
use crate::tool::db_tool::db1_dehash;

#[inline]
pub fn parse_to_u16(input: &[u8]) -> u16 {
    u16::from_be_bytes(input.try_into().unwrap())
}

#[inline]
pub fn parse_to_i16(input: &[u8]) -> i16 {
    i16::from_be_bytes(input.try_into().unwrap())
}

#[inline]
pub fn parse_to_i32(input: &[u8]) -> i32 {
    i32::from_be_bytes(input.try_into().unwrap())
}

#[inline]
pub fn parse_to_u32(input: &[u8]) -> u32 {
    u32::from_be_bytes(input.try_into().unwrap())
}


#[inline]
pub fn parse_to_f32(input: &[u8]) -> f32 {
    f32_round_3((f32::from_be_bytes(input.try_into().unwrap())))
}

#[inline]
pub fn parse_to_f64(input: &[u8]) -> f64 {
    return if let [a, b, c, d, e, f, g, h] = input[..8] {
        f64_round_3(f64::from_be_bytes([e, f, g, h, a, b, c, d]))
    } else {
        0.0
    };
}


#[inline]
pub fn convert_u32_to_noun(input: &[u8]) -> SmolStr {
    db1_dehash(parse_to_u32(input.try_into().unwrap())).into()
}

#[inline]
pub fn parse_to_f64_arr(input: &[u8]) -> [f64; 3] {
    let mut data = [0f64; 3];
    for i in 0..3 {
        data[i] = parse_to_f64(&input[i * 8..i * 8 + 8]);
    }
    data
}

#[inline]
pub fn parse_to_f32_arr(input: &[u8]) -> [f64; 3] {
    let mut data = [0f64; 3];
    for i in 0..3 {
        data[i] = parse_to_f32(&input[i * 4..i * 4 + 4]) as f64;
    }
    data
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

