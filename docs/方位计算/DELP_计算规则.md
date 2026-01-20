# DELP 计算规则（基于 IDA Pro 分析 core.dll）

## 1. DELP 的含义

**DELP = DELta Position**，是元素相对于其 Owner 的**局部坐标系**中的位置增量。

- `DELP` 是一个 `DVec3` 类型的属性
- 分量形式：`DELPE`（East）、`DELPN`（North）、`DELPU`（Up）

## 2. 核心变换函数

| 函数 | 地址 | 作用 |
|------|------|------|
| `TRAVEC` | `0x10691e48` | 正向仿射变换: `result = R * input + T` |
| `TRAVCI` | `0x106923a4` | 逆仿射变换: `result = R * (input - T)` |
| `INTRAM` | `0x10691d44` | 矩阵求逆: `T^(-1) = (R^(-1), -R^(-1) * t)` |
| `CSTRAM` | `0x100b1bca` | 计算从元素 A 到元素 B 的变换矩阵 |
| `GATTRO` | `0x100b14aa` | 获取元素的变换矩阵 |
| `GATRPO` | `0x100afc00` | 获取位置属性 |

## 3. TRAVEC 实现（正向变换）

```c
// 地址: 0x10691e48
int __cdecl TRAVEC(int result, int matrix, int input)
{
    MVMULT(result, matrix, input);  // result = R * input
    for (i = 1; i <= 3; ++i)
        result[i-1] += matrix[i+9-1];  // result += translation
    return result;
}
```

## 4. TRAVCI 实现（逆变换）

```c
// 地址: 0x106923a4
int __cdecl TRAVCI(double *result, double *matrix, double *input)
{
    // 先减去平移
    v4 = input[0] - matrix[9];   // X - TransX
    v5 = input[1] - matrix[10];  // Y - TransY
    v6 = input[2] - matrix[11];  // Z - TransZ
    
    // 再应用旋转矩阵
    result[0] = matrix[0]*v4 + matrix[1]*v5 + matrix[2]*v6;
    result[1] = matrix[3]*v4 + matrix[4]*v5 + matrix[5]*v6;
    result[2] = matrix[6]*v4 + matrix[7]*v5 + matrix[8]*v6;
}
```

## 5. DELP 的处理流程

```
GATRPO (获取位置属性)
  ├── DGETRA (读取属性值，如 DELP)
  ├── GATTRO (获取变换矩阵)
  │     ├── CSTRAM (计算相对变换)
  │     │     ├── GATRAR (获取元素的变换矩阵)
  │     │     ├── INTRAM (矩阵求逆)
  │     │     └── CONCAT (矩阵连接)
  │     └── ...
  └── TRAVEC (应用变换: result = R * pos + T)
```

## 6. getDefaultWRTForAtt 中的 DELP 特殊处理

```c
// 地址: 0x104a6920
// DELP 和 ZDIR 有特殊的 WRT（相对于）处理
if ( a3 != ATT_ZDIR && a3 != ATT_DELP )
    goto LABEL_22;  // 默认处理：返回 Owner

// DELP 的 WRT 默认是 Owner
```

## 7. Rust 实现（正确版本）

```rust
// 处理 DELP 和 ZDIS 属性
// DELP 是局部坐标系的偏移，需要通过 local_quat 旋转到世界坐标系
let delp = att.get_dvec3("DELP").unwrap_or(DVec3::ZERO);
let mut offset = local_quat.mul_vec3(delp);  // ✅ 正确：旋转 DELP

// ZDIS 是沿局部 Z 轴的偏移，也需要旋转
if let Some(zdis) = att.get_f64("ZDIS") {
    offset.z += zdis;  // 在旋转后的坐标系中累加 Z
}

// 最终位置 = PLINE 位置 + 旋转后的局部偏移 + 原始位置
let final_pos = plin_pos + offset + *pos;
```

## 8. 验证结果

测试用例 `17496_142306` (FITT 元件):

| 属性 | 值 |
|------|-----|
| DELP | `(-2180, 0, 0)` |
| ZDIS | `200` |
| POSL | `OBOW` |
| PLINE PLAX | `(-1, 0, 0)` |

计算结果:
- **local ORI**: `Y is Z and Z is -X` ✅
- **local POS**: `(0, 2180, 200)` ✅
- **world POS**: `(-3160, -21150, 5470)` ✅

## 9. 关键结论

1. **DELP 需要旋转**：DELP 定义在元素的局部坐标系中，需要通过 `local_quat.mul_vec3(delp)` 旋转到父节点/世界坐标系

2. **ZDIS 沿局部 Z 轴**：ZDIS 是沿局部 Z 轴的偏移，在旋转后累加

3. **错误写法**：`local_quat.inverse().mul_vec3(delp)` 是错误的

4. **正确写法**：`local_quat.mul_vec3(delp)` 是正确的
