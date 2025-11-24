/// WALL 策略实现模块
/// 专门处理 STWALL 等墙体构件的方向和位置计算

use super::{TransformStrategy, NposHandler};
use crate::rs_surreal::spatial::construct_basis_z_y_exact;
use crate::NamedAttrMap;
use async_trait::async_trait;
use glam::{DMat4, DQuat, DVec3};

pub struct WallStrategy {
    att: NamedAttrMap,
    parent_att: NamedAttrMap,
}

impl WallStrategy {
    pub fn new(att: NamedAttrMap, parent_att: NamedAttrMap) -> Self {
        Self { att, parent_att }
    }

    /// 计算墙体的方向向量
    /// 使用 DPOSE 和 DPOSS 计算扫掠方向作为 Z 轴
    fn calculate_wall_direction(&self) -> Option<DVec3> {
        if let Some(end) = self.att.get_dpose()
            && let Some(start) = self.att.get_dposs()
        {
            Some((end - start).normalize())
        } else {
            // 记录警告：缺少方向定义数据
            let refno = self.att.get_refno().unwrap_or_default();
            tracing::warn!(
                "STWALL {} 缺少方向定义数据: DPOSE={:?}, DPOSS={:?}", 
                refno,
                self.att.get_dpose(),
                self.att.get_dposs()
            );
            None
        }
    }
}

#[async_trait]
impl TransformStrategy for WallStrategy {
    async fn get_local_transform(
        &mut self,
    ) -> anyhow::Result<Option<DMat4>> {
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

        // 5. 构造最终的变换矩阵
        let mat4 = DMat4::from_rotation_translation(rotation, position);
        
        Ok(Some(mat4))
    }
}
