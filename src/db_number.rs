use std::collections::{BTreeMap, HashSet};
use std::fs::File;
use serde::{Serialize, Deserialize};
use std::io::Write;
use derive_more::*;
use itertools::Itertools;
use crate::pdms_types::RefU64;

#[derive(Clone, Debug, Default, Deref, DerefMut, Serialize, Deserialize)]
pub struct DbNumMgr{
    #[deref]
    #[deref_mut]
    pub ref0_dbnos_map: BTreeMap<u32, HashSet<u32>>,  //先看看有没有多个的情况
    // instance_loaded: HashSet<u32>, //已经加载了模型的dbno
    // ref1_dbno_map: BTreeMap<i32, i32>,
}

impl DbNumMgr{

    #[inline]
    pub fn insert(&mut self, refno: RefU64, dbno: i32){
        self.ref0_dbnos_map.entry(refno.get_0() ).or_insert(HashSet::new()).insert(dbno as u32);
    }

    pub fn serialize_to_specify_file(&self, file_path: &str) -> bool {
        let mut file = File::create(file_path).unwrap();
        let serialized = bincode::serialize(&self).unwrap();
        file.write_all(serialized.as_slice()).unwrap();
        true
    }

    // #[inline]
    // pub fn set_mesh_loaded(&mut self, refno: RefU64){
    //     if let Some(k) = self.ref0_dbnos_map.get(&refno.get_0()) {
    //         self.instance_loaded.union(&k);
    //     }
    // }
    //
    #[inline]
    pub fn get_dbno(&self, refno: RefU64) -> Option<u32> {
        if let Some(k) = self.ref0_dbnos_map.get(&refno.get_0()) {
            return k.iter().cloned().next();
        }
        None
    }

    #[inline]
    pub fn get_all_dbnos(&self) -> Vec<u32> {
        let mut v = HashSet::new();
        for kv in &self.ref0_dbnos_map {
            v.union(kv.1);
        }

        v.into_iter().sorted().collect_vec()
    }



    //
    // #[inline]
    // pub fn is_loaded(&self, dbno: u32) -> bool{
    //     self.instance_loaded.contains(&dbno)
    // }

}


