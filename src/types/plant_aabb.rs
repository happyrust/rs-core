use anyhow::anyhow;
use glam::Vec3;
use parry3d::bounding_volume::{Aabb, BoundingVolume};
use parry3d::math::Point;
use serde::{Deserialize, Serialize};
use surrealdb::types::{SurrealValue, Kind, Value};

use crate::shape::pdms_shape::RsVec3;

/// Aabb包装类型，为Aabb实现SurrealValue
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct PlantAabb(pub Aabb);

impl Default for PlantAabb {
    fn default() -> Self {
        Self(Aabb::new_invalid())
    }
}

impl SurrealValue for PlantAabb {
    fn kind_of() -> Kind {
        Kind::Object
    }

    fn into_value(self) -> Value {
        serde_json::to_value(&self.0)
            .expect("序列化 PlantAabb 失败")
            .into_value()
    }

    fn from_value(value: Value) -> anyhow::Result<Self> {
        let json = serde_json::Value::from_value(value)?;
        Ok(PlantAabb(serde_json::from_value(json)?))
    }
}

impl PlantAabb {
    /// 创建新的PlantAabb
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
    pub fn merge(&self, other: &PlantAabb) -> Self {
        Self(self.0.merged(&other.0))
    }

    /// 检查是否包含点
    pub fn contains_point(&self, point: &Vec3) -> bool {
        self.0
            .contains_local_point(&Point::from([point.x, point.y, point.z]))
    }

    /// 扩展Aabb以包含给定点
    pub fn take_point(&mut self, point: &Vec3) {
        self.0.take_point(Point::from([point.x, point.y, point.z]));
    }

    /// 按缩放因子缩放AABB
    pub fn scaled(&self, scale: &Vec3) -> Self {
        let mins = self.0.mins.coords;
        let maxs = self.0.maxs.coords;
        Self(Aabb::new(
            Point::from([mins.x * scale.x, mins.y * scale.y, mins.z * scale.z]),
            Point::from([maxs.x * scale.x, maxs.y * scale.y, maxs.z * scale.z]),
        ))
    }

    /// 通过等距变换转换AABB
    pub fn transform_by(&self, iso: &parry3d::math::Isometry<f32>) -> Aabb {
        self.0.transform_by(iso)
    }
}

impl From<Aabb> for PlantAabb {
    fn from(aabb: Aabb) -> Self {
        Self(aabb)
    }
}

impl From<PlantAabb> for Aabb {
    fn from(rs_aabb: PlantAabb) -> Self {
        rs_aabb.0
    }
}
