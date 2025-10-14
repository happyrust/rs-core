use crate::RefU64;
use serde::{Deserialize, Serialize};

/// `inst_relate` 实体的实时订阅结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveGeomData {
    pub id: RefU64,
    pub tubi_owner: Option<RefU64>,
}

pub const GEOM_LIVE_SQL: &str = r#"
    live select id,
        (
            if in.owner.noun in ['BRAN', 'HANG'] { in.owner }
            else { none }
        ) as tubi_owner
    from inst_relate
    where solid and aabb != none
"#;

pub const PE_LIVE_SQL: &str = r#"
    live select refno,
        noun,
        if op == 2 { noun } else { fn::default_name(id) } as name,
        owner,
        if (->pe_owner.id)[0] == None { 0 } else { record::id(->pe_owner.id[0])[1] } as order,
        op?:0 as op,
        children_updated,
        array::len(select value id from <-pe_owner) as children_count,
        status_code
    from pe
    where !type::is::array(record::id(id))
"#;
