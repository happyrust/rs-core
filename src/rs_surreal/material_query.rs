use std::collections::HashMap;
use std::ptr::eq;
use std::str::FromStr;
use config::{Config, File};
use parry3d::partitioning::QbvhDataGenerator;
use crate::{connect_surdb, get_children_pes, get_pe, query_filter_deep_children, RefU64, SUL_DB, SurlValue};
use serde::{Serialize, Deserialize};
use surrealdb::engine::remote::ws::{Client, Ws};
use surrealdb::key::thing::Thing;
use surrealdb::Surreal;
use crate::options::DbOption;
use crate::pe::SPdmsElement;
use crate::rs_surreal::table_const::GY_DZCL;
use surrealdb::engine::any::Any;
use crate::test::test_surreal::init_test_surreal;
use crate::pdms_types::{ser_refno_as_str, de_refno_from_key_str};

#[derive(Debug, Serialize, Deserialize)]
pub struct MaterialGyDataBend {
    #[serde(deserialize_with = "de_refno_from_key_str")]
    #[serde(serialize_with = "ser_refno_as_str")]
    pub id: RefU64,
    pub code: String,
    pub noun: String,
    pub count: f32,
}

impl MaterialGyDataBend {
    //// 将结构体转为HashMap
    pub fn into_hashmap(self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.entry("参考号".to_string()).or_insert(self.id.to_pdms_str());
        map.entry("编码".to_string()).or_insert(self.code);
        map.entry("部件".to_string()).or_insert(self.noun);
        map.entry("数量".to_string()).or_insert(self.count.to_string());
        map
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MaterialGyData {
    #[serde(deserialize_with = "de_refno_from_key_str")]
    #[serde(serialize_with = "ser_refno_as_str")]
    pub id: RefU64,
    pub code: String,
    pub noun: String,
}

impl MaterialGyData {
    //// 将结构体转为HashMap
    pub fn into_hashmap(self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.entry("参考号".to_string()).or_insert(self.id.to_pdms_str());
        map.entry("编码".to_string()).or_insert(self.code);
        map.entry("类型".to_string()).or_insert(self.noun);
        map
    }

    pub fn into_yk_hashmap(self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.entry("参考号".to_string()).or_insert(self.id.to_pdms_str());
        map.entry("编码".to_string()).or_insert(self.code);
        map.entry("品名".to_string()).or_insert(self.noun);
        map
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MaterialGyValvList {
    #[serde(deserialize_with = "de_refno_from_key_str")]
    #[serde(serialize_with = "ser_refno_as_str")]
    pub id: RefU64,
    pub valv_name: String,
    pub room_code: Option<String>,
    pub valv_belong: String,
    pub valv_length: Option<f32>,
    pub valv_weight: Option<f32>,
    pub valv_x: Option<f32>,
    pub valv_y: Option<f32>,
    pub valv_z: Option<f32>,
    pub valv_supp: String,
}

impl MaterialGyValvList {
    //// 将结构体转为HashMap
    pub fn into_hashmap(self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.entry("参考号".to_string()).or_insert(self.id.to_pdms_str());
        map.entry("阀门位号".to_string()).or_insert(self.valv_name.to_string());
        map.entry("所在房间号".to_string()).or_insert(self.room_code.unwrap_or("".to_string()));
        map.entry("阀门归属".to_string()).or_insert(self.valv_belong.to_string());
        // 没有的给个默认值
        let valv_length = self.valv_length.map_or("0".to_string(), |x| x.to_string());
        map.entry("阀门长度".to_string()).or_insert(valv_length.to_string());
        let valv_weight = self.valv_weight.map_or("0".to_string(), |x| x.to_string());
        map.entry("阀门重量".to_string()).or_insert(valv_weight.to_string());
        let valv_x = self.valv_x.map_or("0".to_string(), |x| x.to_string());
        map.entry("阀门重心X".to_string()).or_insert(valv_x.to_string());
        let valv_y = self.valv_y.map_or("0".to_string(), |x| x.to_string());
        map.entry("阀门重心Y".to_string()).or_insert(valv_y.to_string());
        let valv_z = self.valv_z.map_or("0".to_string(), |x| x.to_string());
        map.entry("阀门重心Z".to_string()).or_insert(valv_z.to_string());

        map.entry("是否阀门支架".to_string()).or_insert(self.valv_supp.to_string());
        map
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MaterialGyEquiList {
    #[serde(deserialize_with = "de_refno_from_key_str")]
    #[serde(serialize_with = "ser_refno_as_str")]
    pub id: RefU64,
    pub name: String,
    pub room_code: Option<String>,
    pub nozz_name: Vec<String>,
    pub nozz_pos: Vec<Vec<f32>>,
    pub nozz_cref: Vec<String>,
}

impl MaterialGyEquiList {
    //// 将结构体转为HashMap
    pub fn into_hashmap(self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.entry("参考号".to_string()).or_insert(self.id.to_pdms_str());
        map.entry("设备位号".to_string()).or_insert(self.name.to_string());
        map.entry("所在房间号".to_string()).or_insert(self.room_code.unwrap_or("".to_string()));
        map.entry("管口号".to_string()).or_insert(serde_json::to_string(&self.nozz_name).unwrap_or("[]".to_string()));
        map.entry("管口坐标".to_string()).or_insert(serde_json::to_string(&self.nozz_pos).unwrap_or("[]".to_string()));
        map.entry("相连管道编号".to_string()).or_insert(serde_json::to_string(&self.nozz_cref).unwrap_or("[]".to_string()));

        map
    }
}

/// 通风 风管管段
#[derive(Debug, Serialize, Deserialize)]
pub struct MaterialTfHavcList {
    #[serde(deserialize_with = "de_refno_from_key_str")]
    #[serde(serialize_with = "ser_refno_as_str")]
    pub id: RefU64,
    #[serde(rename = "Bolt_Qty")]
    #[serde(default)]
    pub bolt_qty: String,
    #[serde(rename = "Description")]
    #[serde(default)]
    pub desc: String,
    #[serde(rename = "Duct_Area")]
    #[serde(default)]
    pub duct_area: String,
    #[serde(rename = "Duct_Wei")]
    #[serde(default)]
    pub duct_wei: String,
    #[serde(rename = "FL_Len")]
    #[serde(default)]
    pub fl_len: String,
    #[serde(rename = "FL_Type")]
    #[serde(default)]
    pub fl_type: String,
    #[serde(rename = "FL_Wei")]
    pub fl_wei: String,
    #[serde(rename = "Height")]
    #[serde(default)]
    pub height: String,
    #[serde(rename = "Length")]
    pub length: String,
    #[serde(rename = "Material")]
    #[serde(default)]
    pub material: String,
    #[serde(rename = "Nut_Qty")]
    #[serde(default)]
    pub nut_qty: String,
    #[serde(rename = "Other_Qty")]
    #[serde(default)]
    pub other_qty: String,
    #[serde(rename = "Other_Type")]
    #[serde(default)]
    pub other_type: String,
    #[serde(rename = "Pressure")]
    #[serde(default)]
    pub pressure: String,
    #[serde(rename = "Room_No")]
    #[serde(default)]
    pub room_no: String,
    #[serde(rename = "Seg_Code")]
    #[serde(default)]
    pub seg_code: String,
    #[serde(rename = "Stif_Len")]
    #[serde(default)]
    pub stif_len: String,
    #[serde(rename = "Stif_Sctn")]
    #[serde(default)]
    pub stif_sctn: String,
    #[serde(rename = "Stif_Wei")]
    #[serde(default)]
    pub stif_wei: String,
    #[serde(rename = "Stud")]
    #[serde(default)]
    pub stud: String,
    #[serde(rename = "Sub_Code")]
    #[serde(default)]
    pub sub_code: String,
    #[serde(rename = "System")]
    #[serde(default)]
    pub system: String,
    #[serde(rename = "Wall_Thk")]
    #[serde(default)]
    pub wall_thk: String,
    #[serde(rename = "Washer_Len")]
    #[serde(default)]
    pub washer_len: String,
    #[serde(rename = "Washer_Type")]
    #[serde(default)]
    pub washer_type: String,
    #[serde(rename = "Washer_Qty")]
    #[serde(default)]
    pub washer_qty: String,
    #[serde(rename = "Width")]
    #[serde(default)]
    pub width: String,
}

impl MaterialTfHavcList {
    //// 将结构体转为HashMap
    pub fn into_hashmap(self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.entry("参考号".to_string()).or_insert(self.id.to_pdms_str());
        map.entry("描述".to_string()).or_insert(self.desc);
        map.entry("管段编号".to_string()).or_insert(self.seg_code);
        map.entry("子项号".to_string()).or_insert(self.sub_code);
        map.entry("材质".to_string()).or_insert(self.material);
        map.entry("压力等级".to_string()).or_insert(self.pressure);
        map.entry("风管长度".to_string()).or_insert(self.length);
        map.entry("风管宽度".to_string()).or_insert(self.width);
        map.entry("风管高度".to_string()).or_insert(self.height);
        map.entry("风管壁厚".to_string()).or_insert(self.wall_thk);
        map.entry("风管面积".to_string()).or_insert(self.duct_area);
        map.entry("风管重量".to_string()).or_insert(self.duct_wei);
        map.entry("加强筋型材".to_string()).or_insert(self.stif_sctn);
        map.entry("加强筋长度".to_string()).or_insert(self.stif_len);
        map.entry("加强筋重量".to_string()).or_insert(self.stif_wei);
        map.entry("法兰规格".to_string()).or_insert(self.fl_type);
        map.entry("法兰长度".to_string()).or_insert(self.fl_len);
        map.entry("法兰重量".to_string()).or_insert(self.fl_wei);
        map.entry("垫圈类型".to_string()).or_insert(self.washer_type);
        map.entry("垫圈长度".to_string()).or_insert(self.washer_len);
        map.entry("螺栓数量".to_string()).or_insert(self.bolt_qty);
        map.entry("其它材料类型".to_string()).or_insert(self.other_type);
        map.entry("其它材料数量".to_string()).or_insert(self.other_qty);
        map.entry("螺杆".to_string()).or_insert(self.stud);
        map.entry("螺母数量".to_string()).or_insert(self.nut_qty);
        map.entry("螺母数量_2".to_string()).or_insert(self.washer_qty);
        map.entry("所在房间号".to_string()).or_insert(self.room_no);
        map.entry("系统".to_string()).or_insert(self.system);
        map
    }
}

/// 电气 托盘及接地
#[derive(Debug, Serialize, Deserialize)]
pub struct MaterialDqMaterialList {
    #[serde(deserialize_with = "de_refno_from_key_str")]
    #[serde(serialize_with = "ser_refno_as_str")]
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
        map.entry("参考号".to_string()).or_insert(self.id.to_pdms_str());
        map.entry("机组号".to_string()).or_insert(self.num.unwrap_or("".to_string()));
        map.entry("子项号".to_string()).or_insert(self.project_num.unwrap_or("".to_string()));
        map.entry("元件等级名称".to_string()).or_insert(self.spre.clone().unwrap_or("".to_string()));
        map.entry("子项名称".to_string()).or_insert(self.project_name.unwrap_or("".to_string()));
        map.entry("专业".to_string()).or_insert(self.major);
        map.entry("房间号".to_string()).or_insert(self.room_code.unwrap_or("".to_string()));
        map.entry("托盘段号".to_string()).or_insert(self.name);
        map.entry("托盘标高".to_string()).or_insert(self.pos.unwrap_or(0.0).to_string());
        map.entry("托盘类型".to_string()).or_insert(self.bran_type.unwrap_or("".to_string()));
        map.entry("材质".to_string()).or_insert(self.material.unwrap_or("".to_string()));
        map.entry("托盘宽度mm".to_string()).or_insert(self.width.unwrap_or(0.0).to_string());
        map.entry("托盘高度mm".to_string()).or_insert(self.height.unwrap_or(0.0).to_string());
        map.entry("规格型号".to_string()).or_insert(self.size_num.unwrap_or(0.0).to_string());
        map.entry("是否刷漆".to_string()).or_insert(self.b_painting.unwrap_or("".to_string()));
        map.entry("刷漆颜色".to_string()).or_insert(self.painting_color.unwrap_or("".to_string()));
        map.entry("有无隔板".to_string()).or_insert(self.b_partition.unwrap_or("".to_string()));
        map.entry("隔板编号".to_string()).or_insert(self.partition_num.unwrap_or("".to_string()));
        map.entry("spre".to_string()).or_insert(self.spre.unwrap_or("".to_string()));
        map.entry("catr".to_string()).or_insert(self.catr.unwrap_or("".to_string()));
        map.entry("水平/竖向".to_string()).or_insert(self.horizontal_or_vertical.unwrap_or("".to_string()));
        map.entry("标准号".to_string()).or_insert(self.stander_num.unwrap_or("".to_string()));
        map.entry("物项编号".to_string()).or_insert(self.item_num.unwrap_or("".to_string()));
        map.entry("单位".to_string()).or_insert(self.unit.unwrap_or("".to_string()));
        map.entry("数量".to_string()).or_insert(self.count.unwrap_or(0.0).to_string());
        map
    }
}

/// 电气 托盘及接地
#[derive(Debug, Serialize, Deserialize)]
pub struct MaterialDqMaterialListStru {
    #[serde(deserialize_with = "de_refno_from_key_str")]
    #[serde(serialize_with = "ser_refno_as_str")]
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
        map.entry("参考号".to_string()).or_insert(self.id.to_pdms_str());
        map.entry("机组号".to_string()).or_insert(self.num.unwrap_or("".to_string()));
        map.entry("子项号".to_string()).or_insert(self.project_num.unwrap_or("".to_string()));
        map.entry("元件等级名称".to_string()).or_insert(self.spre.clone().unwrap_or("".to_string()));
        map.entry("子项名称".to_string()).or_insert(self.project_name.unwrap_or("".to_string()));
        map.entry("专业".to_string()).or_insert(self.major);
        map.entry("房间号".to_string()).or_insert(self.room_code.unwrap_or("".to_string()));
        map.entry("托盘标高".to_string()).or_insert(self.pos.unwrap_or(0.0).to_string());
        map.entry("材质".to_string()).or_insert(self.material.unwrap_or("".to_string()));
        map.entry("托盘支吊架名称".to_string()).or_insert(self.supp_name.unwrap_or("".to_string()));
        map.entry("托盘宽度mm".to_string()).or_insert(self.width.unwrap_or(0.0).to_string());
        map.entry("托盘高度mm".to_string()).or_insert(self.height.unwrap_or(0.0).to_string());
        map.entry("规格型号".to_string()).or_insert(self.size_num.unwrap_or("".to_string()));
        map.entry("是否刷漆".to_string()).or_insert(self.b_painting.unwrap_or("".to_string()));
        map.entry("刷漆颜色".to_string()).or_insert(self.painting_color.unwrap_or("".to_string()));
        map.entry("有无隔板".to_string()).or_insert(self.b_partition.unwrap_or("".to_string()));
        map.entry("隔板编号".to_string()).or_insert(self.partition_num.unwrap_or("".to_string()));
        map.entry("spre".to_string()).or_insert(self.spre.unwrap_or("".to_string()));
        map.entry("catr".to_string()).or_insert(self.catr.unwrap_or("".to_string()));
        map.entry("水平/竖向".to_string()).or_insert(self.horizontal_or_vertical.unwrap_or("".to_string()));
        map.entry("标准号".to_string()).or_insert(self.stander_num.unwrap_or("".to_string()));
        map.entry("物项编号".to_string()).or_insert(self.item_num.unwrap_or("".to_string()));
        map.entry("单位".to_string()).or_insert(self.unit.unwrap_or("".to_string()));
        map
    }
}


///查询工艺大宗材料数据
pub async fn get_gy_dzcl(db: Surreal<Any>, refnos: Vec<RefU64>) -> anyhow::Result<(Vec<MaterialGyData>, Vec<MaterialGyDataBend>)> {
    let mut data = Vec::new();
    let mut tubi_data = Vec::new();
    for refno in refnos {
        let Some(pe) = get_pe(refno).await? else { continue; };
        // 如果是site，则需要过滤 site的 name
        if pe.noun == "SITE".to_string() {
            if !pe.name.contains("PIPE") { continue; };
        }
        // 查询bend的数据
        let refnos = query_filter_deep_children(refno, vec!["BEND".to_string()]).await?;
        let refnos_str = serde_json::to_string(&refnos.into_iter()
            .map(|refno| refno.to_pe_key()).collect::<Vec<String>>())?;
        let sql = format!(r#"select
    id as id,
    string::split(string::split(if refno.SPRE.name == NONE {{ "//:" }} else {{ refno.SPRE.name }},'/')[2],':')[0] as code, // 编码
    refno.TYPE as noun, // 部件
    math::fixed((refno.ANGL / 360) * 2 * 3.1415 * refno.SPRE.refno.CATR.refno.PARA[1],2) as count // 长度
    from {}"#, refnos_str);
        let mut response = db
            .query(sql)
            .await?;
        let mut result: Vec<MaterialGyDataBend> = response.take(0)?;
        tubi_data.append(&mut result);
        // 查询tubi数据
        let refnos = query_filter_deep_children(refno, vec!["BRAN".to_string()]).await?;
        let refnos_str = serde_json::to_string(&refnos.into_iter()
            .map(|refno| refno.to_pe_key()).collect::<Vec<String>>())?;
        let sql = format!(r#"
    select value (select leave as id,
    (select value ( if leave.refno.LSTU.refno.NAME != NONE {{ string::split(array::at(string::split(leave.refno.LSTU.name, '/'), 2), ':')[0] }} else if leave.refno.HSTU.refno.NAME != NONE {{
    string::split(array::at(string::split(leave.refno.HSTU.name, '/'), 2), ':')[0]
    }} else {{ '' }}  ) from $self)[0]  as code,
    'TUBI' as noun,
    world_trans.d.scale[2] as count from ->tubi_relate) from {}"#, refnos_str);
        let mut response = db
            .query(sql)
            .await?;
        let mut result: Vec<Vec<MaterialGyDataBend>> = response.take(0)?;
        if !result.is_empty() {
            result.iter_mut().for_each(|x| tubi_data.append(x));
        }
        // 查询 elbo,tee,flan,gask,olet,redu,cap,couplig
        let refnos = query_filter_deep_children(refno, vec!["ELBO".to_string(),
                                                            "TEE".to_string(), "FLAN".to_string(), "GASK".to_string(),
                                                            "OLET".to_string(), "REDU".to_string(), "CAP".to_string(),
                                                            "COUP".to_string()]).await?;
        let refnos_str = serde_json::to_string(&refnos.into_iter()
            .map(|refno| refno.to_pe_key()).collect::<Vec<String>>())?;
        let sql = format!(r#"select
    id as id,
    string::split(string::split(if refno.SPRE.name == NONE {{ "//:" }} else {{ refno.SPRE.name }},'/')[2],':')[0] as code, // 编码
    refno.TYPE as noun // 部件
    from {}"#, refnos_str);
        let mut response = db
            .query(sql)
            .await?;
        // let mut result: Vec<MaterialGyData> = response.take(0)?;
        let mut result: Vec<MaterialGyData> = response.take(0)?;
        data.append(&mut result);
        // tubi_data.append(&mut result);
    }
    Ok((data, tubi_data))
}

/// 查询工艺阀门清单数据
pub async fn get_gy_valv_list(db: Surreal<Any>, refnos: Vec<RefU64>) -> anyhow::Result<Vec<MaterialGyValvList>> {
    let mut data = Vec::new();
    for refno in refnos {
        let Some(pe) = get_pe(refno).await? else { continue; };
        // 如果是site，则需要过滤 site的 name
        if pe.noun == "SITE".to_string() {
            if !pe.name.contains("PIPE") { continue; };
        }
        // 查询阀门的数据
        let refnos = query_filter_deep_children(refno, vec!["VALV".to_string(), "INST".to_string()]).await?;
        let refnos_str = serde_json::to_string(&refnos.into_iter()
            .map(|refno| refno.to_pe_key()).collect::<Vec<String>>())?;
        let sql = format!(r#"select
        id,
        fn::default_name(id) as valv_name, // 阀门位号
        fn::room_code($this.id)[0] as room_code, // 房间号
        string::split(string::slice(array::at(->pe_owner.out.name,0),1),'-')[0] as valv_belong, // 阀门归属
        if refno.SPRE.refno.CATR.refno.PARA[1] == NONE {{ 0 }} else {{ refno.SPRE.refno.CATR.refno.PARA[1] }} * 2 as valv_length, // 阀门长度
        if refno.SPRE.refno.CATR.refno.NAME != NONE && string::slice(refno.SPRE.refno.CATR.refno.NAME,4,1) != "R" {{ refno.SPRE.refno.CATR.refno.PARA[10] }} else if refno.SPRE.refno.CATR.refno.NAME != NONE && string::slice(refno.SPRE.refno.CATR.refno.NAME,4,1) == "R" {{ refno.SPRE.refno.CATR.refno.PARA[14] }} else {{ 0 }} as valv_weight, // 阀门重量
        if refno.SPRE.refno.CATR.refno.NAME != NONE && string::slice(refno.SPRE.refno.CATR.refno.NAME,4,1) != "R" {{ refno.SPRE.refno.CATR.refno.PARA[7] }} else if refno.SPRE.refno.CATR.refno.NAME != NONE && string::slice(refno.SPRE.refno.CATR.refno.NAME,4,1) == "R" {{ refno.SPRE.refno.CATR.refno.PARA[11] }} else {{ 0 }} as valv_x, // 阀门重心X
        if refno.SPRE.refno.CATR.refno.NAME != NONE && string::slice(refno.SPRE.refno.CATR.refno.NAME,4,1) != "R" {{ refno.SPRE.refno.CATR.refno.PARA[8] }} else if refno.SPRE.refno.CATR.refno.NAME != NONE && string::slice(refno.SPRE.refno.CATR.refno.NAME,4,1) == "R" {{ refno.SPRE.refno.CATR.refno.PARA[12] }} else {{ 0 }} as valv_y, // 阀门重心Y
        if refno.SPRE.refno.CATR.refno.NAME != NONE && string::slice(refno.SPRE.refno.CATR.refno.NAME,4,1) != "R" {{ refno.SPRE.refno.CATR.refno.PARA[9] }} else if refno.SPRE.refno.CATR.refno.NAME != NONE && string::slice(refno.SPRE.refno.CATR.refno.NAME,4,1) == "R" {{ refno.SPRE.refno.CATR.refno.PARA[13] }} else {{ 0 }} as valv_z, // 阀门重心Z
        fn::valv_b_supp(id) as valv_supp // 阀门支架
        from {}"#, refnos_str);
        let mut response = db
            .query(sql)
            .await?;
        let mut result: Vec<MaterialGyValvList> = response.take(0)?;
        data.append(&mut result);
    }
    Ok(data)
}

/// 查询工艺设备清单数据
pub async fn get_gy_equi_list(db: Surreal<Any>, refnos: Vec<RefU64>) -> anyhow::Result<Vec<MaterialGyEquiList>> {
    let mut data = Vec::new();
    for refno in refnos {
        let Some(pe) = get_pe(refno).await? else { continue; };
        // 如果是site，则需要过滤 site的 name
        if pe.noun == "SITE".to_string() {
            if !pe.name.contains("PIPE") { continue; };
        }
        // 查询阀门的数据
        let refnos = query_filter_deep_children(refno, vec!["EQUI".to_string()]).await?;
        let refnos_str = serde_json::to_string(&refnos.into_iter()
            .map(|refno| refno.to_pe_key()).collect::<Vec<String>>())?;
        let sql = format!(r#"select
        id,
        string::slice(refno.NAME,1) as name, // 设备位号
        fn::room_code($this.id)[0] as room_code, // 房间号
        fn::default_names(array::flatten([<-pe_owner[where in.noun='NOZZ']<-pe,  <-pe_owner.in<-pe_owner[where in.noun='NOZZ'].in])) as nozz_name, // 管口号
        array::clump(array::flatten([<-pe_owner[where in.noun='NOZZ']<-pe.refno.POS,  <-pe_owner.in<-pe_owner[where in.noun='NOZZ'].in.refno.POS]),3) as nozz_pos, // 管口坐标

        (select value if (name == NONE) {{ '' }} else {{ string::slice(name, 1) }} from array::flatten([<-pe_owner[where in.noun='NOZZ']<-pe,  <-pe_owner.in<-pe_owner[where in.noun='NOZZ'].in])) as nozz_cref // 相连管道编号
        from {}"#, refnos_str);
        let mut response = db
            .query(sql)
            .await?;
        let mut result: Vec<MaterialGyEquiList> = response.take(0)?;
        data.append(&mut result);
    }
    Ok(data)
}

/// 查询电气 托盘及接地 托盘
pub async fn get_dq_bran_list(db: Surreal<Any>, refnos: Vec<RefU64>) -> anyhow::Result<(Vec<MaterialDqMaterialList>, Vec<MaterialDqMaterialListStru>)> {
    let mut data = Vec::new();
    let mut stru_data = Vec::new();
    for refno in refnos {
        let Some(pe) = get_pe(refno).await? else { continue; };
        // 如果是site，则需要过滤 site的 name
        if pe.noun == "SITE".to_string() {
            if !pe.name.contains("ELEC") { continue; };
        }
        // 查询电气托盘的数据
        let refnos = query_filter_deep_children(refno, vec!["BRAN".to_string()]).await?;
        let refnos_str = serde_json::to_string(&refnos.into_iter()
            .map(|refno| refno.to_pe_key()).collect::<Vec<String>>())?;
        let sql = format!(r#"select
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
        from {}"#, refnos_str);
        let mut response = db
            .query(sql)
            .await?;
        let mut result: Vec<MaterialDqMaterialList> = response.take(0)?;
        data.append(&mut result);
        // 查询电气支吊架的数据
        let zones = get_children_pes(pe.refno).await?;
        for zone in zones {
            if zone.name.contains("MTGD") { continue; };
            let refnos = query_filter_deep_children(refno, vec!["STRU".to_string()]).await?;
            let refnos_str = serde_json::to_string(&refnos.into_iter()
                .map(|refno| refno.to_pe_key()).collect::<Vec<String>>())?;
            let sql = format!(r#"select id,
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
        from {}"#, refnos_str);
            let mut response = db
                .query(sql)
                .await?;
            let mut result: Vec<MaterialDqMaterialListStru> = response.take(0)?;
            stru_data.append(&mut result);
        }
        // 电缆及接地
        let zones = get_children_pes(pe.refno).await?;
        for zone in zones {
            if !zone.name.contains("MTGD") { continue; };
            let refnos = query_filter_deep_children(refno, vec!["GENSEC".to_string()]).await?;
            let refnos_str = serde_json::to_string(&refnos.into_iter()
                .map(|refno| refno.to_pe_key()).collect::<Vec<String>>())?;
            let sql = format!(r#"select id,
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
            from {}"#, refnos_str);
            let mut response = db
                .query(sql)
                .await?;
            let mut result: Vec<MaterialDqMaterialList> = response.take(0)?;
            data.append(&mut result);
        }
    }
    Ok((data, stru_data))
}

/// 查询仪控 大宗材料
pub async fn get_yk_dzcl_list(db: Surreal<Any>, refnos: Vec<RefU64>) -> anyhow::Result<Vec<MaterialGyData>> {
    let mut data = Vec::new();
    for refno in refnos {
        let Some(pe) = get_pe(refno).await? else { continue; };
        // 如果是site，则需要过滤 site的 name
        if pe.noun == "SITE".to_string() {
            if !pe.name.contains("INST") { continue; };
        }
        // 查询bend的数据
        let refnos = query_filter_deep_children(refno, vec!["VALV".to_string(), "TEE".to_string(), "COUP".to_string(), "INST".to_string(), "BEND".to_string()]).await?;
        let refnos_str = serde_json::to_string(&refnos.into_iter()
            .map(|refno| refno.to_pe_key()).collect::<Vec<String>>())?;
        let sql = format!(r#"select
        id,
        if refno.SPRE.name != NONE {{ string::split(string::split(refno.SPRE.name,'/')[2],':')[0] }} else {{ ' ' }} as code ,// 编码
        refno.TYPE as noun
        from {}"#, refnos_str);
        let mut response = db
            .query(sql)
            .await?;
        let mut result: Vec<MaterialGyData> = response.take(0)?;
        data.append(&mut result);
    }
    Ok(data)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MaterialYkInstData {
    #[serde(deserialize_with = "de_refno_from_key_str")]
    #[serde(serialize_with = "ser_refno_as_str")]
    pub id: RefU64,
    pub name: String,
    pub pipe_name: Option<String>,
    pub room_code: Option<String>,
}

impl MaterialYkInstData {
    pub fn into_hashmap(self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.entry("参考号".to_string()).or_insert(self.id.to_pdms_str());
        map.entry("传感器标识".to_string()).or_insert(self.name);
        map.entry("对应根阀编号".to_string()).or_insert(self.pipe_name.unwrap_or("".to_string()));
        map.entry("房间号".to_string()).or_insert(self.room_code.unwrap_or("".to_string()));
        map
    }
}

/// 仪控 仪表管道
pub async fn get_yk_inst_pipe(db: Surreal<Any>, refnos: Vec<RefU64>) -> anyhow::Result<Vec<MaterialYkInstData>> {
    let mut data = Vec::new();
    for refno in refnos {
        let Some(pe) = get_pe(refno).await? else { continue; };
        // 如果是site，则需要过滤 site的 name
        if pe.noun == "SITE".to_string() {
            if !pe.name.contains("INST") { continue; };
        }
        // 查询 inst 的数据
        let refnos = query_filter_deep_children(refno, vec!["INST".to_string()]).await?;
        let refnos_str = serde_json::to_string(&refnos.into_iter()
            .map(|refno| refno.to_pe_key()).collect::<Vec<String>>())?;
        let sql = format!(r#"select
        id,
        fn::default_name($this.id) as name,
        fn::find_pipe_bran($this.id)[0][0] as pipe_name,
        fn::room_code($this.id)[0] as room_code
        from {}"#, refnos_str);
        let mut response = db
            .query(sql)
            .await?;
        let mut result: Vec<MaterialYkInstData> = response.take(0)?;
        data.append(&mut result);
    }
    Ok(data)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MaterialYkEquiListData {
    #[serde(deserialize_with = "de_refno_from_key_str")]
    #[serde(serialize_with = "ser_refno_as_str")]
    pub id: RefU64,
    pub equi_name: String,
    pub room_code: Option<String>,
    pub pos: Option<f32>,
    pub floor_height: String,
}

impl MaterialYkEquiListData {
    pub fn into_hashmap(self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.entry("参考号".to_string()).or_insert(self.id.to_pdms_str());
        map.entry("仪控设备位号".to_string()).or_insert(self.equi_name);
        map.entry("所在房间号".to_string()).or_insert(self.room_code.unwrap_or("".to_string()));
        map.entry("设备绝对标高".to_string()).or_insert(self.pos.unwrap_or(0.0).to_string());
        map.entry("设备相对楼板标高".to_string()).or_insert(self.floor_height);

        map
    }
}

/// 仪控 设备清单
pub async fn get_yk_equi_list_material(db: Surreal<Any>, refnos: Vec<RefU64>) -> anyhow::Result<Vec<MaterialYkEquiListData>> {
    let mut data = Vec::new();
    for refno in refnos {
        let Some(pe) = get_pe(refno).await? else { continue; };
        // 如果是site，则需要过滤 site的 name
        if pe.noun == "SITE".to_string() {
            if !pe.name.contains("INST") { continue; };
        }
        // 查询 EQUI 的数据
        let refnos = query_filter_deep_children(refno, vec!["EQUI".to_string()]).await?;
        let refnos_str = serde_json::to_string(&refnos.into_iter()
            .map(|refno| refno.to_pe_key()).collect::<Vec<String>>())?;
        let sql = format!(r#"select
        id,
        fn::default_name($this.id) as equi_name,
        fn::room_code($this.id)[0] as room_code,
        fn::get_world_pos($this.id)[0][2] as pos, // 坐标 z
        '' as floor_height
        from {}"#, refnos_str);
        let mut response = db
            .query(sql)
            .await?;
        let mut result: Vec<MaterialYkEquiListData> = response.take(0)?;
        data.append(&mut result);
    }
    Ok(data)
}

/// 给排水 大宗材料
#[derive(Debug, Serialize, Deserialize)]
pub struct MaterialGpsDzclData {
    #[serde(deserialize_with = "de_refno_from_key_str")]
    #[serde(serialize_with = "ser_refno_as_str")]
    pub id: RefU64,
    pub code: String,
    pub noun: String,
    pub radius: Option<String>,
    pub length: Option<f32>,
    pub thick: Option<f32>,
    pub count: Option<f32>,
}

impl MaterialGpsDzclData {
    pub fn into_hashmap(self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.entry("参考号".to_string()).or_insert(self.id.to_pdms_str());
        map.entry("物项编码".to_string()).or_insert(self.code);
        map.entry("品名".to_string()).or_insert(self.noun);
        map.entry("外径/Φ".to_string()).or_insert(self.radius.unwrap_or("0.0".to_string()));
        map.entry("长度".to_string()).or_insert(self.count.unwrap_or(0.0).to_string());
        map.entry("厚度".to_string()).or_insert(self.thick.unwrap_or(0.0).to_string());
        map.entry("数量".to_string()).or_insert(self.count.unwrap_or(0.0).to_string());

        map
    }
}

/// 给排水 大宗材料
pub async fn get_gps_dzcl_material(db: Surreal<Any>, refnos: Vec<RefU64>) -> anyhow::Result<Vec<MaterialGpsDzclData>> {
    let mut data = Vec::new();
    for refno in refnos {
        let Some(pe) = get_pe(refno).await? else { continue; };
        // 如果是site，则需要过滤 site的 name
        if pe.noun == "SITE".to_string() {
            if !pe.name.contains("PIPE") { continue; };
        }
        // 查询 BEND 的数据
        let refnos = query_filter_deep_children(refno, vec!["BEND".to_string()]).await?;
        let refnos_str = serde_json::to_string(&refnos.into_iter()
            .map(|refno| refno.to_pe_key()).collect::<Vec<String>>())?;
        let sql = format!(r#"select
        id,
        string::split(string::split(if refno.SPRE.name == NONE {{ "//:" }} else {{ refno.SPRE.name }},'/')[2],':')[0] as code, // 编码
        refno.TYPE as noun ,// 部件
        string::replace(<string>math::fixed(if refno.SPRE.refno.CATR.refno.PARA[3] == NONE {{ 0 }} else {{ refno.SPRE.refno.CATR.refno.PARA[3] }},3),'f','') as radius, // 外径
        math::fixed(if refno.SPRE.refno.CATR.refno.PARA == NONE && refno.ANGL == NONE {{ 0 }}
        else {{ (refno.ANGL / 360) * 2 * 3.1415 * refno.SPRE.refno.CATR.refno.PARA[1] }},2) as count // 数量
        from {}"#, refnos_str);
        let mut response = db
            .query(sql)
            .await?;
        let mut result: Vec<MaterialGpsDzclData> = response.take(0)?;
        data.append(&mut result);
        // 查询tubi的数据
        let refnos = query_filter_deep_children(refno, vec!["BRAN".to_string()]).await?;
        let refnos_str = serde_json::to_string(&refnos.into_iter()
            .map(|refno| refno.to_pe_key()).collect::<Vec<String>>())?;
        let sql = format!(r#"select value (select leave as id,
        (select value ( if leave.refno.LSTU.refno.NAME != NONE {{ string::split(array::at(string::split(leave.refno.LSTU.name, '/'), 2), ':')[0] }} else if leave.refno.HSTU.refno.NAME != NONE {{
          string::split(array::at(string::split(leave.refno.HSTU.name, '/'), 2), ':')[0]
        }} else {{ '' }}  ) from $self)[0]  as code,
        'TUBI' as noun,
        string::replace(<string>math::fixed(if leave.refno.LSTU.refno.NAME != NONE {{ leave.refno.LSTU.refno.CATR.refno.PARA[1] }} else if leave.refno.HSTU.refno.NAME != NONE {{ leave.refno.HSTU.refno.CATR.refno.PARA[1] }} else {{ 0 }},3 ),'f','') as radius, // 外径
        world_trans.d.scale[2] as count from ->tubi_relate) from {};"#, refnos_str);
        let mut response = db
            .query(sql)
            .await?;
        let mut result: Vec<Vec<MaterialGpsDzclData>> = response.take(0)?;
        for mut d in result {
            data.append(&mut d);
        }
        // 查询elbo的数据
        let refnos = query_filter_deep_children(refno, vec!["ELBO".to_string()]).await?;
        let refnos_str = serde_json::to_string(&refnos.into_iter()
            .map(|refno| refno.to_pe_key()).collect::<Vec<String>>())?;
        let sql = format!(r#"select id,
        string::split(string::split(if refno.SPRE.name == NONE {{ "//:" }} else {{ refno.SPRE.name }},'/')[2],':')[0] as code, // 编码
        refno.TYPE as noun ,// 部件
        string::replace(<string>math::fixed(if refno.SPRE.refno.CATR.refno.PARA[3] == NONE {{ 0 }} else {{ refno.SPRE.refno.CATR.refno.PARA[3] }},3),'f','') as radius // 外径
        from {};"#, refnos_str);
        let mut response = db
            .query(sql)
            .await?;
        let mut result: Vec<MaterialGpsDzclData> = response.take(0)?;
        data.append(&mut result);
        // 查询flan的数据
        let refnos = query_filter_deep_children(refno, vec!["FLAN".to_string()]).await?;
        let refnos_str = serde_json::to_string(&refnos.into_iter()
            .map(|refno| refno.to_pe_key()).collect::<Vec<String>>())?;
        let sql = format!(r#"select id,
        string::split(string::split(if refno.SPRE.name == NONE {{ "//:" }} else {{ refno.SPRE.name }},'/')[2],':')[0] as code, // 编码
        refno.TYPE as noun ,// 部件
        string::replace(<string>math::fixed(if refno.SPRE.refno.CATR.refno.PARA[6] == NONE {{ 0 }} else {{ refno.SPRE.refno.CATR.refno.PARA[6] }},3),'f','') as radius, // 外径
        math::fixed(if refno.SPRE.refno.CATR.refno.PARA[4] == NONE {{ 0 }} else {{ refno.SPRE.refno.CATR.refno.PARA[4] }},3) as thick // 厚度
        from {};"#, refnos_str);
        let mut response = db
            .query(sql)
            .await?;
        let mut result: Vec<MaterialGpsDzclData> = response.take(0)?;
        data.append(&mut result);
        // 查询redu的数据
        let refnos = query_filter_deep_children(refno, vec!["REDU".to_string()]).await?;
        let refnos_str = serde_json::to_string(&refnos.into_iter()
            .map(|refno| refno.to_pe_key()).collect::<Vec<String>>())?;
        let sql = format!(r#"select id,
        string::split(string::split(if refno.SPRE.name == NONE {{ "//:" }} else {{ refno.SPRE.name }},'/')[2],':')[0] as code, // 编码
        refno.TYPE as noun ,// 部件
        string::replace(<string>array::join([math::fixed(if refno.SPRE.refno.CATR.refno.PARA[5] == NONE {{ 0 }} else {{ refno.SPRE.refno.CATR.refno.PARA[5] }},3),math::fixed(if refno.SPRE.refno.CATR.refno.PARA[6] == NONE {{ 0 }} else {{ refno.SPRE.refno.CATR.refno.PARA[6] }},3)],';'),'f','') as radius, // 外径
        math::fixed(if refno.SPRE.refno.CATR.refno.PARA[3] == NONE {{ 0 }} else {{ refno.SPRE.refno.CATR.refno.PARA[3] }},3) as length // 长度
        from {};"#, refnos_str);
        let mut response = db
            .query(sql)
            .await?;
        let mut result: Vec<MaterialGpsDzclData> = response.take(0)?;
        data.append(&mut result);
        // 查询tee的数据
        let refnos = query_filter_deep_children(refno, vec!["TEE".to_string()]).await?;
        let refnos_str = serde_json::to_string(&refnos.into_iter()
            .map(|refno| refno.to_pe_key()).collect::<Vec<String>>())?;
        let sql = format!(r#"select id,
        string::split(string::split(if refno.SPRE.name == NONE {{ "//:" }} else {{ refno.SPRE.name }},'/')[2],':')[0] as code, // 编码
        refno.TYPE as noun, // 部件
        string::replace(<string>array::join([<string>math::fixed(if refno.SPRE.refno.CATR.refno.PARA[6] != NONE {{ refno.SPRE.refno.CATR.refno.PARA[6] }} else {{ 0 }},3),<string>math::fixed(if refno.SPRE.refno.CATR.refno.PARA[7] != NONE {{ refno.SPRE.refno.CATR.refno.PARA[7] }} else {{ 0 }},3)],';'),'f','') as radius // 外径
        from {};"#, refnos_str);
        let mut response = db
            .query(sql)
            .await?;
        let mut result: Vec<MaterialGpsDzclData> = response.take(0)?;
        data.append(&mut result);
    }
    Ok(data)
}

/// 设备专业 通信系统
#[derive(Debug, Serialize, Deserialize)]
pub struct MaterialTxTxsbData {
    #[serde(deserialize_with = "de_refno_from_key_str")]
    #[serde(serialize_with = "ser_refno_as_str")]
    pub id: RefU64,
    pub equi_name: String,
    pub ptre_desc: String,
    pub belong_factory: String,
    pub room_code: Option<String>,
    pub x: Option<f32>,
    pub y: Option<f32>,
    pub z: Option<f32>,
    pub ptre_name: String,
}

impl MaterialTxTxsbData {
    pub fn into_hashmap(self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.entry("参考号".to_string()).or_insert(self.id.to_pdms_str());
        map.entry("设备位号".to_string()).or_insert(self.equi_name);
        map.entry("设备名称".to_string()).or_insert(self.ptre_desc);
        map.entry("所属厂房的编号".to_string()).or_insert(self.belong_factory);
        map.entry("房间号".to_string()).or_insert(self.room_code.unwrap_or("".to_string()));
        map.entry("全局坐标X".to_string()).or_insert(self.x.unwrap_or(0.0).to_string());
        map.entry("全局坐标Y".to_string()).or_insert(self.y.unwrap_or(0.0).to_string());
        map.entry("全局坐标Z".to_string()).or_insert(self.z.unwrap_or(0.0).to_string());
        map.entry("设备型号".to_string()).or_insert(self.ptre_name);

        map
    }
}

/// 通信专业 通信设备
pub async fn get_tx_txsb_list_material(db: Surreal<Any>, refnos: Vec<RefU64>) -> anyhow::Result<Vec<MaterialTxTxsbData>> {
    let mut data = Vec::new();
    for refno in refnos {
        let Some(pe) = get_pe(refno).await? else { continue; };
        // 如果是site，则需要过滤 site的 name
        if pe.noun == "SITE".to_string() {
            if !pe.name.contains("ELEC") { continue; };
        }
        // 过滤zone
        let zones = get_children_pes(pe.refno).await?;
        for zone in zones {
            if !zone.name.contains("FAD") { continue; };
            // 查询 EQUI 的数据
            let refnos = query_filter_deep_children(refno, vec!["ELCONN".to_string()]).await?;
            let refnos_str = serde_json::to_string(&refnos.into_iter()
                .map(|refno| refno.to_pe_key()).collect::<Vec<String>>())?;
            let sql = format!(r#"select
            id,
            fn::default_name(owner) as equi_name,
            string::slice(if refno.CATR.refno.PRTREF.desc == NONE {{ '/' }} else {{ refno.CATR.refno.PRTREF.desc }},1) as ptre_desc, // 设备名称
            string::slice(string::split(array::at(refno.REFNO->pe_owner.out->pe_owner.out.name,0),'-')[0],1,3) as belong_factory, // 所属厂房编号
            fn::room_code($this.id)[0] as room_code,
            fn::get_world_pos($this.id)[0][0] as x, // 坐标 x
            fn::get_world_pos($this.id)[0][1] as y, // 坐标 y
            fn::get_world_pos($this.id)[0][2] as z, // 坐标 z
            string::slice(refno.CATR.refno.PRTREF.refno.NAME,1) as ptre_name
            from {}"#, refnos_str);
            let mut response = db
                .query(sql)
                .await?;
            let mut result: Vec<MaterialTxTxsbData> = response.take(0)?;
            data.append(&mut result);
        }
    }
    Ok(data)
}

/// 设备专业 大宗材料
#[derive(Debug, Serialize, Deserialize)]
pub struct MaterialSbListData {
    #[serde(deserialize_with = "de_refno_from_key_str")]
    #[serde(serialize_with = "ser_refno_as_str")]
    pub id: RefU64,
    pub name: String,
    pub pos: Option<f32>,
    pub length: Option<f32>,
    pub room_code: Option<String>,
    pub boxs: Vec<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MaterialSubeData {
    #[serde(deserialize_with = "de_refno_from_key_str")]
    #[serde(serialize_with = "ser_refno_as_str")]
    pub refno: RefU64,
    pub pos: Option<f32>,
    pub length: Option<f32>,
}

impl MaterialSbListData {
    pub fn into_hashmap(self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.entry("参考号".to_string()).or_insert(self.id.to_pdms_str());
        map.entry("设备位号".to_string()).or_insert(self.name);
        map.entry("所在房间".to_string()).or_insert(self.room_code.unwrap_or("".to_string()));
        map.entry("轨道长度".to_string()).or_insert(self.length.unwrap_or(0.0).to_string());
        map.entry("安装标高".to_string()).or_insert(self.pos.unwrap_or(0.0).to_string());

        map
    }
}

/// 设备专业 大宗材料
pub async fn get_sb_dzcl_list_material(db: Surreal<Any>, refnos: Vec<RefU64>) -> anyhow::Result<Vec<MaterialSbListData>> {
    let mut data = Vec::new();
    for refno in refnos {
        let Some(pe) = get_pe(refno).await? else { continue; };
        // 如果是site，则需要过滤 site的 name
        if pe.noun == "SITE".to_string() {
            if !pe.name.contains("EQUI") { continue; };
        }
        // 查询 EQUI 的数据
        let refnos = query_filter_deep_children(refno, vec!["EQUI".to_string()]).await?;
        let refnos_str = serde_json::to_string(&refnos.into_iter()
            .map(|refno| refno.to_pe_key()).collect::<Vec<String>>())?;
        let sql = format!(r#"select
            id,
            fn::default_name($this.id) as name,
            fn::room_code($this.id)[0] as room_code,
            fn::find_group_sube_children($this.id) as boxs
            from {}"#, refnos_str);
        let mut response = db
            .query(sql)
            .await?;
        let result: Vec<MaterialSbListData> = response.take(0)?;
        let mut equi_data = result.into_iter()
            .filter(|x| !x.name.contains("PR") || !x.name.contains("PD"))
            .map(|equi| (equi.id, equi))
            .collect::<HashMap<RefU64, MaterialSbListData>>();
        // 查询轨道长度
        let tray = equi_data.iter().map(|x| x.1.boxs.clone()).collect::<Vec<_>>();
        let equi_children = filter_equi_children(tray);
        let sql = format!(r#"select
            (id.REFNO->pe_owner.out->pe_owner.out.refno)[0][0] as refno,
            array::max(array::max([XLEN,YLEN,ZLEN])) as length,
            array::max(POS[2]) as pos
            from {}"#, serde_json::to_string(&equi_children).unwrap_or("[]".to_string()));
        let mut response = db
            .query(sql)
            .await?;
        let result: Vec<MaterialSubeData> = response.take(0)?;
        // 将轨道长度放到设备的数据中
        for r in result {
            let Some(value) = equi_data.get_mut(&r.refno) else { continue; };
            value.pos = r.pos;
            value.length = r.length;
        }
        data.append(&mut equi_data.into_iter().map(|x| x.1).collect::<Vec<_>>())
    }
    Ok(data)
}

/// 暖通 阀门清单
#[derive(Debug, Serialize, Deserialize)]
pub struct MaterialNtValvData {
    #[serde(deserialize_with = "de_refno_from_key_str")]
    #[serde(serialize_with = "ser_refno_as_str")]
    pub id: RefU64,
    pub name: String,
    pub room_code: Option<String>,
    pub bran_name: String,
    pub valv_size: Vec<f32>,
    pub material: String,
    pub valv_use: String,
}

impl MaterialNtValvData {
    pub fn into_hashmap(self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.entry("参考号".to_string()).or_insert(self.id.to_pdms_str());
        map.entry("阀门位号".to_string()).or_insert(self.name);
        map.entry("所在房间号".to_string()).or_insert(self.room_code.unwrap_or("".to_string()));
        map.entry("阀门归属".to_string()).or_insert(self.bran_name);
        map.entry("阀门尺寸".to_string()).or_insert(serde_json::to_string(&self.valv_size).unwrap_or("[]".to_string()));
        map.entry("阀门材质".to_string()).or_insert(self.material);
        map.entry("阀门功能".to_string()).or_insert(self.valv_use);
        map
    }
}

/// 暖通 阀门清单
pub async fn get_nt_valv_list_material(db: Surreal<Any>, refnos: Vec<RefU64>) -> anyhow::Result<Vec<MaterialNtValvData>> {
    let mut data = Vec::new();
    for refno in refnos {
        let Some(pe) = get_pe(refno).await? else { continue; };
        // 如果是site，则需要过滤 site的 name
        if pe.noun == "SITE".to_string() {
            if !pe.name.contains("HVAC") { continue; };
        }
        // 查询 DAMP 的数据
        let refnos = query_filter_deep_children(refno, vec!["DAMP".to_string()]).await?;
        let refnos_str = serde_json::to_string(&refnos.into_iter()
            .map(|refno| refno.to_pe_key()).collect::<Vec<String>>())?;
        let sql = format!(r#"select
            id,
            fn::default_name($this.id) as name,
            fn::room_code($this.id)[0] as room_code,
            (->pe_owner.out->pe_owner.in.refno.NAME)[0] as bran_name,
            [if refno.DESP[1] == NONE {{ 0 }} else {{ refno.DESP[1] }},if refno.DESP[2] == NONE {{ 0 }} else {{ refno.DESP[2] }},
            if refno.DESP[5] == NONE {{ 0 }} else {{ refno.DESP[5] }}] as valv_size,
            fn::get_valv_material($this.id) as material,
            if name == NONE {{ '' }} else {{ string::slice(name,-3) }} as valv_use
            from {}"#, refnos_str);
        let mut response = db
            .query(sql)
            .await?;
        let mut result: Vec<MaterialNtValvData> = response.take(0)?;
        data.append(&mut result);
    }
    Ok(data)
}

/// 提前运行定义好的方法
pub async fn define_surreal_functions(db: Surreal<Any>) -> anyhow::Result<()> {
    let response = db
        .query(include_str!("material_list/default_name.surql"))
        .await?;
    let response = db
        .query(include_str!("material_list/dq/fn_dq_bran_type.surql"))
        .await?;
    let response = db
        .query(include_str!("material_list/dq/fn_vec3_distance.surql"))
        .await?;
    let response = db
        .query(include_str!("material_list/yk/fn_find_gy_bran.surql"))
        .await?;
    let response = db
        .query(include_str!("material_list/gy/fn_b_valv_supp.surql"))
        .await?;
    let response = db
        .query(include_str!("material_list/dq/fn_dq_horizontal_or_vertical.surql"))
        .await?;
    let response = db
        .query(include_str!("material_list/fn_get_ancestor.surql"))
        .await?;
    let response = db
        .query(include_str!("material_list/sb/fn_find_group_sube_children.surql"))
        .await?;
    let response = db
        .query(include_str!("material_list/nt/fn_get_valv_material.surql"))
        .await?;
    let response = db
        .query(include_str!("material_list/fn_get_world_pos.surql"))
        .await?;
    let response = db
        .query(include_str!("schemas/fn_query_room_code.surql"))
        .await?;
    Ok(())
}

fn filter_equi_children(datas: Vec<Vec<Vec<String>>>) -> Vec<Vec<String>> {
    let mut result = Vec::new();
    for data in datas {
        let filtered_data: Vec<Vec<String>> = data
            .into_iter()
            .filter(|inner_vec| {
                inner_vec.iter().all(|s| s.starts_with("BOX:"))
            })
            .filter(|inner_vec| {
                let count = inner_vec.iter().count();
                count == 3 || count == 4
            })
            .collect();
        if !filtered_data.is_empty() {
            result.push(filtered_data[0].clone())
        }
    }
    result
}