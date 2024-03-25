use glam::{bool, u32};
use std::fs::File;
use serde_derive::{Deserialize, Serialize};
use bevy_ecs::component::Component;
use id_tree::Tree;
use std::ops::{Deref, DerefMut};
use std::io::{Read, Write};
use crate::pdms_types::EleTreeNode;
use crate::RefU64;

impl PdmsTree {
    pub fn serialize_to_bin_file(&self, db_code: u32) -> bool {
        let mut file = File::create(format!("PdmsTree_{}.bin", db_code)).unwrap();
        let serialized = bincode::serialize(&self).unwrap();
        file.write_all(serialized.as_slice()).unwrap();
        true
    }

    pub fn serialize_to_bin_file_with_name(&self, name: &str, db_code: u32) -> bool {
        let mut file = File::create(format!("{name}_{db_code}.bin")).unwrap();
        let serialized = bincode::serialize(&self).unwrap();
        file.write_all(serialized.as_slice()).unwrap();
        true
    }

    pub fn deserialize_from_bin_file(db_code: u32) -> anyhow::Result<Self> {
        let mut file = File::open(format!("PdmsTree_{}.bin", db_code))?;
        let mut buf: Vec<u8> = Vec::new();
        file.read_to_end(&mut buf).ok();
        let r = bincode::deserialize(buf.as_slice())?;
        Ok(r)
    }
    pub fn deserialize_from_bin_file_with_name(name: &str, db_code: u32) -> anyhow::Result<Self> {
        let mut file = File::open(format!("{name}_{db_code}.bin"))?;
        let mut buf: Vec<u8> = Vec::new();
        file.read_to_end(&mut buf).ok();
        let r = bincode::deserialize(buf.as_slice())?;
        Ok(r)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, Component)]
pub struct PdmsTree(pub Tree<EleTreeNode>);

impl Deref for PdmsTree {
    type Target = Tree<EleTreeNode>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for PdmsTree {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl PdmsTree{
    ///获得世界refno
    #[inline]
    pub fn get_world_refno(&self) -> Option<RefU64> {
        self.root_node_id().map(|x| self.get(x).map(|t| t.data().refno).ok()).flatten()
    }

}
