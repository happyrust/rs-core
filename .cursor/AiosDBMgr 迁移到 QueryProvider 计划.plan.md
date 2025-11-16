<!-- 86c8b9cf-3365-4327-990d-726cfdc6724d 4dea6131-a106-4021-b722-780cd2424cac -->
# AiosDBMgr 迁移到 QueryProvider 计划（包含 rs-plant3-d）

## 一、依赖分析总结

### 1.1 AiosDBMgr 的核心功能

**AiosDBMgr** 提供三类功能：

1. **PdmsDataInterface trait 实现** - PDMS 领域特定的数据访问接口

   - 位置：`rs-core/src/aios_db_mgr/aios_mgr.rs:118-312`
   - 方法：`get_world`, `get_pdms_element`, `get_attr`, `get_children`, `get_ipara_from_bran` 等

2. **MySQL 连接池管理** - 静态方法提供连接池

   - `get_project_pool()` - 获取项目 MySQL 连接池
   - `get_global_pool()` - 获取全局 MySQL 连接池
   - `get_puhua_pool()` - 获取浦华数据库连接池
   - `get_project_pools()` - 获取多个项目的连接池映射

3. **SurrealDB 初始化** - 数据库连接初始化

   - `init_from_db_option()` - 从配置文件初始化 SurrealDB 连接

### 1.2 依赖 AiosDBMgr 的文件清单

#### rs-core 仓库

**材料模块（使用 MySQL 连接池）**：

- `src/material/dq.rs:155` - `AiosDBMgr::get_project_pool()`
- `src/material/gps.rs:89` - `AiosDBMgr::get_project_pool()`
- `src/material/gy.rs:150,213,270` - `AiosDBMgr::get_project_pool()`
- `src/material/nt.rs:78` - `AiosDBMgr::get_project_pool()`
- `src/material/sb.rs:60` - `AiosDBMgr::get_project_pool()`
- `src/material/tf.rs:42` - `AiosDBMgr::get_project_pool()`
- `src/material/tx.rs:85` - `AiosDBMgr::get_project_pool()`
- `src/material/yk.rs:115,175,235` - `AiosDBMgr::get_project_pool()`

**其他模块**：

- `src/ssc_setting.rs:1284,1297,1304` - `AiosDBMgr::init_from_db_option()` 和 `PdmsDataInterface`
- `src/rs_surreal/datacenter_query.rs:3` - 导入 AiosDBMgr

#### gen-model 仓库

- `src/lib.rs:21,290` - `AiosDBMgr::init_from_db_option()` 和 `PdmsDataInterface`
- `src/versioned_db/database.rs:9,346` - `AiosDBMgr::init_from_db_option()`, `get_global_pool()`, `get_project_pool()`
- `src/versioned_db/pe.rs:6` - 导入但可能未使用
- `src/team_data.rs:3,75` - `get_project_pool()` 和 `PdmsDataInterface`

#### rs-plant3-d 仓库

**直接依赖**：无

- 代码中未发现直接使用 `AiosDBMgr` 的情况
- 文档中提到了 AiosDBMgr，但引用的是 `rs-server` 项目的代码，不是 rs-plant3-d 本身的代码

**间接依赖**（通过 aios_core）：

- `src/plugins/review_plugin/mod.rs:145-197` - 使用 `aios_core::connect_surdb()` 和 `aios_core::options::DbOption`
- `src/local_spatial_query.rs:90` - 使用 `aios_core::get_world_transform()`
- `src/grpc_service/spatial_query_service_aios.rs:1-2` - 使用 `aios_core::pdms_types::RefU64` 和 `aios_core::spatial::sqlite`
- 大量文件（192个）使用 `aios_core` 的各种查询函数，但未直接依赖 AiosDBMgr

**影响评估**：

- ✅ **低风险**：rs-plant3-d 主要使用 aios_core 的底层函数，不直接依赖 AiosDBMgr
- ⚠️ **需要注意**：如果 aios_core 中的某些函数内部使用了 AiosDBMgr，需要检查并迁移
- 📝 **建议**：在迁移过程中，确保 aios_core 提供的公共 API 保持稳定，避免影响 rs-plant3-d

## 二、迁移策略

### 2.1 功能映射关系

| AiosDBMgr 功能 | QueryProvider 对应 | 迁移方案 |

|---------------|-------------------|---------|

| `PdmsDataInterface` trait 方法 | `QueryProvider` trait | 需要创建基于 QueryProvider 的 `PdmsDataInterface` 实现 |

| `get_project_pool()` 等 MySQL 连接池 | ❌ 无对应 | 需要独立的连接池管理模块 |

| `init_from_db_option()` | `QueryRouter::surreal_only()` | 直接替换 |

### 2.2 迁移方案设计

#### 方案 A：完全移除 AiosDBMgr（推荐）

**步骤 1：创建 MySQL 连接池管理模块**

- 新建 `rs-core/src/db_pool/` 模块
- 提供静态方法 `get_project_pool()`, `get_global_pool()` 等
- 从 `AiosDBMgr` 迁移连接池逻辑

**步骤 2：创建基于 QueryProvider 的 PdmsDataInterface 实现**

- 新建 `rs-core/src/aios_db_mgr/provider_impl.rs`
- 实现 `PdmsDataInterface` trait，内部使用 `QueryProvider`
- 将 `AiosDBMgr` 的 `PdmsDataInterface` 实现迁移到新实现

**步骤 3：替换所有使用点**

- 材料模块：`AiosDBMgr::get_project_pool()` → `db_pool::get_project_pool()`
- 其他模块：`AiosDBMgr::init_from_db_option()` → `QueryRouter::surreal_only()`
- `PdmsDataInterface` 使用：创建 `ProviderPdmsInterface` 实例

**步骤 4：确保 aios_core API 兼容性**

- 检查 aios_core 公共 API 中是否有内部使用 AiosDBMgr 的情况
- 确保所有 aios_core 提供的函数在迁移后仍能正常工作
- 特别关注 rs-plant3-d 使用的函数（`get_world_transform`, `connect_surdb` 等）

## 三、详细迁移步骤

### 3.1 创建 MySQL 连接池管理模块

**文件**：`rs-core/src/db_pool/mod.rs`

```rust
// 从 AiosDBMgr 迁移连接池相关代码
pub async fn get_project_pool() -> anyhow::Result<Pool<MySql>> { ... }
pub async fn get_global_pool() -> anyhow::Result<Pool<MySql>> { ... }
pub async fn get_puhua_pool() -> anyhow::Result<Pool<MySql>> { ... }
```

### 3.2 创建基于 QueryProvider 的 PdmsDataInterface 实现

**文件**：`rs-core/src/aios_db_mgr/provider_impl.rs`

```rust
pub struct ProviderPdmsInterface {
    provider: Arc<dyn QueryProvider>,
}

impl PdmsDataInterface for ProviderPdmsInterface {
    // 使用 QueryProvider 实现所有方法
    async fn get_world(&self, mdb_name: &str) -> ... {
        // 使用 provider.query_by_type() 等
    }
    // ...
}
```

### 3.3 替换使用点（按模块）

**材料模块** (`rs-core/src/material/*.rs`)：

- 替换 `AiosDBMgr::get_project_pool()` → `db_pool::get_project_pool()`
- 移除 `use crate::aios_db_mgr::aios_mgr::AiosDBMgr;`

**gen-model 仓库**：

- `src/lib.rs:290` - 替换为 `QueryRouter::surreal_only()` 和 `ProviderPdmsInterface`
- `src/versioned_db/database.rs:346` - 替换连接池获取方式
- `src/team_data.rs:75` - 替换连接池获取方式

**rs-plant3-d 仓库**：

- ✅ 无需修改（不直接依赖 AiosDBMgr）
- ⚠️ 需要验证 aios_core API 兼容性

## 四、风险评估

### 4.1 高风险点

1. **MySQL 连接池依赖** - 材料模块大量依赖 MySQL，需要确保连接池管理正确
2. **PdmsDataInterface 兼容性** - 确保新实现与现有调用完全兼容
3. **初始化顺序** - SurrealDB 初始化可能影响其他模块
4. **aios_core API 稳定性** - rs-plant3-d 大量使用 aios_core API，需要确保迁移不影响这些 API

### 4.2 测试要求

- 所有材料模块的单元测试
- `PdmsDataInterface` 的集成测试
- 数据库连接池的生命周期测试
- **rs-plant3-d 的回归测试** - 确保迁移后 rs-plant3-d 仍能正常工作

## 五、迁移顺序建议

1. **第一阶段**：创建新模块（db_pool, provider_impl），不删除旧代码
2. **第二阶段**：逐个模块迁移，每个模块迁移后运行测试
3. **第三阶段**：验证 aios_core API 兼容性，确保 rs-plant3-d 不受影响
4. **第四阶段**：移除 AiosDBMgr，清理未使用的导入

## 六、关键决策（已确认）

1. ✅ **MySQL 连接池需要抽象化** - 创建 `ConnectionPoolManager` trait，不使用静态方法
2. ✅ **PdmsDataInterface 需要完全移除** - 所有功能迁移到 QueryProvider
3. ✅ **AiosDBMgr 需要完全移除** - 不留兼容层
4. ⚠️ **需要检查**：aios_core 中哪些函数内部使用了 AiosDBMgr 或 PdmsDataInterface？需要逐一检查并迁移

### To-dos

- [ ] 分析所有依赖 AiosDBMgr 的代码位置和使用模式
- [ ] 创建独立的 MySQL 连接池管理模块 (rs-core/src/db_pool/)
- [ ] 创建基于 QueryProvider 的 PdmsDataInterface 实现 (rs-core/src/aios_db_mgr/provider_impl.rs)
- [ ] 迁移材料模块 (dq, gps, gy, nt, sb, tf, tx, yk) 使用新的连接池模块
- [ ] 迁移 gen-model 仓库中的 AiosDBMgr 使用
- [ ] 迁移 ssc_setting.rs 和 datacenter_query.rs
- [ ] 移除 AiosDBMgr 相关代码和未使用的导入
- [ ] 更新所有相关测试，确保迁移后功能正常