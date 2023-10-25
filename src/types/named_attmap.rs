use bevy_reflect::DynamicStruct;
use sea_query::{Alias, MysqlQueryBuilder, SimpleExpr};
use serde_derive::{Deserialize, Serialize};
use derive_more::{Deref, DerefMut};
use bevy_ecs::component::Component;
use std::collections::BTreeMap;
use sea_orm::{ConnectionTrait, DatabaseConnection};
use crate::orm::{BoolVec, F32Vec, I32Vec, StringVec};
use crate::pdms_types::DEFAULT_NAMED_NOUNS;
use crate::{get_default_pdms_db_info, RefU64};
use crate::attval::AttrVal;
use crate::tool::db_tool::{db1_dehash, db1_hash};
use crate::types::attmap::AttrMap;
use crate::types::named_attvalue::NamedAttrValue;

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
    pub fn new(type_name: &str) -> Self{
        let mut v = Self::default();
        let db_info = get_default_pdms_db_info();
        let hash = db1_hash(type_name) as i32;
        if let Some(info) = db_info.noun_attr_info_map.get(&hash) {
            for kv in info.value(){
                if kv.offset == 0 {
                    v.insert(kv.name.clone(), (&kv.default_val).into());
                }
            }
        }
        v.insert("TYPE".to_string(), NamedAttrValue::StringType(type_name.to_string()));
        v
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

    pub fn get_type(&self) -> String {
        if let Some(NamedAttrValue::StringType(v)) = self.map.get("TYPE") {
            v.to_string()
        } else {
            "unset".to_string()
        }
    }


    pub fn get_type_hash(&self) -> u32 {
        db1_hash(&self.get_type())
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
    pub fn gen_versioned_json_map(&self) -> serde_json::Map<String, serde_json::Value> {
        use serde_json::{Map, Value};

        let mut map = Map::new();

        let type_name = self.get_type();
        let refno = self.get_refno_by_att("REFNO").unwrap_or_default();
        map.insert(
            "@id".into(),
            format!("{}/{}", &type_name, refno.to_string()).into(),
        );
        map.insert("@type".into(), type_name.into());
        map.insert("REFNO".into(), refno.to_string().into());
        map.insert(
            "ELEMENT".into(),
            format!("PdmsElement/{}", refno.to_string()).into(),
        );

        for (key, val) in self.map.clone().into_iter() {
            //refno 单独处理
            if key.starts_with(":") || key.as_str() == "REFNO" {
                continue;
            }
            map.insert(key, val.into());
        }
        map
    }
}

impl NamedAttrMap {

    #[inline]
    pub fn get_columns(&self) -> Vec<Alias>{
        self.map.keys().map(|x|{
            Alias::new(x.clone())
        }).collect()
    }

    #[inline]
    pub fn get_values(&self) -> Vec<sea_query::Value>{
        self.map.values().map(|x|{
            x.clone().into()
        }).collect()
    }

    //填充其他显示类型数据为默认数据
    pub fn fill_explicit_default_values(&mut self) {
        let db_info = get_default_pdms_db_info();
        let noun_hash = self.get_type_hash() as i32;
        if let Some(m) = db_info.noun_attr_info_map.get(&noun_hash) {
            for info in m.value() {
                if info.offset == 0 && !self.map.contains_key(&info.name) {
                    self.map.insert( info.name.clone(), (&info.default_val).into());
                }
            }

            //还有个TYPEX需要加
            if !self.map.contains_key("TYPEX") {
                self.map.insert("TYPEX".to_string(), NamedAttrValue::StringType("".to_string()));
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

    pub fn contains_attr_hash(&self, hash: u32) -> bool{
        self.map.contains_key(&db1_dehash(hash))
    }

    ///执行保存
    pub async fn exec_insert(&self, db: &DatabaseConnection) -> anyhow::Result<()>{
        let sql = self.gen_insert_sql()?;
        db.execute_unprepared(&sql).await?;
        Ok(())
    }

    ///生成保存的sql
    pub fn gen_insert_sql(&self) -> anyhow::Result<String>{
        let type_name = self.get_type();

        let mut query = sea_query::Query::insert()
            .into_table(Alias::new(type_name))
            .columns(self.get_columns())
            .values(self.get_values().into_iter().map(|x| x.into()))?
            .to_owned();
        Ok(query.to_string(MysqlQueryBuilder))
    }

    ///生成插入的语句
    pub async fn exec_insert_many<I>(atts: I, db: &DatabaseConnection) -> anyhow::Result<()>
        where I: IntoIterator<Item = Self> {
        let sql = Self::gen_insert_many_sql(atts)?;
        db.execute_unprepared(&sql).await?;
        Ok(())
    }

    ///生成插入的语句
    pub fn gen_insert_many_sql<I>(atts: I) -> anyhow::Result<String>
        where I: IntoIterator<Item = Self>,
    {
        let mut new_atts = vec![];
        for mut a in atts{
            a.fill_explicit_default_values();
            new_atts.push(a);
        }
        if new_atts.is_empty() {
            return Err(anyhow::anyhow!("Empty atts can't gen insert sql."));
        }
        let type_name = new_atts[0].get_type();
        // dbg!(&type_name);
        // dbg!(db1_hash(&type_name));
        let colums = new_atts[0].get_columns();
        let mut query = sea_query::Query::insert()
            .into_table(Alias::new(type_name))
            .columns(colums).to_owned();
        let multi_values = new_atts.into_iter().map(|x| {
            // dbg!(x.get_values());
            x.get_values()
        }).for_each(|x|{
            query.values(x.into_iter().map(|x| x.into()));
        });
        Ok(query.to_string(MysqlQueryBuilder))
    }
}
