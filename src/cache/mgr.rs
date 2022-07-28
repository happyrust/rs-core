use dashmap::DashMap;
use lazy_static::lazy_static;
use serde::de::DeserializeOwned;
use serde::Serialize;
use sled::{Db, IVec};
use crate::pdms_types::RefU64;
use crate::pdms_types::AttrMap;
use crate::cache::refno::CachedRefBasic;
use crate::pdms_types::PdmsElementVec;
use dashmap::mapref::one::Ref;
use crate::pdms_types::RefU64Vec;



pub const CACHE_SLED_NAME: &'static str = "cache.db";


#[derive(Clone)]
pub struct CacheMgr<
    T: Into<IVec> + From<IVec> + Clone + Serialize + DeserializeOwned> {
    name: String,
    db: Option<sled::Db>,
    map: DashMap<RefU64, T>,
    use_sled: bool,
}

impl<T: Into<IVec> + From<IVec> + Clone + Serialize + DeserializeOwned> CacheMgr<T>
{
    pub fn new(name: &str, save_to_sled: bool) -> Self {
        Self {
            name: name.to_string(),
            db: if save_to_sled{
                sled::open(name).ok()
                    // .open_tree(name).ok()
                // CACHE_DB.open_tree(name)
            }else{
                None
            },
            map: Default::default(),
            use_sled: save_to_sled,
        }
    }

    pub fn use_sled(&self) -> bool {
        self.use_sled
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    #[inline]
    pub fn get(&self, k: &RefU64) -> Option<Ref<RefU64, T>> {
        if self.use_sled && !self.map.contains_key(k) && self.db.is_some(){
            if let Ok(Some(bytes)) = self.db.as_ref().unwrap().get::<IVec>(k.into()) {
                self.map.insert((*k).into(), bytes.into());
            }
        }
        self.map.get(k)
    }

    #[inline]
    pub fn insert(&self, k: RefU64, value: T) -> anyhow::Result<()> {
        self.map.insert(k, value.clone());
        let bytes: IVec = k.into();
        if self.use_sled {
            self.db.as_ref().unwrap().insert(bytes, value)?;
        }
        Ok(())
    }

    #[inline]
    pub fn contains_key(&self, k: &RefU64) -> bool {
        self.map.contains_key(k)
    }
}