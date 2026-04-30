# MBD V2 下一步开发计划（真实页面验收版）

> **目标**：先保证 BRAN 的尺寸标注正确显示，再逐步去掉 V1 算法。
> **最终验收入口**：`http://localhost:3101/?output_project=AvevaMarineSample&mbd_refno=24381_145712`
> **主验收 BRAN**：`24381_145712`

---

## 一、当前结论

当前 V2 不是完整新版算法，而是 V1 `LayoutResult` 到 V2 primitive 的过渡层。已具备 V2 类型、assembler、SmallDimSolver 基础集成、label 避让、leader 生成 / 重连 / reroute、leader-label 冲突检测、`plant-model-gen` 的真实 V2 后端 API，以及 plant3d-web 的 V2 primitive 过渡渲染接入：

```text
GET /api/mbd/v2/pipe/{refno}
```

仍未完成的是 `PolarSystem`、直接产出 V2 primitive 的 `BranchCalculatorV2`。plant3d-web 当前已在 layout_first 模式优先请求 `/api/mbd/v2/pipe/{refno}`，并把 V2 primitive 适配到现有三维标注渲染器；URL 可用 `mbd_api=v1` / `mbd_version=v1` 回滚到 V1。

后续开发的完成标准不能停在 rs-core JSON 输出。JSON 只能作为后端阶段验证，最终必须在 plant3d-web 真实页面显示正确。

---

## 二、处理顺序

1. **V2 API 接入** ✅
   - 在 `plant-model-gen` 增加或接入 `GET /api/mbd/v2/pipe/{refno}`。
   - V2 API 内部显式开启 small dim stacking 与 avoidance。
   - 保留 V1 API，迁移期并行。

2. **尺寸文字方向修正** ✅
   - `LinearDimPrimitive.text.orientation` 优先来自尺寸线方向。
   - `TextBlock.up` 优先来自尺寸偏移方向。
   - 无效方向回退到 `AssemblerContext` 默认方向，后续再补 warning issue。

3. **leader 生成接入** ✅
   - tag / weld 生成 label 时同步生成 `LeaderLinePrimitive`。
   - 使用 `route_leader_line` 选择文字框最近角。
   - 让现有 leader 检测与 reroute 能在真实输出中发挥作用。
   - label 被 avoidance 移动后，会按 label id 重连对应 leader 终点。

4. **chain 聚类修正** ✅
   - chain 分组以端点连接关系为主，不再依赖 `dim.direction` 对 start 点排序。
   - 乱序输入、多个 head、闭环或断链时输出稳定结果；后续再补 layout warning。

5. **pipeline 文档状态修正** ✅
   - `pipeline.rs` 文件头应写明当前已接入 small dim stacking 和 avoidance 开关。
   - 明确 `build_mbd_v2_pipe_data` 只是迁移期 bridge，不是最终 `BranchCalculatorV2`。

6. **plant3d-web V2 渲染接入** ✅
   - 保留现有 `/api/mbd/pipe/{refno}` 路径作为回滚。
   - layout_first 模式优先消费 `/api/mbd/v2/pipe/{refno}`。
   - 当前前端把 `linear_dim`、`slope_mark`、`label`、`leader_line` 适配到现有三维标注渲染器；后续再改为真正按 `primitive.kind` 原生渲染。

7. **下一步：BranchCalculatorV2 / PolarSystem**
   - 从 V1 `LayoutResult` bridge 继续下沉到真正的 V2 branch solver。
   - 复刻 aios_core / MDB 标注里对管轴极坐标、尺寸避让、最佳方向搜索的核心语义。
   - 目标是让 `build_mbd_v2_pipe_data` 不再依赖 V1 layout_result。

---

## 三、SurrealDB 查询规范

BRAN / HANG 尺寸标注的数据读取必须遵守以下规则：

- `tubi_relate` 查询直段必须使用复合 ID Range，不做全表扫描：

  ```sql
  SELECT *
  FROM tubi_relate:[pe:⟨24381_145712⟩, 0]..[pe:⟨24381_145712⟩, ..]
  ```

- 层级查询优先 TreeIndex / SceneTree；只有 TreeIndex 不覆盖时，才使用 SurrealDB 图遍历或递归。
- Rust 查询统一走 `SUL_DB.query_take::<T>(sql, 0).await?` 或 `query_response`。
- 查询结果必须使用 `#[derive(SurrealValue)]` 的强类型结构，不用 `serde_json::Value` 兜底。
- `ref0` 不是 `dbnum`，需要 dbnum 时必须通过项目既有映射取得。
- 下划线 refno（例如 `24381_145712`）是新协议优先格式。

---

## 四、三段验收

### 4.1 后端 JSON 阶段验收

```bash
curl -s "http://127.0.0.1:3100/api/mbd/v2/pipe/24381_145712?debug=true" | jq .
```

通过标准：

- `success = true`
- `data.version = "v2"`
- `data.primitives` 非空
- 至少包含 `linear_dim`
- 包含 `leader_line`
- `issues` 不包含 error

当前主样本 `24381_145712` 的已验证结果：

```json
{
  "success": true,
  "version": "v2",
  "input_refno": "24381_145712",
  "branch_refno": "24381_145712",
  "primitives": 19,
  "primitive_kinds": [
    { "kind": "label", "count": 4 },
    { "kind": "leader_line", "count": 4 },
    { "kind": "linear_dim", "count": 10 },
    { "kind": "slope_mark", "count": 1 }
  ],
  "issues": 0,
  "error_issues": 0
}
```

这一步只能证明后端数据可用，不能代表最终完成。

### 4.2 plant3d-web API 接入验收

当前前端事实：

- `output_project` 在 `App.vue` 中用于项目直达。
- `mbd_refno` 在 `ViewerPanel.vue` 中作为 URL 预加载优先参数，会触发 `requestMbdPipeAnnotation(refno)`；`mbd_pipe` 只是兼容参数。
- 当前 MBD 请求入口仍是 `getMbdPipeAnnotations()`，默认消费 V1 `/api/mbd/pipe/{refno}`。
- V1 API 现在已随 `web_server` feature 默认启用 `mbd-iso`，`mode=layout_first` 会返回 `layout_result`；这保证现有页面路径不会因为 V2 后端改造而失效。

V2 接入时，要么让 `getMbdPipeAnnotations()` 在明确 V2 开关下切到 V2 API，要么新增清晰的 V2 客户端入口；最终验收不能绕过 URL 预加载路径。

### 4.3 真实页面最终验收

最终验收只认以下入口：

```text
http://localhost:3101/?output_project=AvevaMarineSample&mbd_refno=24381_145712
```

必须满足：

- 页面能正常打开，不停留在项目加载状态。
- 当前项目是 `AvevaMarineSample`。
- `mbd_refno=24381_145712` 自动触发，不需要手动右键模型树。
- 后端成功返回该 BRAN 的管道标注数据。
- 三维视图中能看到该 BRAN 的管道模型。
- 管道尺寸标注正确显示：尺寸不缺失、不重复、方向不明显偏转、文字不堆叠到不可读，小尺寸能错层或缩字高。
- MBD 面板显示当前 BRAN/HANG 为 `24381_145712`。
- 浏览器控制台没有导致标注中断的 error。
- 后端接口没有返回 error 级 `issues`。

---

## 五、V1 下线条件

满足以下条件后，才能默认切 V2 并开始删除 V1 算法：

- 主验收 BRAN `24381_145712` 在真实页面稳定显示。
- 10 条典型 BRAN/HANG 样本通过真实页面或截图验收。
- 100 条 BRAN/HANG 批量 JSON 验证无 error 级 issues。
- 尺寸缺失率为 0，重复尺寸为 0。
- 尺寸方向无默认方向兜底。
- plant3d-web 不再依赖 V1 layout fallback 计算尺寸位置。
- 保留一个短期回滚开关，确认稳定后再完全删除 V1。
