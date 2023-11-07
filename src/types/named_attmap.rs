use crate::orm::{BoolVec, F32Vec, I32Vec, StringVec};
use crate::pdms_types::{DEFAULT_NAMED_NOUNS, LEVEL_VISBLE, TOTAL_NEG_NOUN_NAMES};
use crate::tool::db_tool::{db1_dehash, db1_hash};
use crate::types::attmap::AttrMap;
use crate::types::named_attvalue::NamedAttrValue;
use crate::{get_default_pdms_db_info, AttrVal, RefU64, SurlValue, RefI32Tuple};
use bevy_ecs::component::Component;
use bevy_reflect::DynamicStruct;
use derive_more::{Deref, DerefMut};
use indexmap::IndexMap;
use sea_orm::{ConnectionTrait, DatabaseConnection};
use sea_query::{Alias, MysqlQueryBuilder};
use serde_derive::{Deserialize, Serialize};
use std::collections::BTreeMap;
use surrealdb::sql::Strand;
use crate::prim_geo::cylinder::SCylinder;
use crate::prim_geo::*;
use crate::shape::pdms_shape::BrepShapeTrait;
use glam::{Affine3A, Mat4, Vec3, DVec3, Quat, Mat3};
use crate::consts::UNSET_STR;
use crate::tool::math_tool::*;

///带名称的属性map
#[derive(
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
    Serialize,
    Deserialize,
    Deref,
    DerefMut,
    Clone,
    Default,
    Debug,
    Component,
)]
pub struct NamedAttrMap {
    #[serde(flatten)]
    pub map: BTreeMap<String, NamedAttrValue>,
}

impl From<SurlValue> for NamedAttrMap {
    fn from(s: SurlValue) -> Self {
        let mut map = BTreeMap::default();
        //需要根据类型来判断转换成相应的类型
        if let surrealdb::sql::Value::Object(o) = s {
            if let Some(SurlValue::Strand(Strand(type_name))) = o.get("TYPE") {
                let db_info = get_default_pdms_db_info();
                if let Some(m) = db_info.named_attr_info_map.get(type_name) {
                    for (k, v) in o.0 {
                        let default_val = if k == "REFNO" || k == "OWNER" {
                            AttrVal::RefU64Type(Default::default())
                        } else if k == "TYPE" {
                            AttrVal::StringType(Default::default())
                        } else if let Some(val) = m.get(&k) {
                            val.default_val.clone()
                        } else {
                            continue;
                        };
                        let named_value = match default_val {
                            crate::AttrVal::IntegerType(_) => {
                                // NamedAttrValue::IntegerType(i32::try_from(v.clone()).unwrap())
                                NamedAttrValue::IntegerType(v.try_into().unwrap_or_default())
                            }
                            crate::AttrVal::StringType(_)
                            | crate::AttrVal::WordType(_)
                            | crate::AttrVal::ElementType(_) => {
                                NamedAttrValue::StringType(v.try_into().unwrap_or_default())
                            }
                            crate::AttrVal::DoubleType(_) => {
                                NamedAttrValue::F32Type(v.try_into().unwrap_or_default())
                            }
                            crate::AttrVal::DoubleArrayType(_) | crate::AttrVal::Vec3Type(_) => {
                                let v: Vec<surrealdb::sql::Value> =
                                    v.try_into().unwrap_or_default();
                                NamedAttrValue::F32VecType(
                                    v.into_iter()
                                        .map(|x| f32::try_from(x).unwrap_or_default())
                                        .collect(),
                                )
                            }
                            crate::AttrVal::StringArrayType(_) =>  {
                                let v: Vec<surrealdb::sql::Value> =
                                    v.try_into().unwrap_or_default();
                                NamedAttrValue::StringArrayType(
                                    v.into_iter()
                                        .map(|x| String::try_from(x).unwrap_or_default())
                                        .collect(),
                                )
                            }
                            crate::AttrVal::BoolArrayType(_) =>  {
                                let v: Vec<surrealdb::sql::Value> =
                                    v.try_into().unwrap_or_default();
                                NamedAttrValue::BoolArrayType(
                                    v.into_iter()
                                        .map(|x| bool::try_from(x).unwrap_or_default())
                                        .collect(),
                                )
                            }
                            crate::AttrVal::IntArrayType(_) =>  {
                                let v: Vec<surrealdb::sql::Value> =
                                    v.try_into().unwrap_or_default();
                                NamedAttrValue::IntArrayType(
                                    v.into_iter()
                                        .map(|x| i32::try_from(x).unwrap_or_default())
                                        .collect(),
                                )
                            }
                            crate::AttrVal::BoolType(_) => {
                                NamedAttrValue::BoolType(v.try_into().unwrap_or_default())
                            }
                            crate::AttrVal::RefU64Type(_) => {
                                let str: String = v.try_into().unwrap_or_default();
                                NamedAttrValue::RefU64Type(str.as_str().into())
                            }
                            _ => NamedAttrValue::InvalidType,
                        };
                        map.insert(k.clone(), named_value);
                    }
                }
            }
        }
        Self { map }
    }
}

impl From<&AttrMap> for NamedAttrMap {
    fn from(v: &AttrMap) -> Self {
        Self {
            map: v
                .map
                .iter()
                .map(|(h, v)| (db1_dehash(*h), NamedAttrValue::from(v)))
                .collect(),
        }
    }
}

impl Into<DynamicStruct> for NamedAttrMap {
    fn into(self) -> DynamicStruct {
        let mut ds = DynamicStruct::default();
        for (k, v) in self.map {
            match v.clone() {
                NamedAttrValue::InvalidType => {}
                NamedAttrValue::IntegerType(d) => ds.insert(k.as_str(), d),
                NamedAttrValue::StringType(d) => ds.insert(k.as_str(), d),
                NamedAttrValue::F32Type(d) => ds.insert(k.as_str(), d),
                NamedAttrValue::F32VecType(d) => ds.insert(k.as_str(), F32Vec(d)),
                NamedAttrValue::Vec3Type(d) => ds.insert(k.as_str(), F32Vec(d.to_array().into())),
                NamedAttrValue::StringArrayType(d) => ds.insert(k.as_str(), StringVec(d)),
                NamedAttrValue::BoolArrayType(d) => ds.insert(k.as_str(), BoolVec(d)),
                NamedAttrValue::IntArrayType(d) => ds.insert(k.as_str(), I32Vec(d)),
                NamedAttrValue::BoolType(d) => ds.insert(k.as_str(), d),
                NamedAttrValue::ElementType(d) => ds.insert(k.as_str(), d),
                NamedAttrValue::WordType(d) => ds.insert(k.as_str(), d),
                NamedAttrValue::RefU64Type(d) => ds.insert(k.as_str(), d),
            }
        }

        ds
    }
}

impl NamedAttrMap {
    ///初始化
    pub fn new(type_name: &str) -> Self {
        let mut v = Self::default();
        let db_info = get_default_pdms_db_info();
        let hash = db1_hash(type_name) as i32;
        if let Some(info) = db_info.noun_attr_info_map.get(&hash) {
            for kv in info.value() {
                if kv.offset == 0 {
                    v.insert(kv.name.clone(), (&kv.default_val).into());
                }
            }
        }
        v.insert(
            "TYPE".to_string(),
            NamedAttrValue::StringType(type_name.to_string()),
        );
        v
    }

    #[inline]
    pub fn is_neg(&self) -> bool {
        TOTAL_NEG_NOUN_NAMES.contains(&self.get_type_str())
    }

    pub fn split_to_default_groups(&self) -> (NamedAttrMap, NamedAttrMap) {
        let mut default_att = NamedAttrMap::default();
        let mut comp_att = NamedAttrMap::default();

        for (k, v) in self.map.iter() {
            if DEFAULT_NAMED_NOUNS.contains(&k.as_str()) {
                default_att.map.insert(k.clone(), v.clone());
            } else {
                comp_att.insert(k.clone(), v.clone());
            }
        }
        (default_att, comp_att)
    }

    pub fn get_level(&self) -> Option<[u32; 2]> {
        let v = self.get_i32_vec("LEVE")?;
        if v.len() >= 2 {
            return Some([v[0] as u32, v[1] as u32]);
        }
        None
    }

    ///判断构件是否可见
    pub fn is_visible_by_level(&self, level: Option<u32>) -> Option<bool> {
        let levels = self.get_level()?;
        let l = level.unwrap_or(LEVEL_VISBLE);
        Some(levels[0] <= l && l <= levels[1])
    }

    #[inline]
    pub fn get_refno(&self) -> Option<RefU64> {
        if let NamedAttrValue::RefU64Type(d) = self.get_val("REFNO")? {
            return Some(*d);
        }
        None
    }

    #[inline]
    pub fn get_owner_as_string(&self) -> String {
        self.get_as_string("OWNER").unwrap_or(UNSET_STR.into())
    }


    pub fn get_type(&self) -> String {
        if let Some(NamedAttrValue::StringType(v)) = self.map.get("TYPE") {
            v.to_string()
        } else {
            "unset".to_string()
        }
    }

    pub fn get_type_str(&self) -> &str {
        if let Some(NamedAttrValue::StringType(v)) = self.map.get("TYPE") {
            v.as_str()
        } else {
            "unset"
        }
    }

    pub fn get_type_hash(&self) -> u32 {
        db1_hash(self.get_type_str())
    }

    #[inline]
    pub fn get_string_or_default(&self, att_name: &str) -> String {
        self.get_string(att_name).unwrap_or_default()
    }

    #[inline]
    pub fn get_string(&self, att_name: &str) -> Option<String> {
        let att = self.map.get(att_name)?;
        match att {
            NamedAttrValue::StringType(s) => Some(s.clone()),
            _ => None,
        }
    }

    #[inline]
    pub fn get_owner(&self) -> RefU64 {
        self.get_refno_by_att_or_default("OWNER")
    }

    #[inline]
    pub fn get_refno_by_att_or_default(&self, att_name: &str) -> RefU64 {
        self.get_refno_by_att(att_name).unwrap_or_default()
    }

    #[inline]
    pub fn get_refno_by_att(&self, att_name: &str) -> Option<RefU64> {
        let att = self.map.get(att_name)?;
        match att {
            NamedAttrValue::RefU64Type(s) => Some(s.clone()),
            _ => None,
        }
    }

    ///生成版本数据json
    pub fn gen_versioned_json_map(&self) -> IndexMap<String, serde_json::Value> {
        let mut map = IndexMap::new();
        let type_name = self.get_type();
        let refno = self.get_refno_by_att("REFNO").unwrap_or_default();
        map.insert("id".into(), refno.to_string().into());
        map.insert("TYPE".into(), type_name.into());
        map.insert("REFNO".into(), refno.to_string().into());

        for (key, val) in self.map.clone().into_iter() {
            //refno 单独处理
            if key.starts_with(":") || key.as_str() == "REFNO" {
                continue;
            }
            let new_key = key.replace(":", "_");
            map.insert(new_key, val.into());
        }
        map
    }

    pub fn get_matrix(&self) -> Option<Affine3A> {
        let mut affine = Affine3A::IDENTITY;
        let pos = self.get_f32_vec("POS")?;
        affine.translation = glam::f32::Vec3A::new(pos[0] , pos[1] , pos[2] );
        let ang = self.get_f32_vec("ORI")?;
        affine.matrix3 = glam::f32::Mat3A::from_rotation_z(ang[2].to_radians() )
            * glam::f32::Mat3A::from_rotation_y(ang[1].to_radians() )
            * glam::f32::Mat3A::from_rotation_x(ang[0].to_radians() );
        Some(affine)
    }

    #[inline]
    pub fn get_mat4(&self) -> Option<Mat4> {
        Some(Mat4::from(self.get_matrix()?))
    }

    pub fn get_f32_vec(&self, key: &str) -> Option<Vec<f32>> {
        let val = self.get_val(key)?;
        return match val {
            NamedAttrValue::F32VecType(data) => Some(data.clone()),
            NamedAttrValue::Vec3Type(data) => Some(vec![data.x, data.y, data.z]),
            _ => None,
        };
    }

    pub fn get_vec3(&self, key: &str) -> Option<Vec3> {
        if let NamedAttrValue::Vec3Type(d) = self.get_val(key)? {
            return Some(Vec3::new(d[0] , d[1] , d[2] ));
        }
        None
    }

    // pub fn get_dvec3(&self, key: &str) -> Option<DVec3> {
    //     if let NamedAttrValue::Vec3Type(d) = self.get_val(key)? {
    //         return Some(DVec3::new(d[0], d[1], d[2]));
    //     }
    //     None
    // }

    pub fn get_i32_vec(&self, key: &str) -> Option<Vec<i32>> {
        if let NamedAttrValue::IntArrayType(d) = self.get_val(key)? {
            return Some(d.clone());
        }
        None
    }

    ///生成具有几何属性的element的shape
    pub fn create_brep_shape(&self, limit_size: Option<f32>) -> Option<Box<dyn BrepShapeTrait>> {
        let type_noun = self.get_type_str();
        let mut r: Option<Box<dyn BrepShapeTrait>> = match type_noun {
            "BOX" | "NBOX" => Some(Box::new(SBox::from(self))),
            "CYLI" | "SLCY" | "NCYL" => Some(Box::new(SCylinder::from(self))),
            "SPHE" => Some(Box::new(Sphere::from(self))),
            "CONE" | "NCON" | "SNOU" | "NSNO" => Some(Box::new(LSnout::from(self))),
            "DISH" | "NDIS" => Some(Box::new(Dish::from(self))),
            "CTOR" | "NCTO" => Some(Box::new(CTorus::from(self))),
            "RTOR" | "NRTO" => Some(Box::new(RTorus::from(self))),
            "PYRA" | "NPYR" => Some(Box::new(Pyramid::from(self))),
            _ => None,
        };
        if r.is_some() && limit_size.is_some() {
            r.as_mut().unwrap().apply_limit_by_size(limit_size.unwrap());
        }
        r
    }

    #[inline]
    pub fn get_val(&self, key: &str) -> Option<&NamedAttrValue> {
        self.map.get(key).into()
    }

    // #[inline]
    // pub fn get_f64(&self, key: &str) -> Option<f64> {
    //     self.get_val(key)?.double_value()
    // }

    #[inline]
    pub fn get_f32(&self, key: &str) -> Option<f32> {
        self.get_val(key)?.f32_value()
    }

    #[inline]
    pub fn get_f32_or_default(&self, key: &str) -> f32 {
        self.get_f32(key).unwrap_or_default()
    }

    #[inline]
    pub fn get_position(&self) -> Option<Vec3> {
        if let Some(pos) = self.get_f32_vec("POS") {
            return Some(Vec3::new(pos[0] as f32, pos[1] as f32, pos[2] as f32));
        } else {
            //如果没有POS，就以POSS来尝试
            self.get_poss()
        }
    }

    #[inline]
    pub fn get_posse_dist(&self) -> Option<f32> {
        Some(self.get_pose()?.distance(self.get_poss()?))
    }

    #[inline]
    pub fn get_poss(&self) -> Option<Vec3> {
        let pos = self.get_f32_vec("POSS")?;
        if pos.len() == 3 {
            return Some(Vec3::new(pos[0] as f32, pos[1] as f32, pos[2] as f32));
        }
        None
    }

    #[inline]
    pub fn get_pose(&self) -> Option<Vec3> {
        let pos = self.get_f32_vec("POSE")?;
        if pos.len() == 3 {
            return Some(Vec3::new(pos[0] as f32, pos[1] as f32, pos[2] as f32));
        }
        None
    }

    #[inline]
    pub fn get_str(&self, key: &str) -> Option<&str> {
        let v = self.get_val(key)?;
        match v {
            NamedAttrValue::StringType(s) | NamedAttrValue::WordType(s) | NamedAttrValue::ElementType(s) => Some(s.as_str()),
            _ => None,
        }
    }

    #[inline]
    pub fn get_str_or_default(&self, key: &str) -> &str {
        self.get_str(key).unwrap_or("unset")
    }

    #[inline]
    pub fn get_as_strings(&self, keys: &[&str]) -> Vec<String> {
        let mut result = vec![];
        for key in keys {
            result.push(self.get_as_string(*key).unwrap_or(UNSET_STR.to_string()));
        }
        result
    }

    #[inline]
    pub fn get_as_string(&self, key: &str) -> Option<String> {
        use NamedAttrValue::*;
        let v = self.get_val(key)?;
        let s = match v {
            StringType(s) | WordType(s) | ElementType(s) => s.to_string(),
            IntegerType(d) => d.to_string().into(),
            F32Type(d) => d.to_string().into(),
            BoolType(d) => d.to_string().into(),
            F32VecType(d) => d
                .iter()
                .map(|i| format!(" {:.3}", i))
                .collect::<String>()
                .into(),
            StringArrayType(d) => d
                .iter()
                .map(|i| format!(" {}", i))
                .collect::<String>()
                .into(),
            IntArrayType(d) => d
                .iter()
                .map(|i| format!(" {}", i))
                .collect::<String>()
                .into(),
            BoolArrayType(d) => d
                .iter()
                .map(|i| format!(" {}", i))
                .collect::<String>()
                .into(),
            Vec3Type(d) => d.to_array()
                .iter()
                .map(|i| format!(" {:.3}", i))
                .collect::<String>()
                .into(),
            RefU64Type(d) => RefI32Tuple::from(d).into(),
            _ => UNSET_STR.into(),
        };
        Some(s)
    }

    #[inline]
    pub fn get_rotation(&self) -> Option<Quat> {
        let type_name = self.get_type_str();
        let mut quat = Quat::IDENTITY;
        if self.contains_key("ZDIR") {
            let axis_dir = self.get_vec3("ZDIR").unwrap_or_default().normalize();
            if axis_dir.is_normalized() {
                quat = Quat::from_mat3(&cal_mat3_by_zdir(axis_dir));
            }
        } else if self.contains_key("OPDI") {
            //PJOI 的方向
            let axis_dir = self.get_vec3("OPDI").unwrap_or_default().normalize();
            if axis_dir.is_normalized() {
                quat = Quat::from_mat3(&cal_mat3_by_zdir(axis_dir));
            }
        } else {
            match type_name {
                "CMPF" | "PFIT" => {
                    let sjus = self.get_str("SJUS").unwrap_or("unset");
                    //unset 和 UBOT 一样的效果
                    //DTOP, DCEN, DBOT
                    if sjus.starts_with("D") {
                        quat = Quat::from_mat3(&Mat3::from_cols(Vec3::X, Vec3::NEG_Y, Vec3::NEG_Z));
                    }
                }
                _ => {
                    let ang = self.get_f32_vec("ORI")?;
                    let mat = glam::f32::Mat3::from_rotation_z(ang[2].to_radians() as f32)
                        * glam::f32::Mat3::from_rotation_y(ang[1].to_radians() as f32)
                        * glam::f32::Mat3::from_rotation_x(ang[0].to_radians() as f32);

                    quat = Quat::from_mat3(&mat);
                }
            }
        }
        return Some(quat);
    }

}

impl NamedAttrMap {
    #[inline]
    pub fn get_columns(&self) -> Vec<Alias> {
        self.map.keys().map(|x| Alias::new(x.clone())).collect()
    }

    #[inline]
    pub fn get_values(&self) -> Vec<sea_query::Value> {
        self.map.values().map(|x| x.clone().into()).collect()
    }

    //填充其他显示类型数据为默认数据
    pub fn fill_explicit_default_values(&mut self) {
        let db_info = get_default_pdms_db_info();
        let noun_hash = self.get_type_hash() as i32;
        if let Some(m) = db_info.noun_attr_info_map.get(&noun_hash) {
            for info in m.value() {
                if info.offset == 0 && !self.map.contains_key(&info.name) {
                    self.map
                        .insert(info.name.clone(), (&info.default_val).into());
                }
            }

            //还有个TYPEX需要加
            if !self.map.contains_key("TYPEX") {
                self.map.insert(
                    "TYPEX".to_string(),
                    NamedAttrValue::StringType("".to_string()),
                );
            }
        }
    }

    // pub fn fill_explicit_empty_values(&mut self) {
    //     let db_info = get_default_pdms_db_info();
    //     let noun_hash = self.get_type_hash() as i32;
    //     if let Some(m) = db_info.noun_attr_info_map.get(&noun_hash) {
    //         for info in m.value() {
    //             if info.offset == 0 && !self.map.contains_key(&info.name) {
    //                 self.map.insert( info.name.clone(), (&info.default_val).into());
    //             }
    //         }
    //
    //         //还有个TYPEX需要加
    //         if !self.map.contains_key("TYPEX") {
    //             self.map.insert("TYPEX".to_string(), NamedAttrValue::StringType("".to_string()));
    //         }
    //     }
    // }

    pub fn contains_attr_hash(&self, hash: u32) -> bool {
        self.map.contains_key(&db1_dehash(hash))
    }

    ///执行保存
    pub async fn exec_insert(&self, db: &DatabaseConnection, replace: bool) -> anyhow::Result<()> {
        let sql = self.gen_insert_sql(replace)?;
        db.execute_unprepared(&sql).await?;
        Ok(())
    }

    ///生成保存的sql
    pub fn gen_insert_sql(&self, replace: bool) -> anyhow::Result<String> {
        let type_name = self.get_type();

        let mut query = sea_query::Query::insert()
            .into_table(Alias::new(type_name))
            .columns(self.get_columns())
            .to_owned();
        if replace {
            query.replace();
        }
        query
            .values(self.get_values().into_iter().map(|x| x.into()))?
            .to_owned();
        Ok(query.to_string(MysqlQueryBuilder))
    }

    ///生成插入的语句
    pub async fn exec_insert_many<I>(
        atts: I,
        db: &DatabaseConnection,
        replace: bool,
    ) -> anyhow::Result<()>
    where
        I: IntoIterator<Item = Self>,
    {
        let sqls = Self::gen_insert_many_sql(atts, replace)?;
        for sql in sqls {
            db.execute_unprepared(&sql).await?;
        }
        Ok(())
    }

    ///生成插入的语句
    pub fn gen_insert_many_sql<I>(atts: I, replace: bool) -> anyhow::Result<Vec<String>>
    where
        I: IntoIterator<Item = Self>,
    {
        ///按照类型重新划分组
        let mut grouped_map: BTreeMap<String, Vec<Self>> = BTreeMap::new();

        for mut a in atts {
            let type_name = a.get_type();
            a.fill_explicit_default_values();
            grouped_map.entry(type_name).or_insert(Vec::new()).push(a);
        }
        if grouped_map.is_empty() {
            return Err(anyhow::anyhow!("Empty atts can't gen insert sql."));
        }
        let mut final_sqls = Vec::new();
        //这里需要按type归类，按不同的type分类
        for (type_name, new_atts) in grouped_map {
            let colums = new_atts[0].get_columns();
            let mut query = sea_query::Query::insert()
                .into_table(Alias::new(type_name))
                .columns(colums)
                .to_owned();
            if replace {
                query.replace();
            }
            new_atts.into_iter().map(|x| x.get_values()).for_each(|x| {
                query.values(x.into_iter().map(|x| x.into())).unwrap();
            });
            let sql = query.to_string(MysqlQueryBuilder);
            final_sqls.push(sql);
        }
        Ok(final_sqls)
    }

    ///执行增量更新的提交
    pub async fn exec_commit_atts_change(
        db: &DatabaseConnection,
        message: Option<&str>,
    ) -> anyhow::Result<()> {
        // db.execute_unprepared(r#"call dolt_add('.')"#).await.unwrap();
        let msg = message.unwrap_or("提交增量更新数据");
        db.execute_unprepared(&format!(r#"call dolt_commit('-m', '{}')"#, msg))
            .await?;

        Ok(())
    }

    //后面还要根据参考号确定使用哪个类型、还有db numer
    //生成查询语句
    pub fn gen_query_sql(refnos: &Vec<RefU64>) -> anyhow::Result<Vec<String>> {
        //首先要查询到类型信息
        let types = sea_query::Query::select().to_owned();

        //按照类型不同, 分别去查询
        let query = sea_query::Query::select()
            // .cond_where()
            // .columns()
            // .column(Char::Character)
            // .column((Font::Table, Font::Name))
            // .from(Char::Table)
            // .left_join(Font::Table, Expr::col((Char::Table, Char::FontId)).equals((Font::Table, Font::Id)))
            // .and_where(Expr::col(Char::SizeW).is_in([3, 4]))
            // .and_where(Expr::col(Char::Character).like("A%"))
            .to_owned();
        Ok(vec![])
    }
}
