use dashmap::DashMap;
use lazy_static::lazy_static;
use serde::de::DeserializeOwned;
use serde::Serialize;
use sled::IVec;
use crate::pdms_types::RefU64;
use crate::pdms_types::AttrMap;
use crate::cache::refno::CachedRefBasic;
use crate::pdms_types::PdmsElementVec;
use dashmap::mapref::one::Ref;
use crate::pdms_types::RefU64Vec;

pub const CACHE_SLED_NAME: &'static str = "cache.db";

lazy_static! {
    pub static ref CACHE_DB: sled::Db  = {
       sled::open(CACHE_SLED_NAME).unwrap()
    };
    pub static ref PDMS_ATT_MAP_CACHE: CacheMgr< AttrMap>  = CacheMgr::new("ATTR_MAP_CACHE", false);
    pub static ref PDMS_ANCESTOR_CACHE: CacheMgr<RefU64Vec>  = CacheMgr::new("ANCESTOR_CACHE", false);
    pub static ref CACHED_REFNO_BASIC_MAP: CacheMgr< CachedRefBasic>  = CacheMgr::new("REFNO_BASIC_CACHE", false);
    pub static ref CACHED_MDB_SITE_MAP: CacheMgr< PdmsElementVec>  = CacheMgr::new("MDB_SITE_CACHE", true);
}


#[derive(Clone)]
pub struct CacheMgr<
    T: Into<IVec> + From<IVec> + Clone + Serialize + DeserializeOwned> {
    name: String,
    tree: sled::Tree,
    map: DashMap<RefU64, T>,
    use_sled: bool,
}

impl<T: Into<IVec> + From<IVec> + Clone + Serialize + DeserializeOwned> CacheMgr<T>
{
    pub fn new(name: &str, save_to_sled: bool) -> Self {
        let tree = CACHE_DB.open_tree(name).unwrap();
        Self {
            name: name.to_string(),
            tree,
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
        if self.use_sled && !self.map.contains_key(k) {
            if let Ok(Some(bytes)) = self.tree.get::<IVec>(k.into()) {
                self.map.insert((*k).into(), bytes.into());
            }
        }
        self.map.get(k)
    }

    #[inline]
    pub fn load_all(&self) {
        if self.use_sled {
            for k in self.tree.iter() {
                if let Ok((key, value)) = k {
                    self.map.insert(key.into(), value.into());
                }
            }
        }
    }

    #[inline]
    pub fn insert(&self, k: RefU64, value: T) -> anyhow::Result<()> {
        self.map.insert(k, value.clone());
        let bytes: IVec = k.into();
        if self.use_sled {
            self.tree.insert(bytes, value)?;
        }
        Ok(())
    }

    #[inline]
    pub fn contains_key(&self, k: &RefU64) -> bool {
        self.map.contains_key(k)
    }
}