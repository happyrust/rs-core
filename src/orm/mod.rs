pub mod diff_data;
pub mod log_data;
pub mod pdms_element;
// pub mod entities;

pub mod sql;
pub mod traits;
pub mod types;
#[macro_use]
pub mod macros;

// pub mod generated;
// pub use generated::*;

#[cfg(feature = "reflect")]
use bevy_reflect::TypeRegistry;
use once_cell::sync::OnceCell;
use std::any::TypeId;
use std::collections::BTreeMap;

// pub use entities::*;
pub use macros::*;
pub use traits::*;
pub use types::*;

/// 获得类型的注册机
///
/// 注意: 此功能需要 "reflect" feature 开启
///
/// # 返回值
/// - 当 reflect feature 开启时，返回 Some(&TypeRegistry)
/// - 当 reflect feature 未开启时，返回 None
pub fn get_type_registry() -> Option<&'static TypeRegistry> {
    #[cfg(feature = "reflect")]
    {
        static INSTANCE: OnceCell<TypeRegistry> = OnceCell::new();
        Some(INSTANCE.get_or_init(|| {
            let mut type_registry: TypeRegistry = TypeRegistry::default();
            type_registry.register::<pdms_element::Model>();
            type_registry
        }))
    }

    #[cfg(not(feature = "reflect"))]
    {
        None
    }
}

//todo 根据属性描述信息，使用宏来生成类型信息

pub fn get_type_name_cache() -> &'static OrmTypeNameCache {
    static INSTANCE: OnceCell<OrmTypeNameCache> = OnceCell::new();
    INSTANCE.get_or_init(|| {
        let mut type_cache: OrmTypeNameCache = OrmTypeNameCache::default();
        type_cache.type_id_of::<pdms_element::Model>();
        //使用宏来添加，个数有点多
        type_cache
    })
}

#[derive(Default, Clone, Debug)]
pub struct OrmTypeNameCache {
    id_map: BTreeMap<&'static str, TypeId>,
}

impl OrmTypeNameCache {
    pub fn type_id_of<T: 'static>(&mut self) -> TypeId {
        let id = TypeId::of::<T>();
        let names = std::any::type_name::<T>()
            .split("::")
            .into_iter()
            .collect::<Vec<_>>();
        let name = names[names.len() - 2];
        self.id_map.insert(name, id);
        id
    }
    pub fn id_for_name(&self, name: &str) -> Option<TypeId> {
        self.id_map.get(&name).copied()
    }
}
