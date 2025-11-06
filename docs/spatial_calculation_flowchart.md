# 空间计算流程图

本文档描述了使用 SQLite 进行空间计算和房间查询的完整流程。

## 1. 房间号查询主流程

```mermaid
flowchart TD
    A[开始: query_room_number_by_point] --> B[输入: 世界坐标系点 Vec3]
    B --> C[调用 query_room_panel_by_point]
    C --> D{找到房间面板?}
    D -->|否| E[返回 None]
    D -->|是| F[查询 SurrealDB]
    F --> G[SELECT room_num FROM room_panel_relate]
    G --> H[返回房间号 String]
    H --> I[结束]
    E --> I
    
    style A fill:#e1f5ff
    style I fill:#e1f5ff
    style D fill:#fff4e1
    style F fill:#e8f5e9
```

## 2. 房间面板查询详细流程（两阶段查询）

```mermaid
flowchart TD
    A[开始: query_room_panel_by_point] --> B[输入: 世界坐标系点 Vec3]
    B --> C[阶段一: SQLite 粗筛选]
    C --> D[spawn_blocking: 异步转同步]
    D --> E[调用 sqlite::query_containing_point]
    E --> F[打开 SQLite 连接]
    F --> G[执行 AABB 查询]
    G --> H{找到候选?}
    H -->|否| I[返回 None]
    H -->|是| J[提取 Refno 列表]
    J --> K[阶段二: 精确几何检测]
    K --> L[查询 SurrealDB: query_insts]
    L --> M[遍历候选面板]
    M --> N{遍历完成?}
    N -->|否| O[检查 AABB 有效性]
    O --> P{AABB 有效?}
    P -->|否| M
    P -->|是| Q[查找几何实例]
    Q --> R{找到实例?}
    R -->|否| M
    R -->|是| S[遍历实例的网格]
    S --> T[加载 .mesh 文件]
    T --> U{加载成功?}
    U -->|否| S
    U -->|是| V[应用世界变换矩阵]
    V --> W[转换为三角网格 TriMesh]
    W --> X{转换成功?}
    X -->|否| S
    X -->|是| Y[Parry3D 点包含检测]
    Y --> Z{点在网格内?}
    Z -->|是| AA[返回 RefnoEnum]
    Z -->|否| S
    N -->|是| AB[返回 None]
    AA --> AC[结束]
    I --> AC
    AB --> AC
    
    style A fill:#e1f5ff
    style AC fill:#e1f5ff
    style C fill:#fff4e1
    style K fill:#fff4e1
    style H fill:#ffebee
    style P fill:#ffebee
    style R fill:#ffebee
    style U fill:#ffebee
    style X fill:#ffebee
    style Z fill:#ffebee
    style E fill:#e8f5e9
    style L fill:#e8f5e9
```

## 3. SQLite 空间索引查询流程

```mermaid
flowchart TD
    A[开始: query_containing_point] --> B[检查配置]
    B --> C{SQLite 启用?}
    C -->|否| D[返回错误]
    C -->|是| E[获取索引文件路径]
    E --> F{文件存在?}
    F -->|否| G[返回错误]
    F -->|是| H[打开只读连接]
    H --> I[准备 SQL 查询]
    I --> J["WHERE min_x <= x <= max_x<br/>AND min_y <= y <= max_y<br/>AND min_z <= z <= max_z<br/>LIMIT 256"]
    J --> K[执行查询]
    K --> L[映射结果到 AABB]
    L --> M[返回 Vec<RefU64, Aabb>]
    M --> N[结束]
    D --> N
    G --> N
    
    style A fill:#e1f5ff
    style N fill:#e1f5ff
    style C fill:#ffebee
    style F fill:#ffebee
    style J fill:#e8f5e9
```

## 4. KNN 查询流程（K 近邻）

```mermaid
flowchart TD
    A[开始: query_knn] --> B[输入: point, k, radius, types]
    B --> C[初始化 radius = 1.0]
    C --> D[循环最多 10 次]
    D --> E[构建扩展 AABB]
    E --> F["expanded = AABB<br/>(point ± radius)"]
    F --> G[调用 query_overlap]
    G --> H[查询重叠的 AABB]
    H --> I[去重并排序]
    I --> J[计算点到 AABB 距离]
    J --> K[按距离排序]
    K --> L{结果数 >= k?}
    L -->|是| M[截取前 k 个]
    M --> N[返回结果]
    L -->|否| O[保存当前最佳结果]
    O --> P[radius *= 2.0]
    P --> Q{循环次数 < 10?}
    Q -->|是| D
    Q -->|否| R[返回最佳结果]
    R --> S[结束]
    N --> S
    
    style A fill:#e1f5ff
    style S fill:#e1f5ff
    style D fill:#fff4e1
    style L fill:#ffebee
    style Q fill:#ffebee
    style G fill:#e8f5e9
```

## 5. 重叠查询流程（query_overlap）

```mermaid
flowchart TD
    A[开始: query_overlap] --> B[输入: AABB, types, limit, exclude]
    B --> C[构建基础 SQL]
    C --> D["SELECT FROM aabb_index<br/>LEFT JOIN items"]
    D --> E["WHERE AABB 重叠条件<br/>max_x >= min_x AND min_x <= max_x<br/>..."]
    E --> F{有类型过滤?}
    F -->|是| G[添加 AND items.noun IN (...)]
    F -->|否| H{有排除列表?}
    G --> H
    H -->|是| I[添加 AND id NOT IN (...)]
    H -->|否| J{有限制数量?}
    I --> J
    J -->|是| K[添加 LIMIT]
    J -->|否| L[执行查询]
    K --> L
    L --> M[映射结果]
    M --> N[返回 Vec<RefU64, Aabb, noun>]
    N --> O[结束]
    
    style A fill:#e1f5ff
    style O fill:#e1f5ff
    style F fill:#ffebee
    style H fill:#ffebee
    style J fill:#ffebee
    style D fill:#e8f5e9
```

## 6. 系统架构图

```mermaid
graph TB
    subgraph "应用层"
        A[query_room_number_by_point]
        B[query_room_panel_by_point]
        C[query_neareast_along_axis]
    end
    
    subgraph "空间查询层"
        D[sqlite::query_containing_point]
        E[sqlite::query_knn]
        F[sqlite::query_overlap]
    end
    
    subgraph "数据存储层"
        G[(SQLite<br/>aabb_index)]
        H[(SurrealDB<br/>几何数据)]
        I[(文件系统<br/>.mesh 文件)]
    end
    
    subgraph "几何计算层"
        J[Parry3D<br/>点包含检测]
        K[三角网格<br/>TriMesh]
        L[世界变换<br/>矩阵计算]
    end
    
    A --> B
    B --> D
    B --> H
    B --> I
    B --> J
    C --> E
    E --> F
    D --> G
    F --> G
    E --> G
    J --> K
    K --> L
    L --> H
    
    style G fill:#e8f5e9
    style H fill:#e8f5e9
    style I fill:#e8f5e9
    style J fill:#fff4e1
```

## 7. 数据流图

```mermaid
sequenceDiagram
    participant Client as 客户端
    participant RoomQuery as Room Query
    participant SQLite as SQLite 索引
    participant SurrealDB as SurrealDB
    participant FileSystem as 文件系统
    participant Parry3D as Parry3D
    
    Client->>RoomQuery: query_room_number_by_point(point)
    RoomQuery->>RoomQuery: query_room_panel_by_point(point)
    
    Note over RoomQuery,SQLite: 阶段一: 粗筛选
    RoomQuery->>SQLite: spawn_blocking(query_containing_point)
    SQLite->>SQLite: 打开连接
    SQLite->>SQLite: 执行 AABB 查询
    SQLite-->>RoomQuery: 返回候选列表 (Refno, AABB)
    
    Note over RoomQuery,Parry3D: 阶段二: 精确检测
    RoomQuery->>SurrealDB: query_insts(refnos)
    SurrealDB-->>RoomQuery: 返回几何实例数据
    
    loop 遍历每个候选面板
        RoomQuery->>FileSystem: 加载 .mesh 文件
        FileSystem-->>RoomQuery: 返回网格数据
        RoomQuery->>RoomQuery: 应用世界变换矩阵
        RoomQuery->>Parry3D: contains_point(tri_mesh, point)
        Parry3D-->>RoomQuery: 返回是否包含
        alt 点在网格内
            RoomQuery-->>RoomQuery: 返回 RefnoEnum
        end
    end
    
    RoomQuery->>SurrealDB: SELECT room_num FROM room_panel_relate
    SurrealDB-->>RoomQuery: 返回房间号
    RoomQuery-->>Client: 返回房间号 String
```

## 8. 关键数据结构

```mermaid
classDiagram
    class Vec3 {
        +f32 x
        +f32 y
        +f32 z
    }
    
    class Aabb {
        +Point3 mins
        +Point3 maxs
    }
    
    class RefU64 {
        +u64 value
    }
    
    class RefnoEnum {
        +Refno(RefU64)
        +to_pe_key() String
    }
    
    class GeomInst {
        +RefnoEnum refno
        +Transform world_trans
        +Vec~Inst~ insts
    }
    
    class Inst {
        +String geo_hash
        +Transform transform
    }
    
    class PlantMesh {
        +des_mesh_file(path) Result
        +get_tri_mesh_with_flag() TriMesh
    }
    
    class TriMesh {
        +contains_point() bool
    }
    
    Vec3 --> Aabb : 用于查询
    Aabb --> RefU64 : SQLite 存储
    RefU64 --> RefnoEnum : 转换
    RefnoEnum --> GeomInst : 查询
    GeomInst --> Inst : 包含
    Inst --> PlantMesh : 加载
    PlantMesh --> TriMesh : 转换
```

## 关键性能优化点

1. **两阶段查询**：先用 AABB 粗筛选，再用精确几何检测
2. **异步转同步**：使用 `spawn_blocking` 避免阻塞异步运行时
3. **限制候选数量**：SQLite 查询限制为 256 个候选
4. **早期退出**：找到第一个匹配即返回
5. **只读连接**：SQLite 使用只读模式提高性能
6. **批量查询**：SurrealDB 批量查询几何实例

## 配置要求

- `DbOption.enable_sqlite_rtree = true`
- `DbOption.sqlite_index_path` 指向有效的 SQLite 文件
- SQLite 文件必须包含 `aabb_index` 表
- SurrealDB 必须包含 `room_panel_relate` 关系表
- 网格文件必须存在于 `assets/meshes/` 目录


