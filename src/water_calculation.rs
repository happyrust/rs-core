use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use crate::pdms_types::RefU64;
use bevy_ecs::prelude::Resource;
use serde::{Deserialize, Serialize};
use bevy_ecs::prelude::Component;
use bevy_ecs::prelude::Event;


///水淹计算孔洞的结构体
#[derive(Resource, Clone, Debug, Default, Deserialize, Serialize)]
pub struct FloodingHole {
    pub refno: RefU64,
    pub name: String,
    //标记是孔洞还是门洞
    pub is_door: bool,
    //标记是否被选中
    pub is_selected: bool,
}


///水淹计算墙与孔洞关系对应的结构体
#[derive(Resource, Clone, Debug, Default, Deserialize, Serialize)]
pub struct FloodingHoleVec {
    pub data: Vec<HashMap<RefU64, Vec<FloodingHole>>>,
}

///导出水淹计算stp需要用到的数据
#[derive(Component, Clone, Debug, Default, Event, Deserialize, Serialize)]
pub struct ExportFloodingStpEvent {
    //文件名
    pub file_name: String,
    //保存时间
    pub save_time: String,
    //所有选中的模型列表
    pub model_list: Vec<(RefU64, String)>,
    //不需进行封堵的孔洞列表
    pub opening_hole_list: Vec<HashMap<RefU64, Vec<FloodingHole>>>,
    //需要进行封堵的孔洞列表
    pub plugging_hole_list: Vec<HashMap<RefU64, Vec<FloodingHole>>>,
}

impl ExportFloodingStpEvent {
    pub fn to_arango_struct(self) -> FloodingStpToArangodb {
        let mut hasher = DefaultHasher::new();
        self.file_name.hash(&mut hasher);
        let hash_name = hasher.finish();
        FloodingStpToArangodb {
            _key: hash_name.to_string(),
            save_time: self.save_time,
            file_name: self.file_name,
            model_list: self.model_list,
            opening_hole_list: self.opening_hole_list,
            plugging_hole_list: self.plugging_hole_list,
        }
    }
}

///将水淹计算保存到图数据库
#[derive(Component, Clone, Debug, Default, Event, Deserialize, Serialize)]
pub struct FloodingStpToArangodb {
    pub _key: String,
    //文件名
    pub file_name: String,
    //保存时间
    pub save_time: String,
    //所有选中的模型列表
    pub model_list: Vec<(RefU64, String)>,
    //不需进行封堵的孔洞列表
    pub opening_hole_list: Vec<HashMap<RefU64, Vec<FloodingHole>>>,
    //需要进行封堵的孔洞列表
    pub plugging_hole_list: Vec<HashMap<RefU64, Vec<FloodingHole>>>,
}

///将数据库中的数据组织成资源，导出历史记录时使用
#[derive(Component, Resource, Clone, Debug, Default, Event, Deserialize, Serialize)]
pub struct FloodingStpToArangodbVec {
    pub data: Vec<FloodingStpToArangodb>,
}

// #[derive(Component, Clone, Debug, Default, Event, Deserialize, Serialize)]
// pub struct DownloadStpEvent {
//     pub file_name: String,
//     pub contents: Vec<u8>,
// }
