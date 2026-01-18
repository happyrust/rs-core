# FITT ORI/POS Hook for AVEVA E3D

通过 Frida 动态 hook AVEVA E3D 的 `core.dll`，实时捕获 FITT 元素的方位(ORI)和位置(POS)计算过程。

## 前置条件

1. **安装 Python 3.8+**
2. **安装 Frida**:
   ```bash
   pip install frida frida-tools
   ```

3. **AVEVA E3D 2.1** 安装在 `D:\AVEVA\Everything3D2.10`

## 文件说明

| 文件 | 说明 |
|------|------|
| `fitt_hook.js` | Frida hook 脚本，包含所有 hook 逻辑 |
| `run_fitt_hook.py` | Python 启动器，用于注入和管理 hook |
| `run_hook.bat` | Windows 批处理启动脚本 |

## 使用方法

### 方法 1: 附加到已运行的 E3D

1. 先启动 E3D Design 模块
2. 运行 hook:
   ```bash
   python run_fitt_hook.py --attach
   ```

### 方法 2: 使用批处理脚本

```bash
run_hook.bat
```

### 方法 3: 交互模式

```bash
python run_fitt_hook.py --attach -i
```

交互命令:
- `status` - 显示 hook 状态
- `verbose` - 启用详细日志
- `filter FITT` - 只捕获 FITT 元素
- `nofilter` - 清除过滤器
- `quit` - 退出

## Hook 的函数

| 函数 | 偏移 | 说明 |
|------|------|------|
| `DB_PseudoAttPlugger::getPosAtt` | 0x0005e930 | 获取位置属性入口 |
| `DB_PseudoAttPlugger::getOriAtt` | 0x0005e990 | 获取方位属性入口 |
| `runFortranPlugger (D3_Point)` | 0x00051cc0 | Fortran 位置计算 |
| `runFortranPlugger (D3_Matrix)` | 0x00057400 | Fortran 方位计算 |
| `GFITMD` | 0x00375798 | FITT 主计算函数 |
| `GATORI` | 0x000b07b8 | 方位获取 |
| `GATTRO` | 0x000b14aa | 变换获取 |

## 输出示例

```
[2025-01-18T11:30:00.000Z] [INFO] Found core.dll at base: 0x10000000
[2025-01-18T11:30:00.100Z] [INFO] Hooked getPosAtt at 0x1005e930
[2025-01-18T11:30:00.101Z] [INFO] Hooked getOriAtt at 0x1005e990
...

[2025-01-18T11:30:15.500Z] [INFO] [POS] Position: X=1500.000, Y=2300.000, Z=15000.000
[2025-01-18T11:30:15.502Z] [INFO] [ORI] Orientation Matrix:
[2025-01-18T11:30:15.503Z] [INFO]   X-axis: (1.000000, 0.000000, 0.000000)
[2025-01-18T11:30:15.504Z] [INFO]   Y-axis: (0.000000, 1.000000, 0.000000)
[2025-01-18T11:30:15.505Z] [INFO]   Z-axis: (0.000000, 0.000000, 1.000000)
```

## 数据结构

### D3_Point (位置)
```
struct D3_Point {
    double x;  // offset 0x00
    double y;  // offset 0x08
    double z;  // offset 0x10
};
```

### D3_Matrix (方位)
```
struct D3_Matrix {
    double m[9];  // 3x3 旋转矩阵，列优先存储
    // m[0], m[1], m[2] = 第一列 (X轴方向)
    // m[3], m[4], m[5] = 第二列 (Y轴方向)
    // m[6], m[7], m[8] = 第三列 (Z轴方向)
};
```

## 故障排除

### core.dll not found
- 确保 E3D 已完全启动（等待主界面出现）
- 脚本会自动等待 core.dll 加载

### 权限问题
- 以管理员身份运行命令提示符
- 确保 Frida 有足够权限附加到进程

### 无输出
- 尝试在 E3D 中选择/修改 FITT 元素触发属性读取
- 使用 `--verbose` 选项查看详细日志

## 扩展

可以修改 `fitt_hook.js` 添加更多 hook 点或自定义输出格式。关键扩展点:

1. **添加新 hook**: 参考 `hookGFITMD()` 函数
2. **修改输出格式**: 修改 `log()` 函数
3. **添加过滤逻辑**: 在 `onEnter` 回调中添加条件判断

## 相关文档

- `docs/方位计算/GENSEC核心函数功能描述.md` - 核心函数分析
- `.windsurf/plans/fitt_orientation_analysis_plan.md` - FITT 方位分析计划
