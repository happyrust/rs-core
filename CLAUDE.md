# CLAUDE.md
 没有我的命令，不要随意创建测试文件，你要先出方案，然后让我确认之后才能修改和创建文件。
我是 plan 模式的情况在，不要开启代码编辑，我只是让你先出方案设计，只有我确定了方案之后，你才能修改。

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is `aios_core`, a Rust-based plant engineering and design management system. The project handles complex industrial plant data, geometry, materials, and spatial relationships. It integrates with PDMS (Plant Design Management System) data and provides APIs for querying, managing, and analyzing plant design information.

## Build and Development Commands

### Basic Operations
```bash
# Build the project
cargo build

# Run tests
cargo test

# Build with verbose output
cargo build --verbose

# Run tests with verbose output
cargo test --verbose

# Build for release
cargo build --release
```

### Toolchain Information
- Uses Rust nightly toolchain (specified in `rust-toolchain.toml`)
- Edition: 2024
- Uses experimental features: `let_chains`, `trivial_bounds`, `result_flattening`

### Testing
```bash
# Run specific test modules
cargo test test_surreal
cargo test test_spatial
cargo test test_material

# Run tests for specific features
cargo test --features manifold
cargo test --features sql
```

## Architecture Overview

### Core Module Structure

**Database Layer (`rs_surreal/`)**
- Primary database integration with SurrealDB
- Connection management through `SUL_DB` and `SECOND_SUL_DB` global instances
- Query building, spatial operations, and data versioning
- Material list management with domain-specific modules (dq, gps, gy, nt, tf, tx, yk)

**Database Schema Architecture**
- **PE 表 (Plant Element Table)**: 统一存储所有类型的植物工程元素
  - 所有元素（SITE、ZONE、EQUI、PIPE 等）都存储在同一个 `pe` 表中
  - 通过 `noun` 字段区分元素类型（如 'SITE', 'ZONE', 'EQUI', 'PIPE'）
  - 每个元素通过 `refno` (RefU64) 唯一标识
  - 包含 `dbnum` (数据库编号)、`sesno` (会话编号)、`deleted` (删除标记) 等字段

- **类型表（WORL、SITE、ZONE、EQUI 等）与 PE 表的关系**:
  - **重要架构**: 存在 WORL、SITE、ZONE、EQUI、PIPE 等类型表
  - 这些类型表中的 **REFNO 字段指向 pe 表中的记录**
  - 类型表存储特定类型的属性和元数据
  - **pe 表是核心存储**，类型表通过 REFNO 引用 pe 表
  - **查询示例**: `SELECT value REFNO from WORL WHERE REFNO.dbnum = 1112` 返回的是 pe 表中的 id
  - **关键**: 当需要在 pe_owner 关系中使用时，必须通过 REFNO 获取 pe 表的引用

- **pe_owner 关系表**: 表示 PE 元素之间的父子层级关系
  - **关系方向**: `child (in) -[pe_owner]-> parent (out)`
  - `in` 字段指向子节点 (child PE element in pe table)
  - `out` 字段指向父节点 (parent PE element in pe table)
  - 这是一个通用的关系表，适用于所有类型的 PE 元素
  - **重要**: `pe_owner` 针对的是 `pe` 表之间的连接关系，不是针对具体类型表（如 WORL、SITE、ZONE 表）
  - **查询子节点**: 必须使用 pe 表的 id，例如：
    ```sql
    let $world_pe_id = (SELECT value REFNO from WORL WHERE ...)[0];
    SELECT value in FROM $world_pe_id<-pe_owner WHERE in.noun = 'SITE';
    ```

- **层级结构示例**:
  ```
  MDB (多数据库)
    └─ WORL (世界节点, dbnum 标识)
        └─ SITE (站点)
            └─ ZONE (区域)
                └─ EQUI (设备)
                    └─ PIPE (管道)
  ```

- **查询模式**:
  - 查询子节点: `SELECT VALUE in FROM pe_owner WHERE out = <parent_id> AND in.deleted = false`
  - 查询父节点: `SELECT VALUE out FROM pe_owner WHERE in = <child_id>`
  - 反向遍历: `<node_id><-pe_owner` (查找所有指向该节点的关系)
  - 图查询语法: `node->edge_table->target` 或 `node<-edge_table<-source`

- **RELATE 语句 - 创建图关系**:
  - 基本语法: `RELATE from_record->table->to_record`
  - 示例: `RELATE person:aristotle->wrote->article:on_sleep`
  - 结果会自动生成包含 `in`, `out`, `id` 三个字段的关系记录
  - `in` 字段存储关系的源节点（from_record）
  - `out` 字段存储关系的目标节点（to_record）
  - 可以在关系上添加额外数据: `RELATE a->r->b SET field = value` 或 `CONTENT {...}`

- **图遍历查询**:
  - 正向遍历: `SELECT ->wrote->article FROM person` (查询 person 写的 article)
  - 反向遍历: `SELECT <-wrote<-person FROM article` (查询写了 article 的 person)
  - 双向遍历: `SELECT <->sister_of<->city FROM city` (适用于对称关系)
  - 直接返回: `RETURN person:tobie->purchased->product` (直接获取结果)
  - 条件过滤: `->edge[WHERE condition]->target` (在遍历时过滤)
  - 递归查询: `@.{n}->edge->target` (递归 n 层) 或 `@.{1..20}` (范围递归)

**Data Types (`types/`)**
- Core data structures: `RefNo`, `AttMap`, `AttVal`, `NamedAttMap`
- Database info structures and query SQL builders
- Hash utilities and reference number management

**Geometry and Spatial (`prim_geo/`, `spatial/`, `geometry/`)**
- Primitive geometric shapes: cylinders, spheres, boxes, pyramids, etc.
- Spatial calculations and acceleration trees
- Room and zone management with AABB (Axis-Aligned Bounding Box) trees

**Materials and Plant Data (`material/`, `room/`)**
- Material classification and management (dq, gps, gy, nt, sb, tf, tx, yk systems)
- Room calculations, hierarchy, and spatial relationships
- HVAC and piping material calculations

**Configuration and Options**
- Database configuration through `DbOption.toml` files
- Multiple environment support (ABA, AMS variants)
- Project-specific settings and connection strings

### Key Design Patterns

**Database Abstraction**
- Global database connections (`SUL_DB`, `SECOND_SUL_DB`)
- Async initialization functions (`init_surreal()`, `init_test_surreal()`)
- Configuration-driven database setup

**Type System**
- Heavy use of reference numbers (`RefNo`, `RefU64`) for entity identification
- Attribute maps for flexible property storage
- Strong typing with custom derive macros

**Modular Architecture**
- Feature-gated compilation (occ, manifold, sql, render)
- Domain-specific modules for different plant systems
- Clear separation between data types, operations, and storage

## Configuration Files

- `DbOption.toml` - Primary database configuration
- `DbOption_ABA.toml`, `DbOption_AMS.toml` - Environment-specific configs
- `all_attr_info.json` - PDMS database metadata
- Material configuration Excel files in `src/rs_surreal/material_list/tf/`

## Version Control API

The project includes a version management system:

```rust
// Query all history for a session number
aios_core::query_ses_history(sesno: i32) -> Vec<HisRefno>

// Query history for a specific reference number
aios_core::query_history_data(refno: Refno) -> Vec<HisRefno>

// Get differences between two session numbers
aios_core::diff_sesno(refno: Refno, sesno1: i32, sesno2: i32) -> Vec<Diff>
```

## Important Dependencies

- **SurrealDB**: Primary database (custom fork from gitee.com/happydpc/surrealdb)
- **Bevy**: Math and transform utilities
- **Glam**: Vector mathematics
- **Parry**: Geometric collision detection
- **Nalgebra**: Linear algebra operations
- **Manifold**: 3D geometry operations (feature-gated)

## Development Notes

- The project uses experimental Rust features - ensure nightly toolchain
- Database connections require valid `DbOption.toml` configuration
- Material list generation involves complex SurrealQL scripts in `src/rs_surreal/material_list/`
- Spatial computations are performance-critical and use acceleration structures
- Test modules are organized by functional area under `src/test/`

## Working with the Codebase

When making changes:
1. Understand the modular structure - changes often span multiple modules
2. Database queries use SurrealQL - see examples in `material_list/` subdirectories
3. Geometric operations require understanding of the coordinate systems used
4. Material calculations follow domain-specific business rules
5. Always test with various `DbOption.toml` configurations
- add to memory

## aios-core Surreal 查询速查

- 全局实例与扩展：`aios_core::SUL_DB` + `SurrealQueryExt`（`query_take` / `query_response`）
- 单/多结果查询：
```rust
use aios_core::{SUL_DB, SurrealQueryExt, RefnoEnum};

// 单结果查询
let refnos: Vec<RefnoEnum> = SUL_DB.query_take(
  "SELECT value id FROM pe WHERE noun = 'EQUI' LIMIT 10",
  0
).await?;

// 多结果查询
let sql = r#"
  SELECT * FROM pe WHERE noun = 'SITE' LIMIT 5;
  SELECT count() FROM pe;
"#;
let mut resp = SUL_DB.query_response(sql).await?;
let sites: Vec<SPdmsElement> = resp.take(0)?;
let count: i64 = resp.take(1)?;
```

- 层级查询（子孙/子节点/祖先）：
```rust
// 子孙节点 ID（支持类型过滤与层级范围，如 Some("1..5")）
let ids = aios_core::collect_descendant_filter_ids(&[root], &["EQUI", "PIPE"], None).await?;

// 子节点 ID（仅一层）
let children = aios_core::collect_children_filter_ids(root, &[]).await?;

// 祖先查询（向上）
let zones = aios_core::query_filter_ancestors(equip, &["ZONE"]).await?;
```

- 最佳实践：类型安全目标类型、批量优于循环、限制层级范围、利用 `#[cached]` 缓存

> 完整说明参见 gen-model-fork 仓库的 `docs/AIOS_CORE_QUERY_GUIDE.md`。
