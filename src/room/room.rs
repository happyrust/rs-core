use once_cell::sync::Lazy;
use tokio::sync::RwLock;

use crate::{
    accel_tree::acceleration_tree::{AccelerationTree, RStarBoundingBox},
    SUL_DB, RefU64,
};

//或者改成第一次，需要去加载，后续就不用了
//启动的时候就要去加载到内存里
pub static GLOBAL_AABB_TREE: Lazy<RwLock<AccelerationTree>> =
    Lazy::new(|| RwLock::new(AccelerationTree::default()));

// 不要每次都加载，需要检查缓存，如果缓存有，就不用从数据库里刷新了
pub async fn load_aabb_tree() -> anyhow::Result<bool> {

    //如果有缓存文件，直接读取缓存文件
    //测试分页查询
    let mut rstar_objs = vec![];
    let mut offset = 0;

    let page_count = 1000;
    loop {
        //需要过滤
        let sql = format!(
            "select in as refno, aabb.d.* as aabb, in.noun as noun from inst_relate where aabb.d!=none and type=0 start {} limit {page_count}",
            offset
        );
        let mut response = SUL_DB.query(&sql).await?;
        let refno_aabbs: Vec<RStarBoundingBox> = response.take(0).unwrap();
        if refno_aabbs.is_empty() {
            break;
        }
        rstar_objs.extend(refno_aabbs);
        offset += page_count;
    }
    dbg!(rstar_objs.len());

    //存储在全局变量里, 每次都重新加载，还是就用数据文件来表达？当做资源来加载，不用每次都去加载
    //加个时间戳，来表达是不是最新的rtree
    let tree = AccelerationTree::load(rstar_objs);
    // tree.serialize_to_bin_file();
    *GLOBAL_AABB_TREE.write().await = tree;

    Ok(true)
}

