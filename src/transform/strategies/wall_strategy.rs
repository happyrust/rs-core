/// WALL 策略实现模块
/// 专门处理 STWALL、SCTN 等墙体/截面构件的方向和位置计算
/// 这些类型的 BANG 属性影响 local transform，而不是几何体本身
use super::{BangHandler, NposHandler, TransformStrategy};
use crate::NamedAttrMap;
use crate::rs_surreal::spatial::construct_basis_z_y_exact;
use async_trait::async_trait;
use glam::{DMat4, DQuat, DVec3};
use std::sync::Arc;

pub struct WallStrategy {
    att: Arc<NamedAttrMap>,
    parent_att: Arc<NamedAttrMap>,
}

impl WallStrategy {
    pub fn new(att: Arc<NamedAttrMap>, parent_att: Arc<NamedAttrMap>) -> Self {
        Self { att, parent_att }
    }

    /// 计算墙体/截面的方向向量
    /// 使用 DPOSE 和 DPOSS 计算扫掠方向作为 Z 轴
    /// 对于没有这些属性的类型（如部分 SCTN），返回 None 使用默认方向
    fn calculate_wall_direction(&self) -> Option<DVec3> {
        if let Some(end) = self.att.get_dpose()
            && let Some(start) = self.att.get_dposs()
        {
            Some((end - start).normalize())
        } else {
            // 对于 SCTN 等类型，可能没有 DPOSE/DPOSS，使用默认方向
            // 只有 STWALL 类型缺少这些属性时才记录警告
            let type_name = self.att.get_type_str();
            if type_name == "STWALL" {
                let refno = self.att.get_refno().unwrap_or_default();
                tracing::warn!(
                    "STWALL {} 缺少方向定义数据: DPOSE={:?}, DPOSS={:?}",
                    refno,
                    self.att.get_dpose(),
                    self.att.get_dposs()
                );
            }
            None
        }
    }
}

#[async_trait]
impl TransformStrategy for WallStrategy {
    async fn get_local_transform(&mut self) -> anyhow::Result<Option<DMat4>> {
        let att = &self.att;
        let parent_att = &self.parent_att;

        // 1. 虚拟节点检查
        if att.get_bool("IS_VIRTUAL").unwrap_or(false) {
            return Ok(Some(DMat4::IDENTITY));
        }

        // 2. 获取基础位置
        let mut position = att.get_position().unwrap_or_default().as_dvec3();

        // 3. 处理 NPOS 偏移
        NposHandler::apply_npos_offset(&mut position, att);

        // 4. 计算墙体方向
        let mut rotation = DQuat::IDENTITY;

        if let Some(z_direction) = self.calculate_wall_direction() {
            // Y 轴指向上方向 (U)
            let y_axis = DVec3::Z;
            // Z 轴使用计算出的扫掠方向
            let z_axis = z_direction;

            rotation = construct_basis_z_y_exact(y_axis, z_axis);
        }

        // 5. 应用 BANG 旋转到 local transform
        // BANG 影响的是 local transform，而不是几何体本身的旋转
        BangHandler::apply_bang(&mut rotation, att);

        // 6. 构造最终的变换矩阵
        let mat4 = DMat4::from_rotation_translation(rotation, position);

        Ok(Some(mat4))
    }
}
