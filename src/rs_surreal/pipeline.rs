use std::collections::HashMap;

use anyhow::Result;
use bevy_transform::components::Transform;
use futures::future::try_join_all;
use glam::Vec3;

use crate::{
    NamedAttrMap, NamedAttrValue, RefU64, RefnoEnum, get_named_attmap, parsed_data::CateAxisParam,
    pdms_pluggin::heat_dissipation::InstPointMap, pdms_types::PdmsGenericType,
};

use super::{
    geom::query_refnos_point_map, inst::query_tubi_insts_by_brans,
    point::query_arrive_leave_points_of_branch, query::query_bran_fixing_length,
};

/// 管道端口角色枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortRole {
    Arrive,
    Leave,
    Extra,
}

/// 标注所需的端口数据
#[derive(Debug, Clone, Copy)]
pub struct PipelinePort {
    pub number: i32,
    pub role: PortRole,
    pub world_pos: Vec3,
    pub world_dir: Option<Vec3>,
}

impl PipelinePort {
    fn from_axis(axis: &CateAxisParam, role: PortRole, override_number: Option<i32>) -> Self {
        Self {
            number: override_number.unwrap_or(axis.number),
            role,
            world_pos: axis.pt.0,
            world_dir: axis.dir.as_ref().map(|dir| dir.0),
        }
    }
}

/// 用于描述一段测量跨度（主干或支管）
#[derive(Debug, Clone, Copy)]
pub struct PipelineSpan {
    pub start: PipelinePort,
    pub end: PipelinePort,
    /// 真实长度（含弧长补偿）
    pub length: f32,
    /// 起止点的直线距离
    pub straight_length: f32,
}

/// 单段管件聚合信息
#[derive(Debug, Clone)]
pub struct PipelineSegmentRecord {
    pub refno: RefnoEnum,
    pub branch: RefnoEnum,
    pub noun: PdmsGenericType,
    pub noun_raw: Option<String>,
    pub type_name: Option<String>,
    pub name: Option<String>,
    pub spec: Option<String>,
    pub attrs: NamedAttrMap,
    pub transform: Transform,
    pub geo_hash: String,
    pub arrive_number: Option<i32>,
    pub leave_number: Option<i32>,
    pub arrive: Option<PipelinePort>,
    pub leave: Option<PipelinePort>,
    pub extra_ports: Vec<PipelinePort>,
    pub length: f32,
    pub straight_length: f32,
}

impl PipelineSegmentRecord {
    /// 判断是否为指定类型（兼容 noun_raw 的字符串差异）
    pub fn is_kind(&self, kind: PdmsGenericType) -> bool {
        if self.noun == kind {
            return true;
        }
        if let Some(raw) = &self.noun_raw {
            if let Ok(parsed) = raw.parse::<PdmsGenericType>() {
                return parsed == kind;
            }
        }
        false
    }

    /// 主干跨度（ARRI -> LEAV）
    pub fn main_span(&self) -> Option<PipelineSpan> {
        match (self.arrive, self.leave) {
            (Some(start), Some(end)) => Some(PipelineSpan {
                start,
                end,
                length: self.length,
                straight_length: self.straight_length,
            }),
            _ => None,
        }
    }

    /// 支管跨度（主端口与 extra 之间）
    pub fn branch_spans(&self) -> Vec<PipelineSpan> {
        let mut spans = Vec::new();
        if let Some(main_start) = self.arrive {
            spans.extend(self.extra_ports.iter().map(|port| PipelineSpan {
                start: main_start,
                end: *port,
                length: main_start.world_pos.distance(port.world_pos),
                straight_length: main_start.world_pos.distance(port.world_pos),
            }));
        }
        if let Some(main_end) = self.leave {
            spans.extend(self.extra_ports.iter().map(|port| PipelineSpan {
                start: main_end,
                end: *port,
                length: main_end.world_pos.distance(port.world_pos),
                straight_length: main_end.world_pos.distance(port.world_pos),
            }));
        }
        spans
    }

    /// 所有端口（ARRI、LEAV 和额外支管）
    pub fn all_ports(&self) -> impl Iterator<Item = PipelinePort> + '_ {
        self.arrive
            .into_iter()
            .chain(self.leave)
            .chain(self.extra_ports.iter().copied())
    }
}

/// 提供 BRAN 维度数据的聚合查询服务
pub struct PipelineQueryService;

impl PipelineQueryService {
    /// 预拉取单个 BRAN 下所有管件的尺寸标注数据
    pub async fn fetch_branch_segments(
        branch_refno: RefnoEnum,
    ) -> Result<Vec<PipelineSegmentRecord>> {
        let tubi_insts = query_tubi_insts_by_brans(&[branch_refno]).await?;
        if tubi_insts.is_empty() {
            return Ok(Vec::new());
        }

        // 收集所有唯一的 refno（使用 inst.refno，不是 inst.leave）
        let mut unique_refnos: Vec<RefnoEnum> =
            tubi_insts.iter().map(|inst| inst.refno.clone()).collect();
        unique_refnos.dedup();

        let refno_ids: Vec<RefU64> = unique_refnos.iter().map(|refno| refno.refno()).collect();

        // 获取 arrive/leave 点对（已包含世界坐标）
        let arrive_leave_pairs = query_arrive_leave_points_of_branch(branch_refno).await?;

        // 获取点映射（用于多端口元件如三通）
        let point_maps: HashMap<RefnoEnum, InstPointMap> =
            query_refnos_point_map(unique_refnos.clone()).await?;

        // 批量获取属性
        let attr_pairs = try_join_all(unique_refnos.iter().map(|refno| {
            let refno_clone = refno.clone();
            async move {
                let attrs = get_named_attmap(refno_clone).await?;
                Ok::<(RefnoEnum, NamedAttrMap), anyhow::Error>((refno_clone, attrs))
            }
        }))
        .await?;
        let attr_map: HashMap<RefnoEnum, NamedAttrMap> = attr_pairs.into_iter().collect();

        // 批量获取长度
        let length_pairs = try_join_all(refno_ids.iter().map(|refno| {
            let refno_copy = *refno;
            async move {
                let length = query_bran_fixing_length(refno_copy).await?;
                Ok::<(RefU64, f32), anyhow::Error>((refno_copy, length))
            }
        }))
        .await?;
        let length_map: HashMap<RefU64, f32> = length_pairs.into_iter().collect();

        let mut records = Vec::with_capacity(tubi_insts.len());
        for inst in tubi_insts {
            let super::inst::TubiInstQuery {
                refno,
                generic,
                world_trans,
                geo_hash,
                ..
            } = inst;

            let attrs = attr_map.get(&refno).cloned().unwrap_or_default();

            let mut arrive_number = attr_to_i32(&attrs, "ARRI");
            let mut leave_number = attr_to_i32(&attrs, "LEAV");

            // 从 arrive_leave_pairs 获取主端口
            let mut arrive_port = None;
            let mut leave_port = None;
            if let Some(axes_ref) = arrive_leave_pairs.get(&refno) {
                let axes = axes_ref.value();
                arrive_number = arrive_number.or(Some(axes[0].number));
                leave_number = leave_number.or(Some(axes[1].number));
                arrive_port = Some(PipelinePort::from_axis(
                    &axes[0],
                    PortRole::Arrive,
                    arrive_number,
                ));
                leave_port = Some(PipelinePort::from_axis(
                    &axes[1],
                    PortRole::Leave,
                    leave_number,
                ));
            }

            // 获取所有端口（用于三通等多端口元件）
            let mut world_axes: Vec<CateAxisParam> = point_maps
                .get(&refno)
                .map(|map| {
                    map.ptset_map
                        .values()
                        .map(|axis| axis.transformed(&*world_trans))
                        .collect()
                })
                .unwrap_or_default();
            world_axes.sort_by_key(|axis| axis.number);

            // 处理额外端口（不是 ARRI/LEAV 的端口）
            let mut extra_ports = Vec::new();
            for axis in &world_axes {
                let number = axis.number;
                if arrive_number == Some(number) {
                    if arrive_port.is_none() {
                        arrive_port = Some(PipelinePort::from_axis(
                            axis,
                            PortRole::Arrive,
                            Some(number),
                        ));
                    }
                    continue;
                }
                if leave_number == Some(number) {
                    if leave_port.is_none() {
                        leave_port =
                            Some(PipelinePort::from_axis(axis, PortRole::Leave, Some(number)));
                    }
                    continue;
                }
                extra_ports.push(PipelinePort::from_axis(axis, PortRole::Extra, Some(number)));
            }

            // 如果没有找到 arrive 端口，使用第一个端口
            if arrive_port.is_none() {
                if let Some(axis) = world_axes.first() {
                    arrive_number = Some(axis.number);
                    arrive_port = Some(PipelinePort::from_axis(
                        axis,
                        PortRole::Arrive,
                        Some(axis.number),
                    ));
                }
            }
            // 如果没有找到 leave 端口，使用与 arrive 不同的第一个端口
            if leave_port.is_none() {
                if let Some(axis) = world_axes
                    .iter()
                    .find(|axis| Some(axis.number) != arrive_number)
                {
                    leave_number = Some(axis.number);
                    leave_port = Some(PipelinePort::from_axis(
                        axis,
                        PortRole::Leave,
                        Some(axis.number),
                    ));
                }
            }

            // 计算长度
            let straight_length = match (arrive_port.as_ref(), leave_port.as_ref()) {
                (Some(a), Some(l)) => a.world_pos.distance(l.world_pos),
                _ => 0.0,
            };
            let length = match length_map.get(&refno.refno()).copied() {
                Some(len) if len > 0.0 => len,
                _ => straight_length,
            };

            // 提取属性
            let type_name = attr_to_string(&attrs, "TYPE");
            let name = attr_to_string(&attrs, "NAME");
            let spec = attr_to_string(&attrs, "SPEC");

            // 解析 noun 类型
            let noun = generic
                .as_deref()
                .and_then(|g| g.parse::<PdmsGenericType>().ok())
                .unwrap_or(PdmsGenericType::UNKOWN);

            records.push(PipelineSegmentRecord {
                refno,
                branch: branch_refno.clone(),
                noun,
                noun_raw: generic,
                type_name,
                name,
                spec,
                attrs,
                transform: *world_trans,
                geo_hash,
                arrive_number,
                leave_number,
                arrive: arrive_port,
                leave: leave_port,
                extra_ports,
                length,
                straight_length,
            });
        }

        Ok(records)
    }
}

fn attr_to_i32(attrs: &NamedAttrMap, key: &str) -> Option<i32> {
    attr_value(attrs, key).and_then(|value| match value {
        NamedAttrValue::IntegerType(v) => Some(*v),
        NamedAttrValue::LongType(v) => Some(*v as i32),
        NamedAttrValue::F32Type(v) => Some(*v as i32),
        _ => None,
    })
}

fn attr_to_string(attrs: &NamedAttrMap, key: &str) -> Option<String> {
    attr_value(attrs, key).and_then(|value| match value {
        NamedAttrValue::StringType(v)
        | NamedAttrValue::WordType(v)
        | NamedAttrValue::ElementType(v) => Some(v.clone()),
        _ => None,
    })
}

fn attr_value<'a>(attrs: &'a NamedAttrMap, key: &str) -> Option<&'a NamedAttrValue> {
    attrs.get(key).or_else(|| {
        let alt = format!(":{key}");
        attrs.get(alt.as_str())
    })
}
