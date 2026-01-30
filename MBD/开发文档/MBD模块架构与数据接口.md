# MBD 模块架构与数据接口分析

## 模块总览
- 入口与导航：`MBD/mbd.uic` 定义工具条按钮，弹出 `form/mbd.pmlfrm`，按专业路由到各标注窗口（如 `markpipeform`、`marksuppform`、`markinstpipeform` 等）。
- 业务子模块：`markpipe/`、`markinstpipe/`、`markpipe(s)upp/`、`markinstsupp/` 负责管道/支架标注；`form/markEquiForm.pmlfrm`、`form/markCivilForm.pmlfrm`、`form/markArchForm.pmlfrm` 覆盖设备、结构、建筑。
- 基础能力：
  - 通用函数：`function/`（几何、格式、版本、导出、同步等）。
  - 数据对象：`object/`（如 `isobran`、`suisupport`、`markcivil`、`updatefiles`、`mbdinputdata` 等）封装模型读取与计算。
  - 配置模板：`files/`（图纸/校审信息、通用路径、绘制/校审设置等），`form/*AttList.txt`、`mark*/setting/*.txt` 作为界面/属性配置。
  - 数据集：`inputdata/<项目号>/*` CSV 物料库，`function/mbd/checkmbdproject.pmlfnc` 用项目号校验目录存在。
  - 平台集成：`totplant/`（表单 `totplantform.pmlfrm` + 函数 `functions/jsontotplant.pmlfnc` + 对象 `object/updatefiles/updatefiles.pmlobj`）负责向 TPlant 上传 rvm/att/json。

## 线框流程（高层）
```
[MBD 工具按钮 (mbd.uic)]
      |
      v
[主页 form/mbd.pmlfrm]
      |
      +--> [管/支/设/土建等标注窗体 mark*form]
                |
                +--> !!updatembdfiles()  // 与共享目录同步脚本/配置
                +--> !!getattlist()      // 读取 *AttList.txt 配置
                +--> AddCE/Addlist       // 采集当前模型对象
                |         |
                |         +--> 构造业务对象 (如 isobran/suisupport/markequi...)
                |                   |
                |                   +--> 从模型读取属性/几何 (object 内 getatts/split 等)
                |                   +--> 读 CSV 物料库 (getmbdmaterialdata → mbdinputdata)
                |                   +--> 读模板/配置 (files/*, form/setting/*)
                |
                +--> draw()/generatefiles()
                          |
                          +--> function/export/* 生成 rvm/att/json/txt
                          +--> totplant/jsontotplant → updatefiles(ApiRequest) 上传 TPlant
```

## 关键模块拆解
- **管道标注（`markpipe/markpipeform.pmlfrm`）**：加载属性/材料/切管/焊缝列表，实例化 `object/markpipe/isobran.pmlobj`；对象内部按 branch 拆分、收集材料/焊缝/属性，支持生成 JSON/RVM/ATT，可上传 TPlant。
- **支架标注（`markinstsupp/markInstSuppform.pmlfrm`、`markinstpipesupp/markInstPipeSuppform.pmlfrm`、`markpipe/marksuppform.pmlfrm`）**：流程与管道类似，依赖 `object/marksupport/*`、`object/markinstsupport/*` 处理支架构型与材料。
- **设备/土建/建筑标注（`form/markEquiForm.pmlfrm`、`form/markCivilForm.pmlfrm`、`form/markArchForm.pmlfrm`）**：通过相应对象 `markequi`、`markcivil`、`markarch` 采集属性并输出标注/JSON。
- **导出链路（`function/export/*`）**：`getrvm`、`getatt`、`getjson`、`gettxt` 组合生成交付文件；`getmbdfilename` 统一命名；`exportjsonarr`/`getitemjsonarray` 为 JSON 组包入口（后者目前为空壳）。
- **数据同步（`function/updatembdfiles/*`）**：从共享目录 `\\10.102.2.77\Evars\E3D\MbdProject\myprojects` 或本地缓存复制最新表单/配置，版本时间记录在 `updatembdfiles/updatetime.txt`。
- **TPlant 上传（`totplant/functions/jsontotplant.pmlfnc` + `object/updatefiles/updatefiles.pmlobj`）**：调用 `UpFileToPlatform.Interface.ApiRequest` 的 `Login/GetProject/GetAttData/GetModelServer/UploadMultiFiles/UpModelFile`，按模型类型自动生成并上传 rvm/att/json。

## 取数据接口汇总
| 接口/文件 | 作用与数据源 | 现状/依赖 | 需要我们实现/提供 |
| --- | --- | --- | --- |
| `function/inputdata/getmbdmaterialdata.pmlfnc` → `object/mbdinputdata/mbdinputdata.pmlobj` | 读取 `inputdata/<项目号>/PIPE/SUPP/BOLT/SCTN.csv` 物料库；按 `project number` 自动选目录 | 代码已实现，示例数据含 1907/1916/2016/2026/2410 | 新项目需补齐对应 CSV，`project number` 映射（如 JDY→2410）需维护 |
| `function/GetSuppData.pmlfnc` | 解析 `function/SuppMaterial.pmldat` 获取支架标准件、数量 | 已实现，依赖 pmldat 文件完整 | 更新支架库时需维护 `SuppMaterial.pmldat` |
| `function/projectinfo/getprojectinfo.pmlfnc` + `object/project/projectdata.pmlobj` | 从 PDMS/E3D 全局 `project number/name/description/message` 与节点属性 `:zd_nbbm` 组装图纸项目信息 | 运行时取自 PDMS/E3D 环境 | 确保模型存在 `:zd_nbbm` 自定义属性，缺失时需要补写或调整解析 |
| `files/getdrawinginfo*.pmlfnc`、`files/getproofreadingjson.pmlfnc` | 读取 `files/memberdrawinginfo.txt`、`files/memberproofreading.txt` 等模板生成图纸/校审 JSON | 已实现，模板文本驱动 | 模板内容需要按业务更新；缺字段会直接影响导出 |
| `function/mbd/getattlist.pmlfnc` | 读取各 `*AttList.txt` 配置（如 `markpipe/branAttlist.txt`、`form/equiAttList.txt`）驱动属性显示/导出 | 已实现 | 新增/变更属性需同步编辑对应 `*AttList.txt` |
| `function/export/getrvm/getatt/getjson/gettxt` | 基于当前模型对象导出 rvm/att/json/txt，数据来源为 PML 对象采集的模型属性与文件配置 | 逻辑齐备，依赖模型数据完整 | 无需新增接口；确保模型属性/几何可访问 |
| `function/updatembdfiles/updatembdfiles.pmlfnc` | 从共享路径或本地缓存复制最新 MBD 资源，使用 `updatetime.txt` 比较版本 | 已实现，依赖网络共享或本地路径可达 | 需要可访问 `\\10.102.2.77...` 或在 `evar pmllib` 中配置本地缓存目录 |
| `totplant/functions/jsontotplant.pmlfnc` + `object/updatefiles/updatefiles.pmlobj` | 登录 TPlant、获取项目/属性/模型服务，并上传 `UploadMultiFiles` 或 `UpModelFile`（rvm+att+json） | 代码已写死默认 `http://10.30.200.43`，调用外部 `UpFileToPlatform.Interface.ApiRequest` DLL | 需保证 DLL 与服务端 API 可用，实际项目/模型/属性键需配置；若迁移环境，需实现等价 API 或调整地址 |
| `function/export/getitemjsonarray.pmlfnc` | 预留的 JSON 聚合接口（标注“useless”且未完工） | 仅空壳，未被主要流程调用 | 如需统一聚合 JSON，需要补充实现或移除调用 |

## 发现的待补事项
- TPlant 上传链路强依赖 `UpFileToPlatform` DLL 与 `http://10.30.200.43` 服务；在新环境需确认接口文档/鉴权策略，可能需要重新实现 API 封装。
- 物料库、支架库、图纸/校审模板均为文件驱动，新增项目或字段时需及时补齐 CSV/pmldat/txt，否则界面或导出会出现空值。
- `getitemjsonarray.pmlfnc` 暂为空壳，若后续需要统一 JSON 聚合，应明确输入/输出约定后补码或删除。
