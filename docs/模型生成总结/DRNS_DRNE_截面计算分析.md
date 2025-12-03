# DRNS/DRNE 截面计算分析

> 基于 AVEVA E3D core.dll 逆向分析

## 概述

DRNS (Draft Start) 和 DRNE (Draft End) 是 PDMS/E3D 中用于控制管件端面斜切角度的属性。这两个属性决定了管件或原语在起始端和结束端的截面倾斜角度。

## 1. 属性定义

| 属性 | 全局变量地址 | 字符串地址 | 用途 |
|------|-------------|-----------|------|
| **ATT_DRNS** | `0x10f61c50` | `0x10b3b93f` | 起始端斜切角 (Draft Start) |
| **ATT_DRNE** | `0x10f62444` | `0x10b3b920` | 结束端斜切角 (Draft End) |

## 2. 属性初始化

DRNS 和 DRNE 作为 `DB_Attribute` 类型的全局常量，在程序启动时初始化：

```cpp
// DRNS 属性初始化 (地址: 0x1095CBD0)
void init_ATT_DRNS() {
    char* attr = operator new(0x12C);  // 分配 300 字节
    string name;
    string_assign(&name, "DRNS", 4);   // 设置属性名
    ATT_DRNS = DB_Attribute::DB_Attribute(attr, name);
}

// DRNE 属性初始化 (地址: 0x1095CB00)
void init_ATT_DRNE() {
    char* attr = operator new(0x12C);
    string name;
    string_assign(&name, "DRNE", 4);
    ATT_DRNE = DB_Attribute::DB_Attribute(attr, name);
}
```

## 3. 属性值获取

DRNS/DRNE 存储为 double 类型的角度值，通过 `DB_Element::getDouble()` 获取：

```cpp
// 地址: 0x104b7450
double DB_Element::getDouble(const DB_Attribute* attr, int qualifier) {
    double result = 0.0;
    
    if (qualifier) {
        // 带限定符的获取
        DB_Qualifier qual;
        DB_Qualifier::setFortranIntQualifier(&qual, qualifier);
        DB_Element::getAtt(this, attr, &qual, &result);
    } else {
        // 使用默认限定符
        DB_Element::getAtt(this, attr, DB_Element::defaultQualifier_, &result);
    }
    
    return result;
}
```

### 使用示例

```cpp
// 获取元素的 DRNS 和 DRNE 值
double drns = element.getDouble(ATT_DRNS, 0);  // 起始端斜切角
double drne = element.getDouble(ATT_DRNE, 0);  // 结束端斜切角
```

## 4. 角度处理

### 4.1 mthAngle 类

E3D 使用 `mthAngle` 类来处理角度值，支持弧度和度数之间的转换：

```cpp
class mthAngle {
public:
    enum units { RADIANS, DEGREES };
    
    mthAngle();                              // 默认构造
    mthAngle(double value, units u);         // 指定单位构造
    mthAngle(const D2_Vector& v);            // 从2D向量构造
    
    double getAngle(units u) const;          // 获取指定单位的角度值
    void makePrincipal();                    // 规范化到主值区间
    
    // 运算符重载
    mthAngle operator+(const mthAngle& other) const;
    mthAngle operator-(const mthAngle& other) const;
    mthAngle operator*(double scalar) const;
    bool operator<(const mthAngle& other) const;
    // ...
};

// 规范化角度到主值区间
mthAngle mthPrincipalAngle(const mthAngle& angle);
```

### 4.2 欧拉角计算

```cpp
// 地址: 0x104a0450
// 从旋转矩阵计算欧拉角
static void DB_Element::calculateEulerAngles(
    const D3_Matrix& matrix,
    vector<double>& angles
);

// 地址: 0x104a6e60
// 从欧拉角构造旋转矩阵
static void DB_Element::angleToMatrix(
    const vector<double>& angles,
    D3_Matrix& matrix
);
```

## 5. 变换矩阵

### 5.1 D3_Matrix (3x3 旋转矩阵)

```cpp
class D3_Matrix {
public:
    D3_Matrix();
    D3_Matrix(double scalar);                // 标量矩阵
    D3_Matrix(const vector<double>& arr);    // 从数组构造
    
    double get(int row, int col) const;
    void set(int row, int col, double value);
    
    void beIdentity();                       // 设为单位矩阵
    void beZero();                           // 设为零矩阵
    bool isIdentity() const;
    bool isSingular() const;
    
    void asArray9(vector<double>& arr) const;
    static bool arrayIsIdentity(const vector<double>& arr);
};
```

### 5.2 D3_Transform (完整变换)

```cpp
class D3_Transform {
public:
    D3_Transform();
    D3_Transform(const D3_Matrix& rotation, const D3_Vector& translation);
    
    void setRotation(const D3_Matrix& matrix);  // 设置旋转部分
    void setShift(const D3_Vector& vector);     // 设置平移部分
    void beIdentity();
    void moveBy(double x, double y, double z);
    void moveBy(const D3_Vector& offset);
    
    bool isIdentity() const;
    bool isOrthogonal(double tolerance) const;
    double determinant() const;
    bool getInverse(D3_Transform& inverse) const;
    
    D3_Transform operator*(const D3_Transform& other) const;
    D3_Point operator*(const D3_Point& point) const;
    D3_Vector operator*(const D3_Vector& vector) const;
};
```

## 6. 截面几何处理

### 6.1 G2L 布尔运算引擎

```cpp
class G2L_BooleanEngine {
public:
    enum G2L_Op {
        UNION = 1,        // 并集
        INTERSECTION = 2, // 交集
        DIFFERENCE = 3,   // 差集
        XOR = 4           // 对称差
    };
    
    enum G2L_Status { /* 状态码 */ };
    
    static G2L_BooleanEngine& instance();
    
    G2L_Status newObject();
    G2L_Status newKcurve();
    G2L_Status addSpan(double x, double y, double bulge);
    G2L_Status endKcurve();
    G2L_Status endObject();
    
    G2L_Status doOperation(G2L_Op op);
    
    G2L_Status outputObject(int index, int& kcurveCount);
    G2L_Status outputKcurve(int& spanCount);
    G2L_Status outputSpan(double& x, double& y, double& bulge);
    
    void reset();
    void setTrace(int level);
    string queryVersion();
    string errorMessage(const G2L_Status& status);
};
```

### 6.2 Set2d 系列函数

主要的 2D 集合操作函数：

| 函数 | 地址 | 功能 |
|------|------|------|
| `Set2d_core_MainPipeline` | `0x1078bcee` | 主处理管线 |
| `Set2d_CreateLoopsFromEdges` | `0x1078d4e8` | 从边创建循环 |
| `Set2d_ResolveIntersection` | `0x107a0a12` | 解析交点 |
| `Set2d_KnittingStage` | `0x10792926` | 边缝合阶段 |
| `Set2d_LoopTidyStage` | `0x10797228` | 循环整理阶段 |
| `Set2d_CompactEdges` | `0x10786928` | 边压缩 |
| `Set2d_CompactLoops` | `0x1078824e` | 循环压缩 |

## 7. 典型处理流程

```text
┌─────────────────────────────────────────────────────────────┐
│                    DRNS/DRNE 截面处理流程                     │
└─────────────────────────────────────────────────────────────┘

1. 属性读取
   ├── element.getDouble(ATT_DRNS, 0)  → drns_angle
   └── element.getDouble(ATT_DRNE, 0)  → drne_angle

2. 角度转换
   ├── mthAngle(drns_angle, DEGREES)   → start_angle
   └── mthAngle(drne_angle, DEGREES)   → end_angle

3. 构建旋转矩阵
   ├── start_angle → D3_Matrix (起始端旋转)
   └── end_angle   → D3_Matrix (结束端旋转)

4. 应用到变换
   ├── D3_Transform::setRotation(start_matrix)
   └── D3_Transform::setRotation(end_matrix)

5. 截面轮廓变换
   └── 将标准截面按 DRNS/DRNE 角度倾斜

6. 2D 布尔运算
   ├── G2L_BooleanEngine::newObject()
   ├── G2L_BooleanEngine::addSpan(...)  × N
   ├── G2L_BooleanEngine::doOperation(op)
   └── G2L_BooleanEngine::outputObject(...)
```

## 8. 相关属性

与截面定义相关的其他属性：

| 属性 | 说明 |
|------|------|
| **PAAX** | P-Axis A (截面 A 轴方向) |
| **PBAX** | P-Axis B (截面 B 轴方向) |
| **HEIG** | 高度 |
| **DIAM** | 直径 |
| **GENSEC** | 通用截面定义 |

## 9. 在 rs-core 中的应用

对于 rs-core 的几何生成，DRNS/DRNE 的处理逻辑应该是：

```rust
/// 计算斜切端面的变换矩阵
fn calculate_draft_transform(draft_angle: f64, axis: Vec3) -> Mat4 {
    // draft_angle: DRNS 或 DRNE 的值（度数）
    // axis: 截面的法向量方向
    
    let angle_rad = draft_angle.to_radians();
    
    // 绕垂直于轴向的方向旋转
    let rotation = Quat::from_axis_angle(
        axis.cross(Vec3::Y).normalize(),  // 旋转轴
        angle_rad
    );
    
    Mat4::from_quat(rotation)
}

/// 应用 DRNS/DRNE 到圆柱体截面
fn apply_draft_to_cylinder(
    cylinder: &LCylinder,
    drns: f64,  // 起始端斜切角
    drne: f64,  // 结束端斜切角
) -> (Plane, Plane) {
    let axis = cylinder.axis_direction();
    
    // 起始端截面
    let start_plane = Plane::new(
        cylinder.start_point(),
        apply_draft_rotation(axis, drns)
    );
    
    // 结束端截面
    let end_plane = Plane::new(
        cylinder.end_point(),
        apply_draft_rotation(-axis, drne)
    );
    
    (start_plane, end_plane)
}
```

## 10. 参考

- AVEVA E3D Reference Manual - Primitive Attributes
- PDMS Design Reference Manual - Draft Angles
- core.dll 逆向分析 (MD5: 099b9237a64002e46b918c18841f547a)
