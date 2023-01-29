use glam::{Quat, Vec3};
use parry3d::bounding_volume::Aabb;
use serde::{Serialize,Deserialize};

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct RvmGeoInfo{
    pub _key: String,
    pub aabb: Option<Aabb>,
    // 相对世界坐标系下的变换矩阵 rot, translation, scale
    pub world_transform: (Quat, Vec3, Vec3),
}