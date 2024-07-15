use glam::{DVec3, Vec3};
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::character::complete::space0;
use nom::combinator::{map_res, opt};
use nom::number::complete::float;
use nom::sequence::{delimited, preceded};
use nom::IResult;

#[derive(Debug, PartialEq)]
pub struct Coordinate {
    x: Option<f32>,
    y: Option<f32>,
    z: Option<f32>,
}

impl Coordinate {
    pub fn get_dir(&self) -> Option<DVec3> {
        let v = DVec3::new(
            self.x.unwrap_or(0.0) as f64,
            self.y.unwrap_or(0.0) as f64,
            self.z.unwrap_or(0.0) as f64,
        )
        .normalize_or_zero();
        if v.is_normalized() {
            Some(v)
        } else {
            None
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct Direction {
    coordinate: Coordinate,
}

fn ws<'a, F: 'a, O>(inner: F) -> impl FnMut(&'a str) -> IResult<&'a str, O>
where
    F: FnMut(&'a str) -> IResult<&'a str, O>,
{
    delimited(space0, inner, space0)
}

fn parse_float(input: &str) -> IResult<&str, f32> {
    float(input)
}

fn parse_neg_float(input: &str) -> IResult<&str, f32> {
    map_res(
        preceded(
            ws(tag("NEG")),
            delimited(opt(ws(tag("("))), ws(parse_float), opt(ws(tag(")")))),
        ),
        |n| Ok::<_, ()>(-n),
    )(input)
}

fn parse_coordinate_value(input: &str) -> IResult<&str, f32> {
    alt((
        delimited(opt(ws(tag("("))), parse_neg_float, opt(ws(tag(")")))),
        delimited(opt(ws(tag("("))), ws(parse_float), opt(ws(tag(")")))),
    ))(input)
}

fn parse_axis_value(axis: &'static str) -> impl Fn(&str) -> IResult<&str, (String, f32)> {
    move |input: &str| {
        let (input, _) = ws(tag(axis))(input)?;
        let (input, value) = parse_coordinate_value(input)?;
        Ok((input, (axis.to_string(), value)))
    }
}

fn parse_coordinate(input: &str) -> IResult<&str, Coordinate> {
    let (input, values) = nom::multi::many0(alt((
        parse_axis_value("X"),
        parse_axis_value("Y"),
        parse_axis_value("Z"),
    )))(input)?;

    let mut coord = Coordinate {
        x: None,
        y: None,
        z: None,
    };
    for (axis, value) in values {
        match axis.as_str() {
            "X" => coord.x = Some(value),
            "Y" => coord.y = Some(value),
            "Z" => coord.z = Some(value),
            _ => {}
        }
    }

    Ok((input, coord))
}

pub fn parse_to_direction(input: &str) -> IResult<&str, Option<DVec3>> {
    let (input, _) = ws(tag("TO"))(input)?;
    let (input, coordinate) = parse_coordinate(input)?;

    Ok((input, coordinate.get_dir()))
}
