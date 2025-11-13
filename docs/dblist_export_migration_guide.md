# DBLIST 导出算法与迁移指南（基于 IDA 反汇编结论）

本文档总结了在目标二进制 `core.dll` 中实现 “DBLIST（数据库列表）导出/获取” 的核心算法与结构，并给出在本项目（Rust + SurrealDB）中的迁移建议与最小实现骨架。内容来源于对运行时已加载 `core.dll` 的 IDA Pro 分析。

## 1. 概述

- “DBLIST” 并非单一导出函数，而是围绕类 `DB_RefTabDbList` 的一套“按引用表条件筛选数据库”的缓存算法：
  - 构造时传入一个 `DB_Attribute*`，表示筛选所依据的“引用表/属性”。
  - 通过 `databases(out_vec)` 获取结果；若内部标记为脏（dirty），会先 `rebuildCache()` 重建缓存，再将缓存内容追加给调用方。
  - 缓存失效由 `invalidate()` 触发。
- 与 “DBLIST/MDBLIST/DBREF/TEAM/USER/…” 等字面量字符串有关的初始化逻辑存在于一处大型注册函数（`sub_10207360`），它负责 PML/OMD 元数据声明，并不直接产出 dblist；真正的筛选与结果集来自 `DB_RefTabDbList`。

## 2. 关键符号与地址（用于溯源）

- 构造：`DB_RefTabDbList::DB_RefTabDbList(const DB_Attribute*)` at `0x1055d000`
- 置脏：`DB_RefTabDbList::invalidate()` at `0x1055cef0`
- 重建：`DB_RefTabDbList::rebuildCache()` at `0x1055d110`
- 获取：`DB_RefTabDbList::databases(std::vector<DB_DB*>&)` at `0x1055d3c0`
- 相关初始化大函数：`sub_10207360`（出现 “DBLIST/MDBLIST/…” 字符串）

提示：以上 RVA/地址取自当前构建与会话，跨版本可能变化，建议将其作为“符号与行为”的证据而非硬编码依赖。

## 3. 对象布局与职责

`DB_RefTabDbList` 的内存布局（经反编译与访问模式推断）：

- `this+0x00`: vftable
- `this+0x04`: `DB_Attribute* attr`（用于限定引用表/属性）
- `this+0x08`: `dirty: uint8`（构造为 1；`invalidate()` 置 1；`rebuildCache()` 结束置 0）
- `this+0x0c..0x14`: 内部缓存向量的 `[begin, end, capacity_end]`（元素为 `DB_DB*`）
- 析构会释放缓存区并清零三指针槽。

职责划分：

- `invalidate()`：将内部状态置脏，不做其它操作。
- `rebuildCache()`：枚举所有候选数据库，对每个库做“引用表是否存在至少一条匹配项”的检查，匹配则加入内部缓存。
- `databases(out)`：如有脏则先重建，再将缓存批量追加到调用方向量。

## 4. 调用流与筛选逻辑

重建流程（反编译伪代码语义）：

1. 清空内部缓存 `end = begin`。
2. 通过 `DB_MDB::instance(); DB_MDB::dbs(0, out_tmp)` 获得所有候选 `DB_DB*`。
3. 遍历每个 `db`：
   - 构造 `DB_RefTableIterator itr(db, this->attr, default DB_Element)`。
   - 如果迭代器显示“存在内容”（反编译中体现为检查两个内部标志/计数 `v29 || v30`）则 `cache.push_back(db)`。
   - 销毁迭代器。
4. 将 `dirty = 0`；释放临时枚举缓冲。

获取流程：

- `databases(out)`：
  - 若 `dirty == 1` 调用 `rebuildCache()`；
  - 将 `[cache.begin, cache.end)` 追加复制到 `out`。

复杂度：

- 时间复杂度约 `O(N · Cattr)`，其中 N 为数据库数目、`Cattr` 为“某库上检查引用表存在性”的开销。
- 空间复杂度约 `O(K)`，K 为匹配库数量。

## 5. 与字符串“DBLIST/MDBLIST”的关系

在 `sub_10207360` 中可见“DBLIST / MDBLIST / DBREF / TEAM / USER / AUTHUSER …”等字面量，此处为 PML/OMD 的对象/命令定义注册。它们为上层命令语义提供“名-义”绑定，而真正的 dblist 结果由 `DB_RefTabDbList` 在数据层依据 `DB_Attribute` + 引用表迭代器生成。

## 6. 错误与边界行为

- 内部向量扩容采用典型 `std::vector` 策略（约 1.5x 倍增，代码中见 `>> 1`），并在极端情况下抛出 `std::_Xlength_error("vector<T> too long")`。
- `invalidate()` 非线程安全标志位；若实例跨线程共享，需要外部同步。
- `DB_MDB::dbs` 与 `DB_RefTableIterator` 的异常/空集情况会导致缓存为空；`databases(out)` 仍返回空列表。

## 7. 可移植的抽象与伪代码

独立于具体实现的抽象接口：

```
type DbHandle
type Attribute

fn list_dbs() -> Vec<DbHandle>
fn has_any(db: &DbHandle, attr: &Attribute) -> bool  // 存在性查询：是否至少一条

struct RefTabDbList {
  attr: Attribute,
  dirty: bool,
  cache: Vec<DbHandle>,
}

impl RefTabDbList {
  fn new(attr: Attribute) -> Self { dirty = true; cache = [] }
  fn invalidate(&mut self) { self.dirty = true }
  fn rebuild_cache(&mut self) {
    self.cache.clear();
    for db in list_dbs() {
      if has_any(&db, &self.attr) {
        self.cache.push(db);
      }
    }
    self.dirty = false;
  }
  fn databases(&mut self, out: &mut Vec<DbHandle>) {
    if self.dirty { self.rebuild_cache(); }
    out.extend(self.cache.iter().cloned());
  }
}
```

## 8. 在本项目中的迁移设计（Rust + SurrealDB）

迁移目标：复刻“按引用表存在性筛选数据库”的缓存机制，同时对接本项目 SurrealDB 查询扩展。注意遵循仓库规范：

- 查询必须使用 `SurrealQueryExt` 扩展方法（例如 `query_take::<T>(sql, index)`），禁止直接 `.query().await?.take()`。
- 目标类型 `T` 须满足 `T: SurrealValue` 与 `usize: SurrealQueryResult<T>`。

建议设计：

- `RefTabDbList<A, D>`：其中 `A` 为属性描述（或引用表上下文），`D` 为数据库句柄（或连接标识）。
- `invalidate()`：由外层在数据库新增/删除、引用表数据变动、属性切换、上下文变更时调用。
- `rebuild_cache()`：
  - `list_dbs()`：枚举当前上下文下需纳入筛选的数据库集合（取决于你们的“MDB 注册表/连接池”）。
  - `has_any(db, attr)`：在该库上执行 `SELECT count() FROM <ref_table> WHERE <attr 条件> LIMIT 1` 或 `SELECT 1 ... LIMIT 1`；只要 `count > 0` 即视为存在。

Surreal 查询示例（仅示意，注意替换表名/条件）：

```rust
use surrealdb::engine::any::Any;
use surrealdb::Surreal;
use crate::rs_surreal::query_ext::SurrealQueryExt;

async fn has_any(db: &Surreal<Any>, table: &str, attr_key: &str, attr_val: &str) -> anyhow::Result<bool> {
    // 推荐：count() + LIMIT 1，返回 i64
    let sql = format!("SELECT count() FROM {} WHERE {} = $val LIMIT 1", table, attr_key);
    // 注意：此处仅示例参数绑定形式，实际按你们项目的参数规范实现
    // 若项目已有统一的带绑定/trace 的查询包装，应复用之
    let cnt: i64 = db.query_take(&sql, 0).await?;
    Ok(cnt > 0)
}
```

最小骨架（可放入 `src/rs_surreal/` 相邻模块；接口按需调整）：

```rust
pub struct RefTabDbList<A, D> {
    attr: A,
    dirty: bool,
    cache: Vec<D>,
}

impl<A: Clone, D: Clone> RefTabDbList<A, D> {
    pub fn new(attr: A) -> Self {
        Self { attr, dirty: true, cache: Vec::new() }
    }
    pub fn invalidate(&mut self) { self.dirty = true; }
}

impl<A, D> RefTabDbList<A, D> {
    pub async fn rebuild_cache<E, L, H>(&mut self, mut list_dbs: L, mut has_any: H) -> Result<(), E>
    where
        L: FnMut() -> Result<Vec<D>, E>,
        H: FnMut(&D, &A) -> Result<bool, E>,
    {
        self.cache.clear();
        for db in list_dbs()? {
            if has_any(&db, &self.attr)? {
                self.cache.push(db);
            }
        }
        self.dirty = false;
        Ok(())
    }

    pub async fn databases<E, L, H>(&mut self, out: &mut Vec<D>, list_dbs: L, has_any: H) -> Result<(), E>
    where
        L: FnMut() -> Result<Vec<D>, E>,
        H: FnMut(&D, &A) -> Result<bool, E>,
    {
        if self.dirty {
            self.rebuild_cache(list_dbs, has_any).await?;
        }
        out.extend(self.cache.iter().cloned());
        Ok(())
    }
}
```

将以上与本项目的 SurrealDB 扩展结合时，务必：

- 通过统一的 `Surreal<Any>` 连接与你们的“数据库枚举”逻辑协调（如存在多逻辑库/命名空间）。
- 所有查询走 `SurrealQueryExt` 的 `query_take` / `query_response`，以继承现有的报错与追踪机制（`#[track_caller]`）。
- 选择明确的目标类型（如 `i64` 用于 `count()`），避免手动解析原始枚举。

## 9. 失效触发建议

- MDB 环境变更：数据库创建/删除/启用/禁用。
- 引用表数据变更：影响目标 `attr` 的插入/删除/更新。
- 属性切换：`attr` 或其映射（表名/字段名/筛选条件）发生变化。
- 连接上下文变更：租户/命名空间切换等。

以上任一事件后，应调用 `invalidate()`；下一次 `databases()` 会自动重建缓存。

## 10. 测试与验证清单

- 功能正确性：
  - 构造夹具：若干数据库中，仅部分在目标引用表上存在项。
  - 调用 `databases()`：仅返回有项的数据库集合。
- 缓存行为：
  - 连续两次 `databases()`（中间无 `invalidate`）第二次不触发底层查询。
  - 数据变更后 `invalidate()`，再 `databases()` 结果更新。
- 边界情况：
  - 零数据库、零匹配、全部匹配。
  - 大量数据库 + 稀疏匹配，观测扩容与性能。
  - 异常传播：连接失败/查询失败能在调用栈中携带 SQL 与调用位置信息。

## 11. 迁移注意事项

- 线程安全：需要自行为 `dirty` 与 `cache` 做并发保护（如放入 `Arc<RwLock<_>>` 或单线程内使用）。
- 资源释放：Rust `Vec` 自动管理容量；无需手动 `operator delete`。
- 性能：优先用“存在性”短路查询（`LIMIT 1` 或 `count()`），避免全表扫描。
- 可观测性：建议在 `invalidate()`、`rebuild_cache()` 入口/出口打统一日志，便于问题定位与性能跟踪。

---

附：与 IDA 相关的线索摘要

- “DBLIST/MDBLIST/DBREF/TEAM/USER/…” 字面量集中于 `sub_10207360`（PML/OMD 注册流），并多次调用 `PMLOMD/PMLAMD/PMLCMD/PMLEngine::endObjectDefinition`。
- `DB_RefTabDbList::rebuildCache()` 明确调用 `DB_MDB::dbs(0, &tmp)` 获得候选库，再以 `DB_RefTableIterator` + `attr` 判定是否存在项。
- `DB_RefTabDbList::databases()` 在 `dirty` 时触发重建，随后以向量批量拷贝方式返回。

