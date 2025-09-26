# SPINE路径可视化功能

本功能为GENSEC的SPINE路径处理增加了SVG可视化能力，帮助验证路径计算的正确性。

## 功能特性

### 1. SPINE路径长度计算增强
- 支持POINSP直线段和CURVE弧线段的混合路径
- 正确处理GENSEC→SPINE→(POINSP/CURVE)层级关系
- 累加所有路径段的总长度
- 支持THRU类型弧线的精确计算

### 2. SVG可视化
- 自动缩放和居中显示路径
- 清晰的点类型区分（POINSP蓝色，CURVE橙色）
- 路径类型区分（直线绿色，弧线粉色）
- 详细的长度和坐标标注
- 网格线和坐标轴
- 信息面板显示总长度和统计

## 文件结构

```
src/
├── rs_surreal/schemas/functions/
│   ├── spine_calc.surql          # SPINE计算函数
│   ├── test_spine_calc.surql     # 测试函数
│   └── load_spine_functions.sh   # 加载脚本
├── utils/
│   └── svg_generator.rs          # SVG生成器
└── test/
    └── test_gensec_spine.rs      # 测试用例
```

## 使用方法

### 1. 运行测试生成SVG

```bash
# 运行所有SPINE相关测试
cargo test test_gensec_spine

# 运行简单SVG生成测试
cargo test test_svg_generation_simple

# 运行复杂路径测试
cargo test test_svg_generation_complex
```

### 2. 查看生成的SVG文件

测试会在项目根目录生成以下SVG文件：
- `gensec_spine_test.svg` - gensec.txt数据的可视化
- `simple_spine_test.svg` - 简单路径测试
- `complex_spine_test.svg` - 复杂路径测试

在浏览器中打开这些SVG文件即可查看路径。

### 3. SurrealQL函数使用

```sql
-- 计算GENSEC的SPINE总长度
SELECT fn::get_gensec_spine_length(id) as spine_length
FROM pe WHERE noun = 'GENSEC';

-- 直接计算SPINE长度
SELECT fn::calc_spine_length(spine_id) as length
FROM pe WHERE noun = 'SPINE';
```

## test-files/gensec.txt数据分析

该文件包含的SPINE结构：
```
SPINE (YDIR: S)
├── POINSP: (12635, -25862, 1950)
├── POINSP: (12785, -25862, 1950)  # 150mm直线段
├── CURVE: (12884, -25903.01, 1950) RADI=140mm CURTYP=THRU
├── POINSP: (12925, -26002, 1950)  # 弧线段
├── POINSP: (12925, -26152, 1950)  # 150mm直线段
├── POINSP: (12925, -29152, 1950)  # 3000mm直线段
└── POINSP: (12925, -30652, 1950)  # 1500mm直线段
```

### 计算结果
- 段1: 150mm (直线)
- 段2: ~200mm (弧线，半径140mm)
- 段3: 150mm (直线)
- 段4: 3000mm (直线)
- 段5: 1500mm (直线)
- **总长度: ~5000mm**

## SVG可视化说明

### 颜色编码
- 🔵 蓝色圆点：POINSP点
- 🟠 橙色圆点：CURVE点
- 🟢 绿色直线：POINSP间的直线段
- 🌸 粉色曲线：包含CURVE的弧线段

### 信息显示
- 每个点显示编号和坐标
- 每段路径显示长度
- 弧线显示半径信息
- 左上角信息面板显示总统计

## 故障排除

### 1. SVG文件不显示
- 确保浏览器支持SVG
- 检查文件路径是否正确
- 查看控制台错误信息

### 2. 路径计算错误
- 检查POINSP坐标是否正确
- 验证CURVE的RADI和CURTYP属性
- 查看测试输出的详细信息

### 3. 数据库函数加载失败
```bash
# 手动加载函数
cd src/rs_surreal/schemas/functions/
./load_spine_functions.sh
```

## 扩展功能

可以进一步扩展的功能：
1. 3D路径可视化（使用Three.js）
2. 交互式路径编辑
3. 路径优化建议
4. 导出其他格式（DXF, STL等）
5. 批量处理多个GENSEC

## 技术细节

### 弧长计算公式
对于THRU类型弧线：
```
弦长 = distance(P1, P3)
圆心角 = 2 * arcsin(弦长 / (2 * 半径))
弧长 = 半径 * 圆心角
```

### SVG坐标转换
```rust
// 自动计算缩放和偏移，确保路径完整显示
let scale = min(canvas_width/data_width, canvas_height/data_height);
let svg_x = world_x * scale + offset_x;
let svg_y = canvas_height - (world_y * scale + offset_y); // Y轴翻转
```

这个可视化功能大大提高了SPINE路径处理的可调试性和可验证性。