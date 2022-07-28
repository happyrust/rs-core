use glam::{Vec2, Vec3};
use crate::tool::float_tool::f32_round_3;

#[derive(Clone, Debug, Default)]
pub struct Circle2D {
    pub center: Vec2,
    pub r: f32,
}

impl Circle2D {
    /// Returns a Circle that passes through the three points.
    pub fn from_three_points(j: &Vec2, k: &Vec2, l: &Vec2) -> Circle2D {
        let a = j.x * (k.y - l.y) -
            j.y * (k.x - l.x) +
            k.x * l.y -
            l.x * k.y;

        let b = (j.x * j.x + j.y * j.y) * (l.y - k.y) +
            (k.x * k.x + k.y * k.y) * (j.y - l.y) +
            (l.x * l.x + l.y * l.y) * (k.y - j.y);

        let c = (j.x * j.x + j.y * j.y) * (k.x - l.x) +
            (k.x * k.x + k.y * k.y) * (l.x - j.x) +
            (l.x * l.x + l.y * l.y) * (j.x - k.x);

        let d = (j.x * j.x + j.y * j.y) * (l.x * k.y - k.x * l.y) +
            (k.x * k.x + k.y * k.y) * (j.x * l.y - l.x * j.y) +
            (l.x * l.x + l.y * l.y) * (k.x * j.y - j.x * k.y);

        let x = -b / (2. * a);
        let y = -c / (2. * a);
        let r = ((b * b + c * c - 4. * a * d) / (4. * a * a)).sqrt();

        Circle2D { center: Vec2::new(f32_round_3(x), f32_round_3(y)), r: f32_round_3(r) }
    }

    /// Returns a Circle based on two points (segment is diameter)
    pub fn from_two_points(j: &Vec2, k: &Vec2) -> Circle2D {
        let x = (j.x - k.x) / 2. + k.x;
        let y = (j.y - k.y) / 2. + k.y;
        let r = ((j.x - k.x) * (j.x - k.x) + (j.y - k.y) * (j.y - k.y)).sqrt() / 2.;
        Circle2D { center: Vec2::new(f32_round_3(x), f32_round_3(y)), r: f32_round_3(r) }
    }
}