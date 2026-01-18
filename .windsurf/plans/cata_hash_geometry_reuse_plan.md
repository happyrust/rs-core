# Cata Hash 与几何复用优化方案
本计划将基于现有 `cata_hash` 机制，引入可版本化的“几何签名（Geometry Signature）”，使几何参数等价的元件在模型生成阶段稳定共享 `inst_info/geo_relate`，减少重复求值与网格生成。

## 现状梳理（当前 cata_hash 机制）
- **生成位置**
  - `src/types/named_attmap.rs`：`NamedAttrMap::pe()` 在构造 `SPdmsElement` 时写入 `cata_hash: self.cal_cata_hash()`。
  - `src/types/pe.rs`：`SPdmsElement` 持久化时会把 `cata_hash` 存入 `pe` 表。
- **计算逻辑（`NamedAttrMap::cal_cata_hash`）**
  - `ref_name`：`NOZZ/ELCONN` 用 `CATR`，其它用 `SPRE`。
  - 若存在 `SPRE/CATR` 且 `TYPE` 不在 `CATA_WITHOUT_REUSE_GEO_NAMES`：
    - 以 `DefaultHasher` 计算哈希：
      - `SPRE/CATR`（字符串）
      - `DESP`（`hash_f32_slice`，内部对 f32 做 3 位小数 round）
      - `ANGL/HEIG/RADI`（以 `get_as_string` 得到字符串后参与哈希）
      - 若存在 `DRNS/DRNE`：加入二者与 `|POSE-POSS|`
      - `JUSL`（字符串）
    - 返回 `u64` 的十进制字符串。
  - 否则回退为 `REFNO` 字符串（包含 `_`）。
- **消费路径（复用的核心）**
  - `src/geometry/mod.rs`：`EleGeosInfo::id_str()`
    - 若 `cata_hash` 为空或包含 `_`：使用 `refno + sesno` 作为唯一 ID。
    - 否则：直接使用 `cata_hash` 作为 `inst_info` 的 ID，从而让多个 `pe` 指向同一个 `inst_info`。
  - `src/rs_surreal/query.rs`：`query_group_by_cata_hash()`
    - 以 `pe.cata_hash` 分组，并检测 `type::record('inst_info', cata_hash)` 是否存在，用于“只生成一次”。
  - 其它：`rs_surreal/resolve.rs` 中会用 `pe.cata_hash` 查 pseudo-attr 映射（说明 `cata_hash` 已成为跨流程的关键索引）。

## 关键结论：当前机制的优点与主要问题
- **优点**
  - 计算成本低：无需解析表达式或生成几何即可分组。
  - 对 `DESP` 做了 3 位小数 round，能一定程度容忍浮点噪声。
- **主要问题（会直接影响“几何等价复用”的正确性与命中率）**
  1. **哈希输入不完备**：几何求值真实依赖远不止 `DESP/ANGL/HEIG/RADI/JUSL`。
     - `get_or_create_cata_context()` 会注入 `CPAR/PARA`、`ODES/OPAR`、`ADES/APAR`、大量其它数值属性。
     - `cal_cata_hash` 未覆盖这些参数时，可能出现“不同几何却同 hash”的错误复用（代码注释也提到了 `ODESP` 导致复用问题）。
  2. **过度保守的不可复用列表**：`CATA_WITHOUT_REUSE_GEO_NAMES` 直接禁用多个类型的复用，导致复用收益被大幅削弱。
  3. **数值字段的字符串哈希不够稳健**：`ANGL/HEIG/RADI` 以字符串参与哈希，潜在受格式差异影响（例如 1 与 1.0）。
  4. **ID 规则隐含约束**：`EleGeosInfo::id_str()` 以“是否包含 `_`”判断是否可复用，因此新 key 需要避免 `_`，否则会被强制降级为不复用。

## 优化目标（第一性原理）
- **要共享模型生成，本质上要把“几何等价类”定义清楚**：
  - 同一等价类中的元素应生成同一份 `inst_info`（同一组 `geo_relate` + 对应的 `inst_geo` 引用与局部变换）。
  - 等价类的判定应只依赖“真正影响几何的输入”，而非无关元数据。
- **要可演进**：签名算法必须可版本化，避免一次性替换导致历史数据与缓存全部失效。

## 候选方案（从简单到可靠）
### 方案 A：在现有 `cata_hash` 基础上扩充字段 + 规范化
- 将 `ANGL/HEIG/RADI` 按数值解析后使用 `hash_f32`（3 位 round）参与哈希。
- 适度加入 `DDES* / CPAR* / ODES* / OPAR* / ADES* / APAR*` 等可能影响几何的字段。
- **优点**：仍可“生成前分组”。
- **缺点**：很难穷举“真正影响几何的字段”，容易再次漏掉或过度包含导致命中率下降。

### 方案 B（推荐）：几何签名 GeoSig v2 ——基于“表达式引用变量”的精确签名
- 思路：
  - 几何来自表达式求值；表达式里引用了哪些变量，就只把这些变量的“最终数值”纳入签名。
- 做法：
  - 在构建 `CataContext` 后，扫描 `ScomInfo` 中用于几何/轴点/数据集的表达式（GM/AXIS/DTSE）。
  - 提取被引用的 token（例如 `DESP1/CPAR3/ODES2/RPRO_LENG/...`），按名称排序。
  - 从 `CataContext` 取出这些变量的值（数值统一 round/量化），形成 `Vec<(name,value)>` 的 canonical 表示，并哈希。
  - `geo_sig` 形如 `g2:<hash>`（无 `_`）。
- **优点**：
  - “只看几何真正依赖的输入”，可显著减少错误复用；同时避免把无关字段纳入导致命中率下降。
  - 未来扩展新表达式/新变量也自动纳入。
- **代价**：需要一次解析表达式，但仍远比生成几何/网格便宜，而且可缓存。

### 方案 C：后验签名 GeoSig v3 ——基于“已解析的几何参数/形体列表”
- 在生成 `CateGeomsInfo` 或 `CateCsgShape` 后，对：
  - 形体类型 + `hash_unit_mesh_params()` + 局部变换（量化）+ geo_type
  - 做 canonical 序列化与哈希。
- **优点**：最接近“几何真值”，几乎不会错。
- **缺点**：无法在生成前分组；更适合作为“校验/回写/离线去重”。

## 推荐落地路径（渐进式、可回滚）
1. **新增签名字段**：在 `pe` 表增加 `geo_sig`（或复用 `cata_hash` 但加前缀 `g2:`），保持旧字段不动。
2. **生成流程优先使用新签名**：
   - 分组时优先用 `geo_sig`；若为空，回退用旧 `cata_hash`。
   - `inst_info` 的 record id 使用 `geo_sig`（避免 `_`），旧数据仍可兼容查询。
3. **一致性校验**：对同一旧 `cata_hash` 组内抽样/全量计算 `geo_sig`，若出现多个不同值：
   - 记录为“不可复用/需拆分”并输出统计（后续可用于修正策略或黑名单）。
4. **逐步放开不可复用类型**：从复用收益最大、已知稳定的类型开始（你来指定优先级），并用 `geo_sig` 校验兜底。

## 需要你确认的关键问题（决定签名粒度）
- **复用范围**：你希望共享的是
  - 仅 `inst_info/geo_relate`（形体组合与局部变换）
  - 还是连同布尔运算结果（`booled_id` 对应 mesh）也希望缓存复用？
- **精度容忍**：签名内数值 round 建议沿用 3 位小数（现有 `hash_f32` 方案）。是否需要更高/更低？
- **优先支持的 noun 类型**：先把哪些类型纳入可复用（目前被 `CATA_WITHOUT_REUSE_GEO_NAMES` 禁用的那些里，哪些最值得优先做）？

## 任务清单（实现阶段）
- 定义 `GeoSig` 的版本化格式与生成函数（推荐 GeoSig v2）。
- 在模型生成流程中：
  - 分组查询优先按 `geo_sig`；不存在则 fallback `cata_hash`。
  - 生成后回写 `pe.geo_sig`（便于下一次直接命中）。
- 增加校验与统计：检测同 `cata_hash` 内的 `geo_sig` 分裂情况，输出日志/指标。
- 分批启用复用类型并做回归测试（以你指定的数据集/场景为准）。
