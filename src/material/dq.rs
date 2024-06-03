#[cfg(feature = "sql")]
use super::query::create_table_sql;
#[cfg(feature = "sql")]
use super::query::save_material_value;

use crate::aios_db_mgr::aios_mgr::AiosDBMgr;
use crate::{
    get_children_pes, get_pe, insert_into_table_with_chunks, query_filter_deep_children, RefU64,
};
use calamine::{open_workbook, RangeDeserializerBuilder, Reader, Xls};
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;
use surrealdb::engine::any::Any;
use surrealdb::Surreal;
use tokio::task::{self, JoinHandle};

/// 电气专业 托盘及接地
pub async fn save_dq_material(
    refno: RefU64,
    db: Surreal<Any>,
    aios_mgr: &AiosDBMgr,
    mut handles: &mut Vec<JoinHandle<()>>,
) {
    match get_dq_bran_list(db.clone(), vec![refno]).await {
        Ok((mut r, mut str_r)) => {
            let material_data = read_dq_material_excel().unwrap_or_default();
            get_dq_value_from_material(&material_data, &mut r);
            get_dq_value_from_material_stru(&material_data, &mut str_r);

            let r_clone = r.clone();
            let str_r_clone = str_r.clone();
            let db_clone = db.clone();
            let task = task::spawn(async move {
                match insert_into_table_with_chunks(&db_clone, "material_elec_list", r_clone).await
                {
                    Ok(_) => {}
                    Err(e) => {
                        dbg!(&e.to_string());
                    }
                }
            });
            handles.push(task);
            let task = task::spawn(async move {
                match insert_into_table_with_chunks(&db, "material_elec_list", str_r_clone).await {
                    Ok(_) => {}
                    Err(e) => {
                        dbg!(&e.to_string());
                    }
                }
            });
            handles.push(task);
            #[cfg(feature = "sql")]
            {
                let Ok(pool) = aios_mgr.get_project_pool().await else {
                    dbg!("无法连接到数据库");
                    return;
                };
                let task = task::spawn(async move {
                    let table_name = "电气专业_托盘及接地".to_string();
                    let filed = vec![
                        "参考号".to_string(),
                        "机组号".to_string(),
                        "元件等级名称".to_string(),
                        "子项号".to_string(),
                        "子项名称".to_string(),
                        "专业".to_string(),
                        "房间号".to_string(),
                        "托盘段号".to_string(),
                        "托盘标高".to_string(),
                        "托盘类型".to_string(),
                        "托盘支吊架名称".to_string(),
                        "标准号".to_string(),
                        "物项编号".to_string(),
                        "材质".to_string(),
                        "托盘宽度mm".to_string(),
                        "托盘高度mm".to_string(),
                        "规格型号".to_string(),
                        "是否刷漆".to_string(),
                        "刷漆颜色".to_string(),
                        "有无盖板".to_string(),
                        "有无隔板".to_string(),
                        "隔板编号".to_string(),
                        "共用隔板的托盘序号".to_string(),
                        "水平/竖向".to_string(),
                        "单位".to_string(),
                        "数量".to_string(),
                    ];
                    match create_table_sql(&pool, &table_name, &filed).await {
                        Ok(_) => {
                            if !r.is_empty() {
                                let data = r
                                    .into_iter()
                                    .map(|x| x.into_hashmap())
                                    .collect::<Vec<HashMap<String, String>>>();
                                match save_material_value(&pool, &table_name, &filed, data).await {
                                    Ok(_) => {}
                                    Err(e) => {
                                        dbg!(&e.to_string());
                                    }
                                }
                            }
                            if !str_r.is_empty() {
                                let data = str_r
                                    .into_iter()
                                    .map(|x| x.into_hashmap())
                                    .collect::<Vec<HashMap<String, String>>>();
                                match save_material_value(&pool, &table_name, &filed, data).await {
                                    Ok(_) => {}
                                    Err(e) => {
                                        dbg!(&e.to_string());
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            dbg!(&e.to_string());
                        }
                    }
                });
                handles.push(task);
            }
        }
        Err(e) => {
            dbg!(e.to_string());
        }
    }
}

/// 读取电气专业材料表
fn read_dq_material_excel() -> anyhow::Result<HashMap<String, Vec<DqMaterial>>> {
    let mut map = HashMap::new();
    let mut workbook: Xls<_> = open_workbook("resource/电气专业大宗材料属性对应关系表.xls")?;
    let range = workbook.worksheet_range("电气专业大宗材料属性对应关系表").unwrap()?;

    let mut iter = RangeDeserializerBuilder::new().from_range(&range)?;

    while let Some(result) = iter.next() {
        let value: DqMaterial = result?;
        if value.catr_name.is_some() {
            map.entry(value.catr_name.clone().unwrap())
                .or_insert_with(Vec::new)
                .push(value);
        }
    }
    Ok(map)
}

// 将材料表的 标准号 和 单位 填入其中
fn get_dq_value_from_material(
    material_data: &HashMap<String, Vec<DqMaterial>>,
    mut value: &mut Vec<MaterialDqMaterialList>,
) {
    for mut d in value.iter_mut() {
        if d.spre.is_some() && material_data.contains_key(&d.spre.clone().unwrap()) {
            let material_value = material_data.get(&d.spre.clone().unwrap()).unwrap();
            if material_value.is_empty() {
                continue;
            };
            let value = material_value[0].clone();
            d.stander_num = value.stander_num;
            d.unit = value.unit;
            d.item_num = value.code;
        } else if d.catr.is_some() && material_data.contains_key(&d.catr.clone().unwrap()) {
            let material_value = material_data.get(&d.catr.clone().unwrap()).unwrap();
            if material_value.is_empty() {
                continue;
            };
            let value = material_value[0].clone();
            d.stander_num = value.stander_num;
            d.unit = value.unit;
            d.item_num = value.code;
        }
    }
}

// 将材料表的标准号 物项编号 和单位填入其中
fn get_dq_value_from_material_stru(
    material_data: &HashMap<String, Vec<DqMaterial>>,
    mut value: &mut Vec<MaterialDqMaterialListStru>,
) {
    for mut d in value.iter_mut() {
        if d.spre.is_some() && material_data.contains_key(&d.spre.clone().unwrap()) {
            let material_value = material_data.get(&d.spre.clone().unwrap()).unwrap();
            if material_value.is_empty() {
                continue;
            };
            let value = material_value[0].clone();
            d.stander_num = value.stander_num;
            d.unit = value.unit;
            d.item_num = value.code;
        } else if d.catr.is_some() && material_data.contains_key(&d.catr.clone().unwrap()) {
            let material_value = material_data.get(&d.catr.clone().unwrap()).unwrap();
            if material_value.is_empty() {
                continue;
            };
            let value = material_value[0].clone();
            d.stander_num = value.stander_num;
            d.unit = value.unit;
            d.item_num = value.code;
        }
    }
}

/// 电气 托盘及接地
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialDqMaterialList {
    pub id: RefU64,
    pub num: Option<String>,
    pub project_num: Option<String>,
    pub project_name: Option<String>,
    pub major: String,
    pub room_code: Option<String>,
    pub name: String,
    pub pos: Option<f32>,
    pub bran_type: Option<String>,
    pub material: Option<String>,
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub size_num: Option<f32>,
    pub b_painting: Option<String>,
    pub painting_color: Option<String>,
    pub b_cover: Option<String>,
    pub b_partition: Option<String>,
    pub partition_num: Option<String>,
    pub spre: Option<String>,
    pub catr: Option<String>,
    pub horizontal_or_vertical: Option<String>,
    #[serde(default)]
    pub stander_num: Option<String>,
    #[serde(default)]
    pub item_num: Option<String>,
    #[serde(default)]
    pub unit: Option<String>,
    #[serde(default)]
    pub count: Option<f32>,
}

impl MaterialDqMaterialList {
    //// 将结构体转为HashMap
    pub fn into_hashmap(self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.entry("参考号".to_string())
            .or_insert(self.id.to_pdms_str());
        map.entry("机组号".to_string())
            .or_insert(self.num.unwrap_or("".to_string()));
        map.entry("子项号".to_string())
            .or_insert(self.project_num.unwrap_or("".to_string()));
        map.entry("元件等级名称".to_string())
            .or_insert(self.spre.clone().unwrap_or("".to_string()));
        map.entry("子项名称".to_string())
            .or_insert(self.project_name.unwrap_or("".to_string()));
        map.entry("专业".to_string()).or_insert(self.major);
        map.entry("房间号".to_string())
            .or_insert(self.room_code.unwrap_or("".to_string()));
        map.entry("托盘段号".to_string()).or_insert(self.name);
        map.entry("托盘标高".to_string())
            .or_insert(self.pos.unwrap_or(0.0).to_string());
        map.entry("托盘类型".to_string())
            .or_insert(self.bran_type.unwrap_or("".to_string()));
        map.entry("材质".to_string())
            .or_insert(self.material.unwrap_or("".to_string()));
        map.entry("托盘宽度mm".to_string())
            .or_insert(self.width.unwrap_or(0.0).to_string());
        map.entry("托盘高度mm".to_string())
            .or_insert(self.height.unwrap_or(0.0).to_string());
        map.entry("规格型号".to_string())
            .or_insert(self.size_num.unwrap_or(0.0).to_string());
        map.entry("是否刷漆".to_string())
            .or_insert(self.b_painting.unwrap_or("".to_string()));
        map.entry("刷漆颜色".to_string())
            .or_insert(self.painting_color.unwrap_or("".to_string()));
        map.entry("有无隔板".to_string())
            .or_insert(self.b_partition.unwrap_or("".to_string()));
        map.entry("隔板编号".to_string())
            .or_insert(self.partition_num.unwrap_or("".to_string()));
        map.entry("spre".to_string())
            .or_insert(self.spre.unwrap_or("".to_string()));
        map.entry("catr".to_string())
            .or_insert(self.catr.unwrap_or("".to_string()));
        map.entry("水平/竖向".to_string())
            .or_insert(self.horizontal_or_vertical.unwrap_or("".to_string()));
        map.entry("标准号".to_string())
            .or_insert(self.stander_num.unwrap_or("".to_string()));
        map.entry("物项编号".to_string())
            .or_insert(self.item_num.unwrap_or("".to_string()));
        map.entry("单位".to_string())
            .or_insert(self.unit.unwrap_or("".to_string()));
        map.entry("数量".to_string())
            .or_insert(self.count.unwrap_or(0.0).to_string());
        map
    }
}

/// 电气 托盘及接地
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialDqMaterialListStru {
    pub id: RefU64,
    pub num: Option<String>,
    pub project_num: Option<String>,
    pub project_name: Option<String>,
    pub major: String,
    pub room_code: Option<String>,
    pub pos: Option<f32>,
    pub material: Option<String>,
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub supp_name: Option<String>,
    pub size_num: Option<String>,
    pub b_painting: Option<String>,
    pub painting_color: Option<String>,
    pub b_cover: Option<String>,
    pub b_partition: Option<String>,
    pub partition_num: Option<String>,
    pub spre: Option<String>,
    pub catr: Option<String>,
    pub horizontal_or_vertical: Option<String>,
    #[serde(default)]
    pub stander_num: Option<String>,
    #[serde(default)]
    pub item_num: Option<String>,
    #[serde(default)]
    pub unit: Option<String>,
    #[serde(default)]
    pub count: Option<String>,
}

impl MaterialDqMaterialListStru {
    //// 将结构体转为HashMap
    pub fn into_hashmap(self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.entry("参考号".to_string())
            .or_insert(self.id.to_pdms_str());
        map.entry("机组号".to_string())
            .or_insert(self.num.unwrap_or("".to_string()));
        map.entry("子项号".to_string())
            .or_insert(self.project_num.unwrap_or("".to_string()));
        map.entry("元件等级名称".to_string())
            .or_insert(self.spre.clone().unwrap_or("".to_string()));
        map.entry("子项名称".to_string())
            .or_insert(self.project_name.unwrap_or("".to_string()));
        map.entry("专业".to_string()).or_insert(self.major);
        map.entry("房间号".to_string())
            .or_insert(self.room_code.unwrap_or("".to_string()));
        map.entry("托盘标高".to_string())
            .or_insert(self.pos.unwrap_or(0.0).to_string());
        map.entry("材质".to_string())
            .or_insert(self.material.unwrap_or("".to_string()));
        map.entry("托盘支吊架名称".to_string())
            .or_insert(self.supp_name.unwrap_or("".to_string()));
        map.entry("托盘宽度mm".to_string())
            .or_insert(self.width.unwrap_or(0.0).to_string());
        map.entry("托盘高度mm".to_string())
            .or_insert(self.height.unwrap_or(0.0).to_string());
        map.entry("规格型号".to_string())
            .or_insert(self.size_num.unwrap_or("".to_string()));
        map.entry("是否刷漆".to_string())
            .or_insert(self.b_painting.unwrap_or("".to_string()));
        map.entry("刷漆颜色".to_string())
            .or_insert(self.painting_color.unwrap_or("".to_string()));
        map.entry("有无隔板".to_string())
            .or_insert(self.b_partition.unwrap_or("".to_string()));
        map.entry("隔板编号".to_string())
            .or_insert(self.partition_num.unwrap_or("".to_string()));
        map.entry("spre".to_string())
            .or_insert(self.spre.unwrap_or("".to_string()));
        map.entry("catr".to_string())
            .or_insert(self.catr.unwrap_or("".to_string()));
        map.entry("水平/竖向".to_string())
            .or_insert(self.horizontal_or_vertical.unwrap_or("".to_string()));
        map.entry("标准号".to_string())
            .or_insert(self.stander_num.unwrap_or("".to_string()));
        map.entry("物项编号".to_string())
            .or_insert(self.item_num.unwrap_or("".to_string()));
        map.entry("单位".to_string())
            .or_insert(self.unit.unwrap_or("".to_string()));
        map
    }
}

/// 查询电气 托盘及接地 托盘
pub async fn get_dq_bran_list(
    db: Surreal<Any>,
    refnos: Vec<RefU64>,
) -> anyhow::Result<(Vec<MaterialDqMaterialList>, Vec<MaterialDqMaterialListStru>)> {
    let mut data = Vec::new();
    let mut stru_data = Vec::new();
    for refno in refnos {
        let Some(pe) = get_pe(refno).await? else {
            continue;
        };
        // 如果是site，则需要过滤 site的 name
        if pe.noun == "SITE".to_string() {
            if !pe.name.contains("ELEC") {
                continue;
            };
        }
        // 查询电气托盘的数据
        let refnos = query_filter_deep_children(refno, vec!["BRAN".to_string()]).await?;
        let refnos_str = serde_json::to_string(
            &refnos
                .into_iter()
                .map(|refno| refno.to_pe_key())
                .collect::<Vec<String>>(),
        )?;
        let sql = format!(
            r#"select
        id,
        string::slice(fn::find_ancestor_type($this.id,"SITE")[0].refno.NAME,1,1) as num, // 机组号
        string::slice(string::split(fn::find_ancestor_type($this.id,"SITE")[0].refno.NAME,'-')[0],2) as project_num, //子项号
        if string::contains(fn::find_ancestor_type($this.id,"SITE")[0].refno.NAME,'MCT') || string::contains(fn::default_name(fn::find_ancestor_type($this.id,"ZONE")[0]),'MSUP')  {{ '主托盘' }} else {{ '次托盘' }} as major,//专业
        fn::find_ancestor_type($this.id,"SITE")[0].refno.DESC as project_name, //子项名称
        fn::room_code($this.id)[0] as room_code,
        fn::default_name($this.id) as name,// 托盘段号
        refno.HPOS[2] as pos, // 托盘标高
        //<-pe_owner[where in.noun!='ATTA'] order by order_num,

        fn::dq_bran_type($this.id) as bran_type, // 托盘类型
        '碳钢Q235' as material, // 材质
        (select value (select value v from only udas.* where u.NAME =='/BranWidth' limit 1) from type::thing("ATT_UDA", meta::id(id)))[0][0] as width, // 托盘宽度
        (select value (select value v from only udas.* where u.NAME =='/BranHigh' limit 1) from type::thing("ATT_UDA", meta::id(id)))[0][0] as height, // 托盘高度
        if (select value (select value v from only udas.* where u.NAME =='/BranWidth' limit 1) from type::thing("ATT_UDA", meta::id(id)))[0][0] == NONE {{ 0 }} else {{ (select value (select value v from only udas.* where u.NAME=='/BranWidth' limit 1) from type::thing("ATT_UDA", meta::id(id)))[0][0] }} * if (select value (select value v from only udas.* where u.NAME=='/BranHigh' limit 1) from type::thing("ATT_UDA", meta::id(id)))[0][0] == NONE {{ 0 }} else {{ (select value (select value v from only udas.* where u.NAME=='/BranHigh' limit 1) from type::thing("ATT_UDA", meta::id(id)))[0][0] }} as size_num, // 规格型号
        'Y' as b_painting, // 是否刷漆
        string::split(fn::default_name($this.id),'-')[1] as painting_color,
        if (select value in from only (select * from <-pe_owner[where in.noun != 'ATTA'] order by order_num) limit 1).refno.DESP[2] == 1 {{ '是' }} else {{ '否' }} as b_cover, //有无盖板
        if refno.DESC == NONE {{ '无' }} else {{ '是' }} as b_partition, // 有无隔板
        refno.DESC as partition_num, // 隔板编号
        (select value in from only (select * from <-pe_owner[where in.noun != 'ATTA'] order by order_num) limit 1).refno.SPRE.refno.NAME as spre,
        (select value in from only (select * from <-pe_owner[where in.noun != 'ATTA'] order by order_num) limit 1).refno.SPRE.refno.CATR.refno.NAME as catr,
        fn::dq_horizontal_or_vertical($this.id) as horizontal_or_vertical
        from {}"#,
            refnos_str
        );
        let mut response = db.query(sql).await?;
        let mut result: Vec<MaterialDqMaterialList> = response.take(0)?;
        data.append(&mut result);
        // 查询电气支吊架的数据
        let zones = get_children_pes(pe.refno).await?;
        for zone in zones {
            if zone.name.contains("MTGD") {
                continue;
            };
            let refnos = query_filter_deep_children(refno, vec!["STRU".to_string()]).await?;
            let refnos_str = serde_json::to_string(
                &refnos
                    .into_iter()
                    .map(|refno| refno.to_pe_key())
                    .collect::<Vec<String>>(),
            )?;
            let sql = format!(
                r#"select id,
        string::slice(fn::find_ancestor_type($this.id,"SITE")[0].refno.NAME,1,1) as num, // 机组号
        string::slice(string::split(fn::find_ancestor_type($this.id,"SITE")[0].refno.NAME,'-')[0],2) as project_num, //子项号
        fn::find_ancestor_type($this.id,"SITE")[0].refno.DESC as project_name, //子项名称
        '支吊架' as major,//专业
        fn::room_code($this.id)[0] as room_code, // 房间号
        fn::default_name($this.id) as supp_name, // 托盘支吊架名称
        '碳钢Q355' as material,  //材质
        if (<-pe_owner.in<-pe_owner[where in.noun='SCTN'|| in.noun = 'GENSEC'].in.refno.SPRE.name)[0] == NONE {{ '' }}
        else {{ array::last(string::split((<-pe_owner.in<-pe_owner[where in.noun='SCTN'|| in.noun = 'GENSEC'].in.refno.SPRE.name)[0],'-')) }} as size_num, // 规格型号
        (<-pe_owner.in<-pe_owner[where in.noun='SCTN'|| in.noun = 'GENSEC'].in.refno.SPRE.name)[0] as spre,
        (<-pe_owner.in<-pe_owner[where in.noun='SCTN'|| in.noun = 'GENSEC'].in.refno.SPRE.refno.CATR.refno.NAME)[0] as catr,
        '2' as count // 数量
        from {}"#,
                refnos_str
            );
            let mut response = db.query(sql).await?;
            let mut result: Vec<MaterialDqMaterialListStru> = response.take(0)?;
            stru_data.append(&mut result);
        }
        // 电缆及接地
        let zones = get_children_pes(pe.refno).await?;
        for zone in zones {
            if !zone.name.contains("MTGD") {
                continue;
            };
            let refnos = query_filter_deep_children(refno, vec!["GENSEC".to_string()]).await?;
            let refnos_str = serde_json::to_string(
                &refnos
                    .into_iter()
                    .map(|refno| refno.to_pe_key())
                    .collect::<Vec<String>>(),
            )?;
            let sql = format!(
                r#"select id,
            string::slice(fn::find_ancestor_type($this.id,"SITE")[0].NAME,1,1) as num, // 机组号
            string::slice(string::split(fn::find_ancestor_type($this.id,"SITE")[0].NAME,'-')[0],2) as project_num, //子项号
            fn::find_ancestor_type($this.id,"SITE")[0].NAME.DESC as project_name, //子项名称
            '主托盘接地' as major,//专业
            fn::room_code($this.id)[0] as room_code, // 房间号
            fn::default_name($this.id) as name, // 托盘段号
            math::fixed(refno.POS[2],3) as pos,
            '裸铜缆' as material,  //材质
            refno.SPRE.refno.NAME as spre,
            refno.SPRE.refno.CATR.refno.NAME as catr,
            math::fixed(fn::vec3_distance(array::clump(<-pe_owner.in<-pe_owner[where in.noun='POINSP'].in.refno.POS,3)[0],array::clump(<-pe_owner.in<-pe_owner[where in.noun='POINSP'].in.refno.POS,3)[1]),2) as count
            from {}"#,
                refnos_str
            );
            let mut response = db.query(sql).await?;
            let mut result: Vec<MaterialDqMaterialList> = response.take(0)?;
            data.append(&mut result);
        }
    }
    Ok((data, stru_data))
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct DqMaterial {
    #[serde(rename = "元件等级名称")]
    pub catr_name: Option<String>,
    #[serde(rename = "标准号")]
    pub stander_num: Option<String>,
    #[serde(rename = "物项代码")]
    pub code: Option<String>,
    #[serde(rename = "名称")]
    pub name: Option<String>,
    #[serde(rename = "DESIGNATION")]
    pub desc: Option<String>,
    #[serde(rename = "单位")]
    pub unit: Option<String>,
    #[serde(rename = "数量")]
    pub count: Option<String>,
    #[serde(rename = "质保等级")]
    pub level: Option<String>,
}
