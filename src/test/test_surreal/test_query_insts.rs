use crate::rs_surreal::query_insts;
use crate::{RefnoEnum, SUL_DB, SurrealQueryExt};

use super::test_helpers::init_sul_db_with_memory;

/// 验证 query_insts 在内存版 SurrealDB 上能够返回最小化的几何实例数据。
#[tokio::test]
async fn query_insts_returns_single_instance_from_mem_db() -> anyhow::Result<()> {
    // 使用 kv-mem 引擎初始化全局 SUL_DB，避免依赖外部 Surreal 服务。
    init_sul_db_with_memory().await?;

    // 构造最小化的 inst_relate/inst_geo 数据，覆盖 query_insts SQL 所需字段。
    let setup_sql = r#"
        CREATE inst_geo:demo_inst CONTENT {
            meshed: true,
            visible: true,
            ptset: {
                d: {
                    pt: [
                        [0.0, 0.0, 0.0],
                        [1.0, 0.0, 0.0]
                    ]
                }
            }
        };

        CREATE inst_relate:17496_100 CONTENT {
            in: {
                id: "17496_100",
                owner: "17496/1",
                old_pe: none
            },
            out: inst_geo:demo_inst,
            generic: "PIPE",
            booled_id: "inst_geo:demo_hash",
            aabb: {
                d: {
                    mins: [0.0, 0.0, 0.0],
                    maxs: [1.0, 1.0, 1.0]
                }
            },
            world_trans: {
                d: {
                    translation: [0.0, 0.0, 0.0],
                    rotation: [0.0, 0.0, 0.0, 1.0],
                    scale: [1.0, 1.0, 1.0]
                }
            },
            dt: time::now()
        };
    "#;

    SUL_DB.query_response(setup_sql).await?;

    let refno: RefnoEnum = "17496/100".into();
    let insts = query_insts(&[refno.clone()], true).await?;

    assert_eq!(insts.len(), 1, "应该返回一条几何实例记录");
    let inst = &insts[0];

    assert_eq!(inst.refno, refno);
    assert_eq!(inst.generic, "PIPE");
    let aabb = inst.world_aabb.as_ref().expect("world_aabb should exist");
    let mins = aabb.mins();
    assert!(
        [mins.0.x, mins.0.y, mins.0.z]
            .iter()
            .all(|v| (*v - 0.0).abs() < f32::EPSILON)
    );
    let maxs = aabb.maxs();
    assert!(
        [maxs.0.x, maxs.0.y, maxs.0.z]
            .iter()
            .all(|v| (*v - 1.0).abs() < f32::EPSILON)
    );
    assert_eq!(inst.insts.len(), 1, "应该生成一条几何实体引用");
    assert_eq!(inst.insts[0].geo_hash, "inst_geo:demo_hash");
    assert!(inst.has_neg, "存在 booled_id 时应标记为含有布尔实体");
    assert!(inst.pts.as_ref().is_some_and(|pts| !pts.is_empty()));

    Ok(())
}
