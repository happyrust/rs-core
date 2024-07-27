use std::collections::HashMap;
use surrealdb::Surreal;
use surrealdb::engine::any::Any;
use crate::aios_db_mgr::aios_mgr::AiosDBMgr;
use crate::material::dq::save_dq_material;
use crate::material::gps::save_gps_material_dzcl;
use crate::material::gy::{save_gy_material_dzcl, save_gy_material_equi, save_gy_material_valv};
use crate::material::nt::save_nt_material_dzcl;
use crate::material::sb::save_sb_material_dzcl;
use crate::material::tf::save_tf_material_hvac;
use crate::material::tx::save_tx_material_equi;
use crate::material::yk::{save_yk_material_dzcl, save_yk_material_equi, save_yk_material_pipe};
use crate::ssc_setting::{query_all_site_with_major, set_pdms_major_code};
use crate::{query_filter_ancestors, RefU64, SUL_DB};
use crate::pdms_user::RefnoMajor;

pub mod dq;
pub mod gy;
pub mod gps;
pub mod yk;
pub mod tf;
pub mod tx;
pub mod sb;
pub mod nt;
pub(crate) mod query;

/// 保存所有的材料表单数据
pub async fn save_all_material_data(aios_mgr: &AiosDBMgr) -> anyhow::Result<()> {
    // 生成专业代码
    set_pdms_major_code(&aios_mgr).await?;
    // 提前跑已经创建surreal的方法
    if let Err(e) = define_surreal_functions(SUL_DB.clone()).await {
        dbg!(e.to_string());
        return Ok(());
    }
    let mut handles = Vec::new();
    // 查找所有带专业的site
    let sites = query_all_site_with_major().await?;
    // 处理所有专业表单的数据
    for site in sites {
        dbg!(&site.id);
        match site.major.as_str() {
            // 工艺
            "T" => {
                // 大宗材料
                println!("工艺布置专业-大宗材料");
                save_gy_material_dzcl(site.id, SUL_DB.clone(), aios_mgr, &mut handles).await;
                // 设备清单
                println!("工艺布置专业-设备清单");
                save_gy_material_equi(site.id, SUL_DB.clone(), aios_mgr, &mut handles).await;
                // 阀门清单
                println!("工艺布置专业-阀门清单");
                save_gy_material_valv(site.id, SUL_DB.clone(), aios_mgr, &mut handles).await;
            }
            // 仪控
            "I" => {
                // 大宗材料
                println!("仪控专业-大宗材料");
                save_yk_material_dzcl(site.id, SUL_DB.clone(), aios_mgr, &mut handles).await;
                // 仪表管道
                println!("仪控专业-仪表管道");
                save_yk_material_pipe(site.id, SUL_DB.clone(), aios_mgr, &mut handles).await;
                // 设备清单
                println!("仪控专业-设备清单");
                save_yk_material_equi(site.id, SUL_DB.clone(), aios_mgr, &mut handles).await;
            }
            // 通风
            "V" => {
                // 风管管段
                println!("通风专业-风管管段");
                save_tf_material_hvac(site.id, SUL_DB.clone(), aios_mgr, &mut handles).await;
            }
            // 电气
            "E" => {
                // 托盘及接地
                println!("电气专业-托盘及接地");
                save_dq_material(site.id, SUL_DB.clone(), aios_mgr, &mut handles).await;
            }
            // 通信
            "TX" => {
                // 通信系统
                println!("通信专业-通信系统");
                save_tx_material_equi(site.id, SUL_DB.clone(), aios_mgr, &mut handles).await;
            }
            // 给排水
            "W" => {
                // 大宗材料
                println!("给排水专业-大宗材料");
                save_gps_material_dzcl(site.id, SUL_DB.clone(), aios_mgr, &mut handles).await;
            }
            // 设备
            "EQUI" => {
                // 大宗材料
                println!("设备专业-大宗材料");
                save_sb_material_dzcl(site.id, SUL_DB.clone(), aios_mgr, &mut handles).await;
            }
            // 暖通
            "N" => {
                // 阀门清单
                println!("暖通专业-阀门清单");
                save_nt_material_dzcl(site.id, SUL_DB.clone(), aios_mgr, &mut handles).await;
            }
            _ => {}
        }
    }
    // 等待保存线程完成
    println!("查询完毕，等待数据库保存完成");
    futures::prelude::future::join_all(handles).await;
    Ok(())
}

/// 提前运行定义好的方法
pub async fn define_surreal_functions(db: Surreal<Any>) -> anyhow::Result<()> {
    let response = db
        .query(include_str!("../rs_surreal/material_list/default_name.surql"))
        .await?;
    let response = db
        .query(include_str!("../rs_surreal/material_list/dq/fn_dq_bran_type.surql"))
        .await?;
    let response = db
        .query(include_str!("../rs_surreal/material_list/dq/fn_vec3_distance.surql"))
        .await?;
    let response = db
        .query(include_str!("../rs_surreal/material_list/yk/fn_find_gy_bran.surql"))
        .await?;
    let response = db
        .query(include_str!("../rs_surreal/material_list/gy/fn_b_valv_supp.surql"))
        .await?;
    let response = db
        .query(include_str!(
            "../rs_surreal/material_list/dq/fn_dq_horizontal_or_vertical.surql"
        ))
        .await?;
    let response = db
        .query(include_str!("../rs_surreal/material_list/fn_get_ancestor.surql"))
        .await?;
    let response = db
        .query(include_str!(
            "../rs_surreal/material_list/sb/fn_find_group_sube_children.surql"
        ))
        .await?;
    let response = db
        .query(include_str!("../rs_surreal/material_list/nt/fn_get_valv_material.surql"))
        .await?;
    let response = db
        .query(include_str!("../rs_surreal/material_list/fn_get_world_pos.surql"))
        .await?;
    let response = db
        .query(include_str!("../rs_surreal/schemas/fn_query_room_code.surql"))
        .await?;
    db.query(include_str!("../rs_surreal/tools/bolt.surql")).await?;
    db.query(include_str!("../rs_surreal/tools/common.surql")).await?;
    db.query(include_str!("../rs_surreal/tools/fln.surql")).await?;
    db.query(include_str!("../rs_surreal/tools/formula.surql")).await?;
    db.query(include_str!("../rs_surreal/tools/hvac.surql")).await?;
    db.query(include_str!("../rs_surreal/tools/len.surql")).await?;
    db.query(include_str!("../rs_surreal/tools/stif.surql")).await?;
    db.query(include_str!("../rs_surreal/tools/washer.surql")).await?;
    Ok(())
}

/// 查询节点属于哪个专业和专业下的具体分类
pub async fn get_refnos_belong_major(
    refnos: &Vec<RefU64>,
) -> anyhow::Result<HashMap<RefU64, RefnoMajor>> {
    let mut result = HashMap::new();
    for refno in refnos {
        // 向上找到zone
        let zone = query_filter_ancestors(*refno, &["ZONE"]).await?;
        if zone.is_empty() { continue; };
        let zone = zone[0];
        // 找zone和site对应的专业
        let sql = format!("select value major from type::thing('pdms_major',meta::id({}));
        select value major from type::thing('pdms_major',meta::id((select value ->pe_owner.out.refno from {})[0][0]));", zone.to_pe_key(), zone.to_pe_key());
        let Ok(mut response) = SUL_DB.query(sql).await else { continue; };
        let zone_major: Vec<String> = response.take(0)?;
        let site_major: Vec<String> = response.take(1)?;
        if zone_major.is_empty() || site_major.is_empty() { continue; };
        result.entry(*refno).or_insert(RefnoMajor {
            refno: refno.to_pdms_str(),
            major: site_major[0].clone(),
            major_classify: zone_major[0].clone(),
        });
    }
    Ok(result)
}