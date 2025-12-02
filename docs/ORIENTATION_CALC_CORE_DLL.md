# core.dll 方位计算算法（YDIR + POSL + DELP）

> 本文完全基于 IDA Pro 对 **core.dll** 的静态分析，总结 AVEVA 内核中 YDIR / POSL / DELP 与 `D3_Transform` 之间的方位计算关系，不涉及本仓库代码实现。

## 1. 相关类型与入口函数（来自 core.dll 符号）

- **几何与变换类型**（通过符号名与重载可确定）
  - `D3_Transform`
    - `??0D3_Transform@@QAE@ABVD3_Matrix@@ABVD3_Vector@@@Z`：由旋转矩阵 + 位移向量构造
    - `?setRotation@D3_Transform@@QAEXABVD3_Matrix@@@Z`
    - `?setShift@D3_Transform@@QAEXABVD3_Vector@@@Z`
    - `?getShift@D3_Transform@@QBEABVD3_Vector@@XZ`
    - `?moveBy@D3_Transform@@QAEXABVD3_Vector@@@Z`
  - `D3_Matrix`, `D3_Vector`, `D3_Point`：作为 `getAtt` 重载的目标类型出现

- **元素接口**
  - `?getPosOriDirAsDouble@DB_Element@@ABE_NPBVDB_Attribute@@ABVDB_Qualifier@@AAV?$vector@NV?$allocator@N@std@@@std@@@Z`
    - 核心作用：给定某个几何相关属性，输出一组 `double`，语义上对应“位置 + 方位 + 方向”
  - `?getAtt@DB_Element@@QBE_NPBVDB_Attribute@@ABVDB_Qualifier@@AAVD3_Point@@@Z`
  - `?getAtt@DB_Element@@QBE_NPBVDB_Attribute@@ABVDB_Qualifier@@AAVD3_Vector@@@Z`
  - `?getAtt@DB_Element@@QBE_NPBVDB_Attribute@@ABVDB_Qualifier@@AAVD3_Matrix@@@Z`
  - `?calculateEulerAngles@DB_Element@@SAXABVD3_Matrix@@AAV?$vector@NV?$allocator@N@std@@@std@@@Z`

- **PLINE / 位置线相关函数**
  - `PLNPOS`（地址 `0x1020313c`）—— 通过 POSL 在线上求 3D 位置
  - `PLNDIR`（地址 `0x10203994`）—— 通过 POSL 在线上求 3D 方向（切线）
  - `?transformPos@DBE_Pline@@CA_NABVDB_Element@@0ABVD3_Point@@AAV3@AAPAVMR_Message@@@Z`
  - `?transformDir@DBE_Pline@@CA_NABVDB_Element@@0ABVD3_Vector@@AAV3@AAPAVMR_Message@@@Z`

- **属性与名词常量**
  - `?ATT_YDIR@@3QBVDB_Attribute@@B`
  - `?ATT_POSL@@3QBVDB_Attribute@@B`
  - `?ATT_DELP@@3QBVDB_Attribute@@B`
  - `?NOUN_SPINE@@3QBVDB_Noun@@B`，字符串 `" VIA SPINE"`

---

## 2. 总体思路：三层拆解“离 / 反 / 变”

从 core.dll 的调用关系可还原出一条统一思路：

- **离（平移层）**：
  - 由 **POSL** 通过 `PLNPOS` + `DBE_Pline::transformPos` 求得路径上的三维位置 `base_pos`。
- **反（旋转基准层）**：
  - 由路径方向 `path_dir`（`PLNDIR` + `transformDir`）与 **YDIR** 属性构造局部坐标系 `(X, Y, Z)`，
  - 形成 `D3_Matrix` 作为 `D3_Transform` 的旋转部分。
- **变（局部偏移 / 镜像等变异层）**：
  - 由 **DELP** 等增量位置属性，在局部坐标系下计算附加偏移，再叠加到 `D3_Transform`，
  - 同时结合镜像标志（如 LMIRR 等）对方向与偏移进行翻转。

下面用伪代码分别描述这三层逻辑。

> 说明：伪代码为 C++/伪 C 风格，仅表达数据流与几何含义，不对应 core.dll 的真实参数排布与错误处理细节。

---

## 3. 离：平移层（POSL → 位置）

### 3.1 输入与目标

- **输入**：
  - 当前几何元素 `elem : DB_Element`
  - 属性 `ATT_POSL`（位置线标识）
  - 与之关联的路径所有者 `pline_owner : DB_Element`（PLINE 或 SPINE 之类）
- **输出**：
  - 世界坐标中的基础位置 `base_pos : D3_Point`
  - 同时为下一层“反”准备路径方向的原始数据（局部方向向量）。

### 3.2 离层伪代码

```cpp
// 离层：通过 POSL 得到世界坐标下的基础位置 base_pos
bool compute_base_position(
    const DB_Element& elem,
    const DB_Attribute& ATT_POSL,
    const DB_Qualifier& qual,
    D3_Point& base_pos,       // out
    D3_Vector& path_dir_raw   // out: 路径方向（局部系）
) {
    // 1. 读取 POSL 字符串（如 "L/1000" 等）
    std::string posl;
    if (!elem.getAtt(&ATT_POSL, qual, posl)) {
        // 无 POSL，退回到普通 POS 属性（不在本文重点范围内）
        return fallback_position(elem, qual, base_pos, path_dir_raw);
    }

    // 2. 解析 POSL 语法，得到在线上的参数 p
    POSLParam p = parse_POSL_token(posl);    // 解析语法，细节在 PLNPOS 内部

    // 3. 找到拥有该位置线的元素（可能是 LINE / SPINE 等上级）
    const DB_Element* pline_owner = find_pline_owner(elem, qual);
    if (!pline_owner) return false;

    // 4. 在线的局部坐标系中求点和方向
    D3_Point  local_on_path = PLNPOS(*pline_owner, p);   // via PLNPOS
    D3_Vector local_dir     = PLNDIR(*pline_owner, p);   // via PLNDIR

    // 5. 将局部点和方向变换到世界坐标（使用 DBE_Pline::transformXXX）
    D3_Point world_pos;
    D3_Vector world_dir;
    MR_Message* msg = nullptr;    // 错误与诊断信息

    DBE_Pline::transformPos(
        *pline_owner,
        elem,
        local_on_path,
        world_pos,
        msg
    );

    DBE_Pline::transformDir(
        *pline_owner,
        elem,
        local_dir,
        world_dir,
        msg
    );

    base_pos      = world_pos;
    path_dir_raw  = world_dir;    // 为“反”层准备
    return true;
}
```

> 结论：在 **离层**，POSL 决定“元素在路径（PLINE/SPINE）上的参考点”，通过 `PLNPOS` / `transformPos` 得到 `base_pos`，为后续方位与偏移计算提供几何基点。

---

## 4. 反：旋转基准层（YDIR + 路径方向 → 旋转矩阵）

### 4.1 输入与目标

- **输入**：
  - 来自“离层”的：
    - `base_pos : D3_Point`（路径上的基点，世界坐标）
    - `path_dir_raw : D3_Vector`（路径方向向量）
  - 元素属性：
    - `ATT_YDIR`（期望的局部 Y 方向）
- **目标**：
  - 在世界坐标系下构造一套右手正交基 `(x_axis, y_axis, z_axis)`：
    - 一般取 `z_axis = normalize(path_dir_raw)`
    - 以 YDIR 为参考，通过两次叉积正交化得到 `x_axis` 与 `y_axis`
  - 将该基打包为 `D3_Matrix`，作为 `D3_Transform` 的旋转部分。

### 4.2 反层伪代码

```cpp
// 反层：由路径方向 + YDIR 构造旋转矩阵 R
bool compute_orientation_basis(
    const DB_Element& elem,
    const DB_Attribute& ATT_YDIR,
    const DB_Qualifier& qual,
    const D3_Vector& path_dir_raw,
    D3_Matrix& R                // out
) {
    D3_Vector path_dir = path_dir_raw;
    if (!path_dir.normalize_safe()) {
        // 路径方向不可用，退化处理
        return fallback_orientation(elem, qual, R);
    }

    // 1. 读取 YDIR 属性，视为“期望的局部 Y 轴方向”
    D3_Vector y_ref;
    bool has_ydir = elem.getAtt(&ATT_YDIR, qual, y_ref);

    if (!has_ydir || !y_ref.normalize_safe()) {
        // 无 YDIR 时，core.dll 会回退到其他惯例（如仅用 path_dir + 某全局轴）
        return fallback_orientation_without_ydir(path_dir, R);
    }

    // 2. 防止 YDIR 与路径方向近乎共线（dot ≈ ±1）
    if (fabs(dot(y_ref, path_dir)) > 0.99) {
        // 退化：改用某个固定参考轴作为初始 y_ref
        y_ref = choose_safe_global_axis(path_dir);
    }

    // 3. 两次叉积构造正交基
    D3_Vector ref_dir = cross(y_ref, path_dir);  // 与路径和期望 Y 都垂直
    ref_dir.normalize_safe();

    D3_Vector y_axis = cross(path_dir, ref_dir); // 真正的局部 Y
    y_axis.normalize_safe();

    D3_Vector x_axis = cross(y_axis, path_dir);  // 补齐 X
    x_axis.normalize_safe();

    // 4. 按列打包成旋转矩阵 R
    R = D3_Matrix::fromCols(x_axis, y_axis, path_dir);
    return true;
}
```

> 关键点：
>
> - **路径方向 `path_dir` 提供 Z 轴**，
> - **YDIR 提供期望 Y 的“倾向”**，通过两次叉积保证 `(X, Y, Z)` 正交且右手，
> - 退化处理确保在 YDIR 与路径方向平行时仍能构造出稳定的坐标系。

---

## 5. 变：局部偏移与镜像等变异层（DELP + 镜像标志）

### 5.1 输入与目标

- **输入**：
  - “离层”与“反层”的结果：
    - `base_pos : D3_Point`
    - `R : D3_Matrix`
  - 元素属性：
    - `ATT_DELP`（增量位置向量）
    - 各类镜像 / 取反标志（如 LMIRR 等，具体符号在其它属性常量中）
- **目标**：
  - 在 **局部坐标系** 下应用 DELP（与 LMIRR 等标志结合），
  - 将偏移转换到世界系并叠加到 `base_pos`，
  - 最终形成完整的 `D3_Transform`：
    - 旋转 = `R`
    - 位移 = `base_pos + R * local_delta`（或等价表达）。

### 5.2 变层伪代码

```cpp
// 变层：在局部坐标系中应用 DELP 和镜像等变异
bool apply_local_variations(
    const DB_Element& elem,
    const DB_Attribute& ATT_DELP,
    const DB_Qualifier& qual,
    const D3_Matrix& R,
    const D3_Point& base_pos,
    D3_Transform& T            // out
) {
    // 1. 初始化变换（旋转 + 平移基点）
    T.setRotation(R);
    T.setShift(base_pos.asVector());

    // 2. 读取 DELP：局部坐标系中的增量位置
    D3_Vector local_delta(0, 0, 0);
    bool has_delp = elem.getAtt(&ATT_DELP, qual, local_delta);

    if (has_delp) {
        // 3. 根据镜像标志（如 LMIRR 等）对局部 delta 做符号翻转
        MirrorFlags mf = read_mirror_flags(elem, qual);  // 例如 LMIRR, etc.

        if (mf.flip_x) local_delta.x = -local_delta.x;
        if (mf.flip_y) local_delta.y = -local_delta.y;
        if (mf.flip_z) local_delta.z = -local_delta.z;

        // 4. 将局部 delta 通过 R 变换到世界坐标
        D3_Vector world_delta = R * local_delta;

        // 5. 在 T 上叠加平移
        T.moveBy(world_delta);
    }

    // 其他变异：如按某些标志调整旋转基准、引入额外旋转等，
    // 也都在这一层对 T 进行微调（在 core.dll 中对应额外属性和函数调用）。

    return true;
}
```

> 直观理解：
>
> - **离层** 给了我们“站在路径上的哪个点 base_pos”；
> - **反层** 决定“人面朝哪，头朝哪（R）”；
> - **变层** 决定“人在自己局部坐标系下前后左右挪一点（DELP + 镜像）”。

---

## 6. 三层组合：从 YDIR + POSL + DELP 到 D3_Transform

### 6.1 总控伪代码（仅 core.dll 语义）

```cpp
// 高层：基于 core.dll 的三层方位计算
bool build_transform_from_yposd(
    const DB_Element& elem,
    const DB_Qualifier& qual,
    D3_Transform& T
) {
    // --- 离：求基础位置 ---
    D3_Point  base_pos;
    D3_Vector path_dir_raw;
    if (!compute_base_position(elem, ATT_POSL, qual, base_pos, path_dir_raw)) {
        return false;  // 无法定位到路径上的点
    }

    // --- 反：构造旋转基准 ---
    D3_Matrix R;
    if (!compute_orientation_basis(elem, ATT_YDIR, qual, path_dir_raw, R)) {
        return false;  // 无法得到稳定的坐标系
    }

    // --- 变：局部偏移与镜像 ---
    if (!apply_local_variations(elem, ATT_DELP, qual, R, base_pos, T)) {
        return false;
    }

    return true;
}
```

### 6.2 SPINE / “VIA SPINE” 的特殊点

- 在字符串表中可以看到：
  - `?NOUN_SPINE@@3QBVDB_Noun@@B`
  - 字符串 `" VIA SPINE"`
- 这表明：
  - SPINE 是一个独立的 noun，有自己的路径参数化；
  - 当 POSL 或其它位置语法中包含 “VIA SPINE” 时，
    - `PLNPOS` / `PLNDIR` 内部会以 SPINE 为路径来源，
    - 但在“离 / 反 / 变”三层结构上，算法流程 **与普通 PLINE 完全一致**：
      - 离：利用 SPINE 上的参数，求基点 `base_pos`；
      - 反：利用 SPINE 的切线作为 `path_dir`，加上 YDIR 构造 R；
      - 变：在该局部系下应用 DELP 与镜像。

---

## 7. 小结（供实现时对照）

从 IDA / core.dll 的角度看，YDIR、POSL、DELP 在方位计算中的职责非常清晰：

- **POSL（离）**：
  - 决定元素在 PLINE / SPINE 等路径上的“参考点”，
  - 通过 `PLNPOS` + `DBE_Pline::transformPos` 变成世界坐标 `base_pos`。

- **YDIR（反）**：
  - 不直接作为世界 Y 轴，而是与路径方向（`PLNDIR` + `transformDir`）一起，
  - 通过两次叉积构造一套稳定的局部坐标系 `(X, Y, Z)`，
  - 打包为 `D3_Matrix`，成为 `D3_Transform` 的旋转部分。

- **DELP（变）**：
  - 在局部坐标系中提供附加位移，
  - 结合镜像等标志变换符号，
  - 经由旋转矩阵 R 映射到世界，再叠加到 `base_pos`。

这三层组合起来，就形成了 core.dll 中“以路径为基准、以 YDIR 为方向、以 DELP 为微调”的完整方位计算链路。
