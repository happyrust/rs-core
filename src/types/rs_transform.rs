use anyhow::anyhow;
use bevy_ecs::component::Component;
use bevy_reflect::Reflect;
use bevy_transform::components::Transform;
use glam::{Quat, Vec3};
use serde_derive::{Deserialize, Serialize};
use std::fmt::Debug;
use surrealdb::types::SurrealValue;

#[derive(
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
    Serialize,
    Deserialize,
    Clone,
    Copy,
    Default,
    Component,
    Reflect,
    // Deref,
    // DerefMut,
)]
pub struct RsTransform(pub Transform);

impl Debug for RsTransform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("RsTransform").field(&self.0).finish()
    }
}

impl RsTransform {
    /// 获取平移分量
    pub fn translation(&self) -> Vec3 {
        self.0.translation
    }

    /// 获取旋转分量
    pub fn rotation(&self) -> Quat {
        self.0.rotation
    }

    /// 获取缩放分量
    pub fn scale(&self) -> Vec3 {
        self.0.scale
    }

    /// 从平移、旋转、缩放创建Transform
    pub fn from_translation_rotation_scale(translation: Vec3, rotation: Quat, scale: Vec3) -> Self {
        Self(Transform {
            translation,
            rotation,
            scale,
        })
    }
}

impl SurrealValue for RsTransform {
    fn kind_of() -> surrealdb::types::Kind {
        surrealdb::types::Kind::Array(Box::new(surrealdb::types::Kind::Number), None)
    }

    fn into_value(self) -> surrealdb::types::Value {
        surrealdb::types::Value::Array(surrealdb::types::Array::from(vec![
            // translation: x, y, z
            surrealdb::types::Value::Number(surrealdb::types::Number::Float(
                self.0.translation.x as f64,
            )),
            surrealdb::types::Value::Number(surrealdb::types::Number::Float(
                self.0.translation.y as f64,
            )),
            surrealdb::types::Value::Number(surrealdb::types::Number::Float(
                self.0.translation.z as f64,
            )),
            // rotation: x, y, z, w
            surrealdb::types::Value::Number(surrealdb::types::Number::Float(
                self.0.rotation.x as f64,
            )),
            surrealdb::types::Value::Number(surrealdb::types::Number::Float(
                self.0.rotation.y as f64,
            )),
            surrealdb::types::Value::Number(surrealdb::types::Number::Float(
                self.0.rotation.z as f64,
            )),
            surrealdb::types::Value::Number(surrealdb::types::Number::Float(
                self.0.rotation.w as f64,
            )),
            // scale: x, y, z
            surrealdb::types::Value::Number(surrealdb::types::Number::Float(self.0.scale.x as f64)),
            surrealdb::types::Value::Number(surrealdb::types::Number::Float(self.0.scale.y as f64)),
            surrealdb::types::Value::Number(surrealdb::types::Number::Float(self.0.scale.z as f64)),
        ]))
    }

    fn from_value(value: surrealdb::types::Value) -> anyhow::Result<Self> {
        match value {
            surrealdb::types::Value::Array(arr) => {
                if arr.len() != 10 {
                    return Err(anyhow::anyhow!(
                        "数组长度必须为 10 才能转换为 RsTransform (x,y,z + qx,qy,qz,qw + sx,sy,sz)"
                    ));
                }

                let tx = match &arr[0] {
                    surrealdb::types::Value::Number(n) => n.to_f64().unwrap_or(0.0) as f32,
                    _ => return Err(anyhow::anyhow!("Transform translation.x 必须是数字")),
                };
                let ty = match &arr[1] {
                    surrealdb::types::Value::Number(n) => n.to_f64().unwrap_or(0.0) as f32,
                    _ => return Err(anyhow::anyhow!("Transform translation.y 必须是数字")),
                };
                let tz = match &arr[2] {
                    surrealdb::types::Value::Number(n) => n.to_f64().unwrap_or(0.0) as f32,
                    _ => return Err(anyhow::anyhow!("Transform translation.z 必须是数字")),
                };
                let qx = match &arr[3] {
                    surrealdb::types::Value::Number(n) => n.to_f64().unwrap_or(0.0) as f32,
                    _ => return Err(anyhow::anyhow!("Transform rotation.x 必须是数字")),
                };
                let qy = match &arr[4] {
                    surrealdb::types::Value::Number(n) => n.to_f64().unwrap_or(0.0) as f32,
                    _ => return Err(anyhow::anyhow!("Transform rotation.y 必须是数字")),
                };
                let qz = match &arr[5] {
                    surrealdb::types::Value::Number(n) => n.to_f64().unwrap_or(0.0) as f32,
                    _ => return Err(anyhow::anyhow!("Transform rotation.z 必须是数字")),
                };
                let qw = match &arr[6] {
                    surrealdb::types::Value::Number(n) => n.to_f64().unwrap_or(0.0) as f32,
                    _ => return Err(anyhow::anyhow!("Transform rotation.w 必须是数字")),
                };
                let sx = match &arr[7] {
                    surrealdb::types::Value::Number(n) => n.to_f64().unwrap_or(0.0) as f32,
                    _ => return Err(anyhow::anyhow!("Transform scale.x 必须是数字")),
                };
                let sy = match &arr[8] {
                    surrealdb::types::Value::Number(n) => n.to_f64().unwrap_or(0.0) as f32,
                    _ => return Err(anyhow::anyhow!("Transform scale.y 必须是数字")),
                };
                let sz = match &arr[9] {
                    surrealdb::types::Value::Number(n) => n.to_f64().unwrap_or(0.0) as f32,
                    _ => return Err(anyhow::anyhow!("Transform scale.z 必须是数字")),
                };

                let transform = Transform {
                    translation: Vec3::new(tx, ty, tz),
                    rotation: Quat::from_xyzw(qx, qy, qz, qw),
                    scale: Vec3::new(sx, sy, sz),
                };

                Ok(RsTransform(transform))
            }
            _ => Err(anyhow::anyhow!("值必须是数组类型才能转换为 RsTransform")),
        }
    }
}

impl From<Transform> for RsTransform {
    fn from(transform: Transform) -> Self {
        Self(transform)
    }
}

impl From<RsTransform> for Transform {
    fn from(rs_transform: RsTransform) -> Self {
        rs_transform.0
    }
}
