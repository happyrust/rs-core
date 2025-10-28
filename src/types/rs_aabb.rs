use anyhow::anyhow;
use glam::Vec3;
use parry3d::bounding_volume::{Aabb, BoundingVolume};
use parry3d::math::Point;
use serde::{Deserialize, Serialize};
use surrealdb::types::SurrealValue;

use crate::shape::pdms_shape::RsVec3;

/// Aabb包装类型，为Aabb实现SurrealValue
#[derive(
    Serialize,
    Deserialize,
    Clone,
    Copy,
    Debug,
    PartialEq,
)]
pub struct RsAabb(pub Aabb);

impl Default for RsAabb {
    fn default() -> Self {
        Self(Aabb::new_invalid())
    }
}

impl RsAabb {
    /// 创建新的RsAabb
    pub fn new(mins: Vec3, maxs: Vec3) -> Self {
        Self(Aabb::new(mins.into(), maxs.into()))
    }
    
    /// 从Transform合并创建Aabb
    pub fn from_transform(mins: Vec3, maxs: Vec3) -> Self {
        Self(Aabb::new(mins.into(), maxs.into()))
    }
    
    /// 获取中心点
    pub fn center(&self) -> RsVec3 {
        RsVec3(self.0.center().coords.into())
    }
    
    /// 获取半 extents
    pub fn half_extents(&self) -> RsVec3 {
        RsVec3(self.0.half_extents().into())
    }
    
    /// 获取最小点
    pub fn mins(&self) -> RsVec3 {
        RsVec3(self.0.mins.coords.into())
    }
    
    /// 获取最大点
    pub fn maxs(&self) -> RsVec3 {
        RsVec3(self.0.maxs.coords.into())
    }
    
    /// 合并两个Aabb
    pub fn merge(&self, other: &RsAabb) -> Self {
        Self(self.0.merged(&other.0))
    }
    
    /// 检查是否包含点
    pub fn contains_point(&self, point: &Vec3) -> bool {
        self.0.contains_local_point(&Point::from([point.x, point.y, point.z]))
    }
    
    /// 扩展Aabb以包含给定点
    pub fn take_point(&mut self, point: &Vec3) {
        self.0.take_point(Point::from([point.x, point.y, point.z]));
    }
}

impl SurrealValue for RsAabb {
    fn kind_of() -> surrealdb::types::Kind {
        surrealdb::types::Kind::Array(Box::new(surrealdb::types::Kind::Number), None)
    }
    
    fn into_value(self) -> surrealdb::types::Value {
        surrealdb::types::Value::Array(surrealdb::types::Array::from(vec![
            // mins: x, y, z
            surrealdb::types::Value::Number(surrealdb::types::Number::Float(self.0.mins.x as f64)),
            surrealdb::types::Value::Number(surrealdb::types::Number::Float(self.0.mins.y as f64)),
            surrealdb::types::Value::Number(surrealdb::types::Number::Float(self.0.mins.z as f64)),
            // maxs: x, y, z
            surrealdb::types::Value::Number(surrealdb::types::Number::Float(self.0.maxs.x as f64)),
            surrealdb::types::Value::Number(surrealdb::types::Number::Float(self.0.maxs.y as f64)),
            surrealdb::types::Value::Number(surrealdb::types::Number::Float(self.0.maxs.z as f64)),
        ]))
    }
    
    fn from_value(value: surrealdb::types::Value) -> anyhow::Result<Self> {
        match value {
            surrealdb::types::Value::Array(arr) => {
                if arr.len() != 6 {
                    return Err(anyhow::anyhow!("数组长度必须为 6 才能转换为 RsAabb (mins x,y,z + maxs x,y,z)"));
                }
                
                let mins_x = match &arr[0] {
                    surrealdb::types::Value::Number(n) => n.to_f64().unwrap_or(0.0) as f32,
                    _ => return Err(anyhow::anyhow!("Aabb mins.x 必须是数字")),
                };
                let mins_y = match &arr[1] {
                    surrealdb::types::Value::Number(n) => n.to_f64().unwrap_or(0.0) as f32,
                    _ => return Err(anyhow::anyhow!("Aabb mins.y 必须是数字")),
                };
                let mins_z = match &arr[2] {
                    surrealdb::types::Value::Number(n) => n.to_f64().unwrap_or(0.0) as f32,
                    _ => return Err(anyhow::anyhow!("Aabb mins.z 必须是数字")),
                };
                let maxs_x = match &arr[3] {
                    surrealdb::types::Value::Number(n) => n.to_f64().unwrap_or(0.0) as f32,
                    _ => return Err(anyhow::anyhow!("Aabb maxs.x 必须是数字")),
                };
                let maxs_y = match &arr[4] {
                    surrealdb::types::Value::Number(n) => n.to_f64().unwrap_or(0.0) as f32,
                    _ => return Err(anyhow::anyhow!("Aabb maxs.y 必须是数字")),
                };
                let maxs_z = match &arr[5] {
                    surrealdb::types::Value::Number(n) => n.to_f64().unwrap_or(0.0) as f32,
                    _ => return Err(anyhow::anyhow!("Aabb maxs.z 必须是数字")),
                };
                
                let mins = Vec3::new(mins_x, mins_y, mins_z);
                let maxs = Vec3::new(maxs_x, maxs_y, maxs_z);
                Ok(RsAabb(Aabb::new(mins.into(), maxs.into())))
            }
            _ => Err(anyhow::anyhow!("值必须是数组类型才能转换为 RsAabb")),
        }
    }
}

impl From<Aabb> for RsAabb {
    fn from(aabb: Aabb) -> Self {
        Self(aabb)
    }
}

impl From<RsAabb> for Aabb {
    fn from(rs_aabb: RsAabb) -> Self {
        rs_aabb.0
    }
}
