# 方位计算系统指南

本文档详细说明了 rs-core 中的方位（Orientation）计算流程，包括核心计算函数的几何逻辑和 `DefaultStrategy` 的处理管线。这些函数的命名和逻辑旨在贴合底层几何构建原理。

## 1. 核心方位构建函数

这些函数位于 `src/rs_surreal/spatial.rs`，用于根据给定的轴向量构建局部坐标系基底（Basis）。

| 新函数名 | 原函数名 (参考) | 输入参数 | 几何构建逻辑 (Z=Primary, Y=Secondary) | 典型应用场景 |
| :--- | :--- | :--- | :--- | :--- |
| **`construct_basis_z_ref_x`** | `cal_ori_by_z_axis_ref_x` | `v` (Z轴) | Z=`v`. 若 Z 垂直，Ref=Y；否则 Ref=Z. <br> Y = Z × Ref. X = Y × Z. | 通用 Z 轴定向，偏向保持 Y 轴水平或指北。 |
| **`construct_basis_z_ref_y`** | `cal_ori_by_z_axis_ref_y` | `v` (Z轴) | Z=`v`. 若 Z 垂直，Ref=Y；否则 Ref=Z. <br> X = Ref × Z. Y = Z × X. | 类似上者，但构造顺序不同，用于部分土建截面。 |
| **`construct_basis_z_default`** | `cal_spine_orientation_basis` | `v` (Z轴), `neg` | **垂直时**：Y 指北 (Global Y). <br> **水平时**：Y 指上 (Global Z). | PDMS 默认方向规则（Implicit Direction）。 |
| **`construct_basis_z_y_hint`** | `cal_spine_orientation_basis_with_ydir` | `z`, `ydir`, `neg` | 优先使用 `ydir` 作为 Y 轴。若共线则回退到默认逻辑。 | GENSEC/WALL 子节点，受父级 YDIR 影响。 |
| **`construct_basis_z_y_exact`** | `cal_ori_by_ydir` | `y_ref`, `z` | Z=`z`. Y 由 `y_ref` 投影正交化得到。 | 显式 `YDIR` 属性处理。 |
| **`construct_basis_z_opdir`** | `cal_ori_by_opdir` | `v` (Z轴) | 类似 `ref_x`，但在特定角度下参考轴选取逻辑微调（Ref = -Y）。 | `OPDI` (操作方向) 属性。 |
| **`construct_basis_z_extrusion`** | `cal_ori_by_extru_axis` | `v` (Z轴), `neg` | Ref=X (垂直时) 或 Z. X = Ref × Z. | 挤出体截面方位计算。 |
| **`construct_basis_x_cutplane`** | `cal_cutp_ori` | `x_axis`, `cutp` | Y = cutp × x_axis. X = `x_axis`. Z = X × Y. | `CUTP` (切割平面) 属性。 |

## 2. 方位计算管线 (DefaultStrategy)

`DefaultStrategy` 用于大多数构件的局部变换计算。计算流程如下：

```mermaid
flowchart TD
    Start([开始计算 Local Transform]) --> InitPos[1. 初始化位置: POS + NPOS]
    InitPos --> BangCheck[2. 检查 BANG 属性]
    
    BangCheck --> ZDIS{3. 处理 ZDIS/PKDI}
    ZDIS -- POINSP 类型 --> ZdisPoinsp[特殊处理: 沿父级 Spine 偏移]
    ZDIS -- 其他 --> ZdisGen[通用处理: 计算 Z 轴位移或 Spine 映射]
    
    ZdisGen --> ParentInfo[4. 提取父级信息]
    ZdisPoinsp --> ParentInfo
    ParentInfo --> ExtruCheck{父级是 GENSEC?}
    ExtruCheck -- 是 --> GetSpine[获取父级 Spline 切向 (Z) 和 YDIR]
    ExtruCheck -- 否 --> DposeCheck[检查 DPOSE/DPOSS 计算轴向]
    
    DposeCheck --> InitRot[5. 初始化旋转 Quat]
    GetSpine --> InitRot
    
    InitRot --> HasOri{有显式 ORI 属性?}
    HasOri -- 是 --> UseOri[直接使用 ORI]
    HasOri -- 否 --> AutoOri{有推导轴向 (Z)?}
    
    AutoOri -- 是 (GENSEC子级) --> ConstructHint[construct_basis_z_y_hint]
    AutoOri -- 是 (其他) --> ConstructRefY[construct_basis_z_ref_y]
    AutoOri -- 否 --> Identity[保持 Identity]
    
    Identity --> PoslCheck{6. 有 POSL 属性?}
    UseOri --> PoslCheck
    ConstructHint --> PoslCheck
    ConstructRefY --> PoslCheck
    
    PoslCheck -- 是 --> HandlePosl[POSL/PLIN 复杂定位逻辑]
    PoslCheck -- 否 --> YdirOpdi[处理 YDIR / OPDI]
    
    YdirOpdi --> OpdiCheck{有 OPDI?}
    OpdiCheck -- 是 --> ConstructOpdi[construct_basis_z_opdir]
    OpdiCheck -- 否 --> YdirCheck{有 YDIR?}
    
    YdirCheck -- 是 --> ConstructYdir[construct_basis_z_y_exact]
    YdirCheck -- 否 --> Fallback[保持当前 Quat]
    
    ConstructOpdi --> ApplyBang[7. 应用 BANG 旋转 (Z轴)]
    ConstructYdir --> ApplyBang
    Fallback --> ApplyBang
    HandlePosl --> End([结束])
    
    ApplyBang --> CutpCheck{8. 有 CUTP 且无 OPDI/ORI?}
    CutpCheck -- 是 --> ConstructCutp[construct_basis_x_cutplane]
    CutpCheck -- 否 --> End
    
    ConstructCutp --> End
```

## 3. 关键逻辑说明

### 3.1 虚拟节点 (Virtual Nodes)

对于 `SPINE` 等虚拟节点，`get_local_transform` 会在管线最开始直接返回单位矩阵 (`Identity`)，不进行上述计算。这意味着虚拟节点不会对空间层级产生几何影响。

### 3.2 旋转初始化策略

- **优先显式属性**：如果构件有 `ORI` 属性，或者父级是 `GENSEC` 且构件是 `TMPL`，则优先使用该属性。
- **推导轴向**：如果构件没有 `ORI`，但父级提供了挤出方向（如 `GENSEC` 的 Spline 切向），则会根据该方向构建基底。
  - 对于 `GENSEC` 子节点，使用 `construct_basis_z_y_hint`，尝试继承父级的 YDIR。
  - 对于其他情况，使用 `construct_basis_z_ref_y`。

### 3.3 属性优先级

优先级从低到高（后续步骤覆盖前者）：

1. 默认 Identity
2. 自动推导（基于父级 Extrusion/Spline）
3. 显式 `ORI` 属性
4. `YDIR` / `OPDI` 属性
5. `BANG` (Beta Angle) 修正
6. `CUTP` (Cut Plane) 修正 (仅当无 OPDI/ORI 时)
