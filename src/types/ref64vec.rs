use std::vec::IntoIter;
use serde_with::serde_as;
use serde_derive::{Deserialize, Serialize};
use bevy_ecs::component::Component;
use derive_more::{Deref, DerefMut};
use crate::cache::mgr::BytesTrait;
use crate::{query_refno_sesno, RefU64};

#[serde_as]
#[derive(
    Serialize,
    Deserialize,
    Clone,
    Debug,
    Default,
    Component,
    Deref,
    DerefMut,
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
)]
pub struct RefU64Vec(pub Vec<RefU64>);


impl RefU64Vec {
    /// 生成 owner 的 relate 关系，只生成历史数据
    pub async fn gen_owner_relates_h(&self, owner: RefU64, sesno: u32, dbnum: i32) -> anyhow::Result<Vec<String>> {
        let mut pe_owner_h_relates = Vec::new();
        for (index, &child) in self.0.iter().enumerate() {
            let (child_sesno, is_latest_pe) = query_refno_sesno(child, sesno, dbnum).await?;
            //child id 需要去 pe_ses 里查询得到最近的那个版本
            //如果是历史数据，加上 old 的标签
            if child_sesno != 0 && !is_latest_pe {
                pe_owner_h_relates.push(
                    format!(r#"{{ id: pe_owner:['{0}_{sesno}', {index}], in: pe:['{1}',{child_sesno}],
                        out: pe:['{0}', {sesno}],  old: true }}"#,
                            owner, child)
                );
            } else {
                // dbg!((child, child_sesno, refno, sesno));
                pe_owner_h_relates.push(
                    format!(r#"{{ id: pe_owner:['{0}_{sesno}', {index}], in: pe:{1}, out: pe:['{0}', {sesno}], old: true }}"#,
                            owner, child)
                );
            }
        }
        Ok(pe_owner_h_relates)
    }
}

impl BytesTrait for RefU64Vec {}

impl From<Vec<RefU64>> for RefU64Vec {
    fn from(d: Vec<RefU64>) -> Self {
        RefU64Vec(d)
    }
}

impl IntoIterator for RefU64Vec {
    type Item = RefU64;
    type IntoIter = IntoIter<RefU64>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl RefU64Vec {
    #[inline]
    pub fn push(&mut self, v: RefU64) {
        if !self.0.contains(&v) {
            self.0.push(v);
        }
    }
}
