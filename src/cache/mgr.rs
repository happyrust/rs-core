use std::io::{Read, Write};
use std::sync::Arc;
use dashmap::DashMap;
use lazy_static::lazy_static;
use serde::de::DeserializeOwned;
use serde::Serialize;
use crate::pdms_types::RefU64;
use crate::pdms_types::AttrMap;
use crate::cache::refno::CachedRefBasic;
use crate::pdms_types::PdmsElementVec;
use dashmap::mapref::one::Ref;
use crate::pdms_types::RefU64Vec;
use serde::Deserialize;
#[cfg(not(target_arch = "wasm32"))]
use redb::{
    Builder, Database, Durability, Error, MultimapTableDefinition, ReadableTable, TableDefinition,
};


pub const CACHE_SLED_NAME: &'static str = "cache.rdb";
#[cfg(not(target_arch = "wasm32"))]
const TABLE: TableDefinition<u64, &[u8]> = TableDefinition::new("my_data");


pub trait BytesTrait {
    fn to_bytes(&self) -> Vec<u8>;
    fn from_bytes(bytes: &[u8]) -> Self;
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Debug)]
pub struct CacheMgr<T: BytesTrait + Clone + Serialize + DeserializeOwned> {
    name: String,
    // #[serde(skip_serializing)]
    db: Option<Arc<Database>>,
    map: DashMap<RefU64, T>,
    use_redb: bool,
}

#[cfg(target_arch = "wasm32")]
#[derive(Clone)]
pub struct CacheMgr<
    T: Clone + Serialize + DeserializeOwned> {
    name: String,
    map: DashMap<RefU64, T>,
    use_redb: bool,
}

#[cfg(not(target_arch = "wasm32"))]
impl<T: BytesTrait + Clone + Serialize + DeserializeOwned> CacheMgr<T> {
    pub fn new(name: &str, save_to_redb: bool) -> Self {
        Self {
            name: name.to_string(),
            db: if save_to_redb {
                unsafe { Database::create(name).map(|x| Arc::new(x)).ok() }
            } else {
                None
            },
            map: Default::default(),
            use_redb: save_to_redb,
        }

    }

    pub fn save_to_file(&self, path: &str) -> anyhow::Result<bool>{
        let bytes = bincode::serialize(&self.map)?;
        let mut file = std::fs::File::create(path)?;
        file.write_all(&bytes);
        Ok(true)
    }

    pub fn load_map_from_file(&self, path: &str) -> anyhow::Result<bool>{
        let mut file = std::fs::File::open(path)?;
        let mut bytes = vec![];
        file.read_to_end(&mut bytes)?;
        let mut map = bincode::deserialize::<DashMap<RefU64, T>>(&bytes)?;
        for (k, v) in map.into_iter() {
            self.map.insert(k, v);
        }
        Ok(true)
    }

    pub fn use_redb(&self) -> bool {
        self.use_redb
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    // pub fn len(&self) -> usize {
    //     if self.use_redb {
    //         self.db.map(|d| d.)
    //     }else{
    //         self.map.len()
    //     }
    // }

    #[inline]
    pub fn get(&self, k: &RefU64) -> Option<Ref<RefU64, T>> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            if self.use_redb && !self.map.contains_key(k) && self.db.is_some() {
                if let Some(db) = &self.db {
                    let read_txn = db.begin_read().ok()?;
                    let table = read_txn.open_table(TABLE).ok()?;
                    if let Ok(Some(bytes)) = table.get(&**k) {
                        self.map.insert((*k).into(), T::from_bytes(bytes.value()));
                    };
                }
            }
        }
        self.map.get(k)
    }

    #[inline]
    pub fn insert(&self, k: RefU64, value: &T) -> anyhow::Result<()> {
        self.map.insert(k, value.clone());
        if self.use_redb {
            #[cfg(not(target_arch = "wasm32"))]
            {
                if let Some(db) = &self.db {
                    let write_txn = db.begin_write()?;
                    {
                        let mut table = write_txn.open_table(TABLE)?;
                        table.insert(&*k, &*value.to_bytes())?;
                    }
                    write_txn.commit()?;
                }
            }
        }
        Ok(())
    }

    #[inline]
    pub fn contains_key(&self, k: &RefU64) -> bool {
        self.map.contains_key(k)
    }
}


#[cfg(target_arch = "wasm32")]
impl<T: Clone + Serialize + DeserializeOwned> CacheMgr<T>
{
    pub fn new(name: &str, save_to_redb: bool) -> Self {
        Self {
            name: name.to_string(),
            map: Default::default(),
            use_redb: save_to_redb,
        }

    }

    pub fn use_redb(&self) -> bool {
        self.use_redb
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    #[inline]
    pub fn get(&self, k: &RefU64) -> Option<Ref<RefU64, T>> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            if self.use_redb && !self.map.contains_key(k) && self.db.is_some() {
                // if let Ok(Some(bytes)) = self.db.as_ref().unwrap().get(k.into()) {
                //     self.map.insert((*k).into(), bytes.into());
                // }
                if let Some(db) = &self.db {
                    let read_txn = db.begin_read().ok()?;
                    let table = read_txn.open_table(TABLE).ok()?;
                    if let Ok(Some(bytes)) = table.get(&**k) {
                        self.map.insert((*k).into(), T::from_bytes(bytes));
                    }
                }
            }
        }
        self.map.get(k)
    }

    #[inline]
    pub fn insert(&self, k: RefU64, value: &T) -> anyhow::Result<()> {
        self.map.insert(k, value.clone());
        if self.use_redb {
            #[cfg(not(target_arch = "wasm32"))]
            {
                if let Some(db) = &self.db {
                    let write_txn = db.begin_write()?;
                    {
                        //todo use on file
                        let mut table = write_txn.open_table(TABLE)?;
                        table.insert(&*k, &value.to_bytes())?;
                    }
                    write_txn.commit()?;
                }
            }
        }
        Ok(())
    }

    #[inline]
    pub fn contains_key(&self, k: &RefU64) -> bool {
        self.map.contains_key(k)
    }
}