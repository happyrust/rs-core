# 负实体计算链路（模型生成 → 关系写库 → 布尔运算）

## 1. 角色定义
- 正实体：主体几何，存储于 `inst_relate` / `inst_geo`。
- 负实体：需要从主体减去的几何；关系表 `neg_relate`（本元件）与 `ngmr_relate`（跨元件 NGMR）。
- 承载结构：`ShapeInstancesData` 的 `neg_relate_map`、`ngmr_neg_relate_map`（`rs-core/src/geometry/mod.rs`）。

## 2. 生成阶段的收集（ShapeInstancesData 构建）
- PRIM：几何生成后调用 `collect_descendant_filter_ids` 查子孙负实体，`insert_negs(refno, &neg_refnos)` 写入（`gen-model-fork/src/fast_model/prim_model.rs`）。
- LOOP：对 LOOP owner 先查直接负实体，再补充 `CMPF` 内的负实体并合并到同一正实例（`gen-model-fork/src/fast_model/loop_model.rs`）。
- CATA：根据 `pos_neg_map/neg_own_pos_map` 判定几何的负/Compound 类型；命中时写 `insert_negs`，有 NGMR 时通过 `insert_ngmr(ele_refno, owners, ngmr_geo)` 生成跨元件负关系（`gen-model-fork/src/fast_model/cata_model.rs`）。
- 上述批次通过 flume 发送 `ShapeInstancesData`，在 orchestrator 中消费。

## 3. 关系写库
- 入口：`save_instance_data_optimize`（`gen-model-fork/src/fast_model/pdms_inst.rs`）。
- 批量写入：
  - `neg_relate`: `{ in: 负实体, out: 正实体, id: [neg_refno, index] }`。
  - `ngmr_relate`: `{ in: 负实体载体, out: 正实体, ngmr: 负几何 }`。
  - 同步落盘 inst_info / inst_geo / geo_relate、aabb / trans / vec3。

## 4. 网格生成与布尔运算
- 基础网格：`gen_inst_meshes` 生成 mesh 与 aabb（`gen-model-fork/src/fast_model/mesh_generate.rs`）。
- 调度入口：`run_boolean_worker` / `booleans_meshes_in_db` 扫描 `neg_relate` + `ngmr_relate` 目标，确定布尔任务。
- 元件库布尔：`apply_cata_neg_boolean_manifold` 使用 `query_cata_neg_boolean_groups`，正/负 mesh 相减；成功新建 `inst_geo` 并标记 `inst_relate.booled=true`，失败 `bad_bool=true`（`gen-model-fork/src/fast_model/manifold_bool.rs`）。
- 实例级布尔：`apply_insts_boolean_manifold` → `query_negative_entities_batch` 过滤有负实体的实例 → `query_manifold_boolean_operations_batch_optimized` 获取变换，`apply_boolean_for_query` 将负实体转到正实体坐标系批量相减；成功更新 `inst_relate.booled_id`，失败标记 `bad_bool`。

## 5. 线框流程图
```
[模型生成]
  └─ 收集负实体/NGMR (prim/loop/cata)
       ↓ ShapeInstancesData.neg_relate_map / ngmr_neg_relate_map
[save_instance_data_optimize]
  └─ 批量写 neg_relate / ngmr_relate
       ↓ SurrealDB
[mesh_generate]
  └─ 基础 mesh + aabb
[boolean worker]
  ├─ apply_cata_neg_boolean_manifold (has_cata_neg)
  └─ apply_insts_boolean_manifold (neg_relate / ngmr_relate)
       ↓ booled_id / booled / bad_bool 标记
```

## 6. 排查要点
- 关系缺失：确认生成阶段是否调用 `insert_negs` / `insert_ngmr`，以及配置是否包含负实体类别。
- 布尔未执行：检查 mesh/aabb 是否生成；布尔 worker 仅处理存在负关系且 aabb 不为 NONE 的实例。
- SQL 快查：
  - `SELECT <-neg_relate<- FROM pe:⟨{refno}⟩;`
  - `SELECT <-ngmr_relate<- { in, ngmr } FROM pe:⟨{refno}⟩;`
  - `SELECT in, bad_bool FROM inst_relate WHERE bad_bool = true LIMIT 10;`
