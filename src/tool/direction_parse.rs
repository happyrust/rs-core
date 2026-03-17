use glam::*;
use nom::number::complete::float;
use nom::*;
use std::collections::HashMap;

use lazy_static::lazy_static;
use nom::Parser;
use nom::branch::alt;
use nom::bytes::complete::*;
use nom::combinator::{complete, opt, recognize};
use nom::sequence::{delimited, pair};

lazy_static! {
    pub static ref AXISES_MAP: HashMap<&'static str, Vec3> = {
        let mut s = HashMap::new();
        s.insert("X", Vec3::X);
        s.insert("Y", Vec3::Y);
        s.insert("Z", Vec3::Z);
        s.insert("E", Vec3::X);
        s.insert("N", Vec3::Y);
        s.insert("U", Vec3::Z);
        s.insert("-X", -Vec3::X);
        s.insert("-Y", -Vec3::Y);
        s.insert("-Z", -Vec3::Z);
        s.insert("W", -Vec3::X);
        s.insert("S", -Vec3::Y);
        s.insert("D", -Vec3::Z);
        s
    };
}

#[derive(Debug, Default)]
pub struct Rotation {
    axis: Vec3,
    angle: f64,
}

#[derive(Debug, Default)]
pub struct RotationStruct {
    origin_axis: Vec3,
    rot1: Option<Rotation>,
    rot2: Option<Rotation>,
}

pub fn signed_axis(input: &str) -> IResult<&str, (Option<&str>, &str)> {
    pair(
        opt(tag("-")), // maybe sign?
        alt((
            tag("X"),
            tag("Y"),
            tag("Z"),
            tag("E"),
            tag("N"),
            tag("U"),
            tag("W"),
            tag("S"),
            tag("D"),
        )),
    )
    .parse(input)
}

use crate::tool::math_tool::convert_to_xyz;
use crate::tool::parse_to_dir::parse_to_direction;
use nom::number::complete::double;

pub fn parse_angle(input: &str) -> IResult<&str, f64> {
    alt((double, delimited(tag("("), double, tag(")")))).parse(input)
}

fn parse_axis_rotation(input: &str) -> IResult<&str, Rotation> {
    let (input, angle) = parse_angle(input)?;
    let (input, axis) = recognize(signed_axis).parse(input)?;
    Ok((
        input,
        Rotation {
            axis: *AXISES_MAP.get(axis).unwrap(),
            angle,
        },
    ))
}

pub fn parse_rotation_struct(input: &str) -> IResult<&str, RotationStruct> {
    let (input, axis) = recognize(signed_axis).parse(input)?;
    if input.is_empty() {
        return Ok((
            input,
            RotationStruct {
                origin_axis: *AXISES_MAP.get(axis).unwrap(),
                rot1: None,
                rot2: None,
            },
        ));
    }
    let (input, rot1) = opt(complete(parse_axis_rotation)).parse(input)?;
    if input.is_empty() {
        return Ok((
            input,
            RotationStruct {
                origin_axis: *AXISES_MAP.get(axis).unwrap(),
                rot1,
                rot2: None,
            },
        ));
    }
    let (input, rot2) = opt(complete(parse_axis_rotation)).parse(input)?;
    Ok((
        input,
        RotationStruct {
            origin_axis: *AXISES_MAP.get(axis).unwrap(),
            rot1,
            rot2,
        },
    ))
}

///解析expression到direction
pub fn parse_expr_to_dir(expr: &str) -> Option<DVec3> {
    parse_expr_to_dir_and_quat(expr).map(|(dir, _)| dir)
}

/// 解析方向表达式，同时返回"从 Z 轴旋转到 dir"的旋转四元数。
/// 返回的 quat 满足 `quat * Z = dir`，因此 `quat * X` 是 PDMS 旋转矩阵的 X 轴方向。
///
/// 内部处理：方向表达式 `A (angle) B` 中 origin_axis=A，旋转 quat_raw 满足
/// `quat_raw * A = dir`。需要补偿从 Z→A 的旋转：`result = quat_raw * from_rotation_arc(Z, A)`
pub fn parse_expr_to_dir_and_quat(expr: &str) -> Option<(DVec3, DQuat)> {
    if let Ok(to_dir) = parse_to_direction(expr, None) {
        // TO 格式表达式没有旋转上下文，返回 identity
        return to_dir.map(|d| (d, DQuat::IDENTITY));
    }

    let expr = convert_to_xyz(expr).replace(" ", "");
    if let Ok((_, rs)) = parse_rotation_struct(&expr) {
        let origin: DVec3 = rs.origin_axis.into();
        let mut axis: DVec3 = origin;
        let mut quat_raw = DQuat::IDENTITY;
        if let Some(rot1) = rs.rot1 {
            let target_axis = axis.cross(rot1.axis.into());
            let quat1 = DQuat::from_axis_angle(target_axis, rot1.angle.to_radians() as _);
            axis = (quat1 * axis).normalize();
            quat_raw = quat1 * quat_raw;
            if let Some(rot2) = rs.rot2 {
                let target_axis = axis.cross(rot2.axis.into());
                let quat2 = DQuat::from_axis_angle(target_axis, rot2.angle.to_radians() as _);
                axis = (quat2 * axis).normalize();
                quat_raw = quat2 * quat_raw;
            }
        }
        // quat_raw * origin = axis (= dir)
        // 我们需要 R 使得 R * Z = dir
        // R = quat_raw * from_rotation_arc(Z, origin)
        // 因为 from_rotation_arc(Z, origin) * Z = origin
        // 所以 R * Z = quat_raw * origin = dir ✓
        let origin_n = origin.normalize();
        let z_to_origin = if (origin_n - DVec3::Z).length() < 1e-10 {
            DQuat::IDENTITY // origin 已经是 Z
        } else if (origin_n + DVec3::Z).length() < 1e-10 {
            // origin = -Z, 绕 X 旋转 180°
            DQuat::from_axis_angle(DVec3::X, std::f64::consts::PI)
        } else {
            DQuat::from_rotation_arc(DVec3::Z, origin_n)
        };
        let result_quat = quat_raw * z_to_origin;
        return Some((axis, result_quat));
    }
    None
}
