# WeGame 已安装游戏检测实现方案

日期：2026-10-29。状态：已实现；范围按用户要求限定为已安装候选。

静态进程名延期：按用户要求撤销 Unity 布局启发式、process_name 字段及 GUI 列。在找到可验证且与引擎无关的数据来源前，不实现临时识别逻辑；已安装候选检测继续保留。

依据：[实机调研记录](../../wegame-install-detection-research.md)。已验证客户端版本 6.7.0.10150；崩坏 3 覆盖安装中和完成，DeltaForce 仅覆盖安装中。以下规则是工程决策，不是腾讯承诺的协议。

## 1. 目标和范围

仅支持通过 Windows 卸载注册表发现的已安装候选：定位目录，以 Rail 身份、构建元数据和内容存在检查筛选游戏。提供客户端定位、已安装枚举和按 ID 查询，接入 Rust、C FFI、Tauri GUI、Node。

首版覆盖具有卸载项和 Rail 元数据的游戏。检测离线、只读，不要求客户端运行或管理员权限。客户端缺失不阻止扫描仍有安装证据的游戏。

不枚举安装任务、不扫描 downloading、不推导或配置库根目录、不扫描全盘。无卸载项游戏、手动导入游戏、传统腾讯网游和历史 TGP 版本不在首版覆盖保证内。

不新增专属扫描报告、状态枚举、任务展示或库目录配置。不实现安装、启动、卸载、进程管理、更新检查和下载进度，也不解析客户端二进制数据库或下载缓存。

“已安装”表示检测到落盘身份、构建和内容，不保证文件完整性、授权有效或立即可玩；更新过程的精确状态尚未验证。

## 2. 对外 Interface

```rust
pub fn executable() -> Result<PathBuf>;
pub fn games() -> Result<Vec<Game>>;
pub fn find(id: &str) -> Result<Game>;
```

- games 枚举卸载项并校验，正常扫描无结果返回空集合，不先要求找到客户端。
- find 使用同一枚举和去重规则；ID 要求规范的正十进制字符串，非法返回 InvalidGame，未找到返回 GameNotFound。未找到不证明机器上绝无该游戏。
- executable 独立定位客户端，未找到返回 LauncherNotFound。
- 非 Windows 返回已有 LauncherNotFound，并说明不支持该平台。
- 保持 Game / GameState 结构；不增加 scan(options)、ScanOptions、ScanReport 或专属 FFI/Tauri 命令。

## 3. 客户端与候选目录发现

### 注册表

用现有 winreg 读取 HKCU/HKLM 的 Software\Microsoft\Windows\CurrentVersion\Uninstall，显式处理 32/64 位视图。通过 KEY_WOW64_32KEY/64KEY 选视图，不同时硬编码 WOW6432Node。键不存在正常跳过，重复视图结果去重。

客户端候选依次来自用户及机器 Software\Classes\wegame\shell\open\command、WeGame 卸载项的 DisplayIcon、卸载程序旁的 wegame.exe。要求候选为实际存在的普通文件；协议仅用于定位，不推导启动参数。

每条卸载项按顺序产生并校验候选目录：

1. 非空且有效的绝对 InstallLocation。
2. UninstallString 中直接可执行文件的父目录。
3. DisplayIcon 文件的父目录，剥离可选图标索引。

本机崩坏 3 没有 InstallLocation，需从 `F:\WeGameApps\rail_apps\崩坏3(2001481)\崩坏3卸载.exe` 反推目录。必须通过 Rail 身份校验，不能按游戏名称、发布商或路径含 WeGame 就接受。

### 路径解析

- 展开 REG_EXPAND_SZ 环境变量，保留 UTF-16 路径到 PathBuf，不按空格拆命令。
- 引号命令提取首个被引用的 EXE；未引用且含空格时枚举 .exe 终点，要求唯一、实际存在的绝对普通文件。歧义记录日志并跳过该来源。
- msiexec、rundll32、cmd、PowerShell 等公共宿主的目录不能当游戏目录，仍可尝试同项 InstallLocation。
- 不执行卸载命令，不把命令写入 Game.commands。
- 首版跳过 UNC 网络来源，避免枚举触发网络遍历；缺失目录属于残留候选。
- 不假定客户端和游戏同盘，不从目录名读取游戏 ID，不要求目录名必须是 rail_apps。

## 4. 已安装校验

### 元数据

读取候选目录下的：

```text
rail_files/rail_game_identify.json
rail_files/rail_game_build_info.dat
```

身份 JSON 要求 game_id 为正整数 u64，输出为十进制字符串，避免 JS Number 精度问题。构建 .dat 实为 JSON，要求 repo_build_id 为非空字符串，version_name 和 version_description 为可选字符串。版本只用于内部解析和测试，不扩展 Game。

忽略未知字段，允许 UTF-8 BOM，每个文件最多 64 KiB。首版不默默接受尚未观察到的字符串类型 game_id。读取前后比较文件大小和修改时间；变化或短暂解析失败最多立即重读一次，仍不稳定或非法则跳过并记录错误。这不构成事务快照。

### 内容存在

排除 rail_files、tiny_cache 和已知卸载程序，在目标目录寻找至少一个非零普通文件。最多检查两层、256 项，不跟随子目录 reparse point/junction。权限错误或达到上限仍不能判断时，保守跳过并记录原因。

该检查仅排除只剩元数据或卸载器的空壳，不证明文件完整。不硬编码 BH3.exe 或具体游戏白名单，也不因存在 EXE 就接受。

### 局部未完成标记

仅对注册表定位的候选检查 `rail_files/download_cache.dat`：存在（包括零字节）或无法确认是否存在时，保守跳过。

这是已安装候选的排除条件，不是安装任务发现；不访问 downloading，不输出下载状态。更新或残留标记可能造成暂时漏检，需在文档说明。该规则也不能排除所有未知更新情形。

### 最终接受条件

目录存在、身份与构建有效、有实际内容、目标侧无 download_cache.dat，且读取没有未解决错误，才生成 Game。缺失、残留和不确定候选均不返回，不强制构造成 installed=false。

## 5. 去重与 Game 映射

尽量通过 canonicalize 合并相同实际路径，保留原始展示路径。不只靠 Unicode 字符串转小写处理别名；规范化失败不强行合并。

同 ID 多个有效目录按规范化路径稳定排序选择一个，并记录多安装日志；find 使用同样规则。同路径重复项的名称优先 HKCU、其次 HKLM，再按注册表键路径排序。结果按 ID 稳定排序。

```text
_type = "wegame"
id = 身份 JSON 的 game_id 十进制字符串
name = 对应卸载项 DisplayName > 目录名 > "WeGame <id>"
path = 实际目标目录
match_identity = None
commands.install/launch/uninstall = None
state.installed = true
state.downloading/needs_update = false
state.total_bytes/received_bytes = None
```

两个 false 延续旧布尔契约，表示不提供对应检测能力，不是确定没有下载或无需更新。GUI 对这两项展示“不支持检测”或隐藏，文档解释其语义。

能力矩阵只开放 List / Find / Executable。现有默认 Launch 分支必须显式限制 WeGame；manager 的通用进程路径匹配不作为已验证的 WeGame 能力开放。

## 6. 实现组织与错误处理

```text
src/wegame/
  mod.rs       # 三方法、候选筛选、去重、Game 构建
  manifest.rs  # JSON 解析与内容校验
  windows.rs   # 注册表、客户端定位、命令路径解析
```

不引入扫描器框架或公开报告类型。注册表 adapter 输出私有 UninstallEntry 列表，内部扫描函数接受列表，测试使用假注册表输入和临时目录。纯解析及候选校验可在非 Windows 测试，WinReg 代码 cfg 隔离，优先使用现有依赖。

单个损坏候选通过现有 print_error 记录后继续，不能阻断有效游戏。普通软件没有 Rail 身份文件正常跳过，不制造无关错误日志。来源键不存在正常跳过；所有可用来源枚举均失败则返回 WinReg，部分成功可返回有效集合并记录失败来源。接口是尽力枚举，不承诺扫描覆盖完整。

不缓存结果，不添加后台轮询。Tauri 沿用已有阻塞任务封装；若没有，则在调用处使用 spawn_blocking，避免阻塞 UI。

## 7. 跨层修改

| 位置 | 修改 |
| --- | --- |
| src/lib.rs | 导出 wegame |
| src/prelude.rs | GameType::WeGame 和 wegame 双向映射 |
| FFI src/lib.rs | parse_launcher/gs_list/gs_find/gs_executable 加分支 |
| FFI 头文件 | 更新平台说明，函数签名和释放契约不变 |
| Tauri commands.rs | 三处通用分发增加 WeGame，无专属命令 |
| capability.rs | 仅三项能力，更新数量和否定测试 |
| GUI types.ts | 平台类型、列表、标签增加 WeGame |
| GUI 列表/详情 | 复用已安装展示，更新和下载项注明不支持 |
| Node types/wegame.rs、types/mod.rs、lib.rs | 导出三方法，错误转 JS exception，避免新增 unwrap panic |
| node/lib/index.d.ts | 增加三方法声明，无报告类型 |
| README.md、docs/wegame.md | 范围、发现方式、状态语义、能力矩阵 |
| examples/、tests/ | 枚举示例和行为测试 |

保持 FFI ABI 和 Game JSON；新增 Rust 枚举成员会影响下游穷尽匹配，发布说明须注明。Node 不在根 workspace members 中，需独立验证。

## 8. 测试与验收

用脱敏 JSON、私有注册表输入和临时目录测试，不提交真实游戏程序、大缓存或账号数据：

1. 有效卸载项和完成布局返回正确 ID、名称、路径及 installed=true。
2. 无 InstallLocation 时从中文、空格路径正确发现游戏。
3. 引号、未引用命令、图标索引、环境变量、公共卸载宿主、歧义解析。
4. 只有目录、EXE、卸载器、元数据或无卸载项的磁盘游戏不误报。
5. 模拟提前写入卸载项但 download_cache.dat 仍存在，必须排除。
6. JSON 损坏、超限、并发变化、权限失败和内容检查截断不会误报，也不中断其他候选。
7. 重复视图、路径和 ID 的稳定选择；目录数字不同于元数据时 ID 仍来自 JSON。
8. FFI/Node 字符串 ID、错误转换、GUI 三项能力及非 Windows 编译。

实机崩坏 3 应返回 2001481 和实际路径。DeltaForce 下载期间不进入列表；完成后确认卸载项和元数据再验证，不能提前声称完成布局已验证。无卸载项属于首版不覆盖范围。

残留和删除用 fixture 模拟，不改真实游戏。任务识别、暂停恢复和进度不属于验收项。

验证结果：已安装候选实现的 cargo test --workspace 共 64 个测试通过，GUI 类型检查和生产构建通过；Node 和 C FFI 已做本机调用验证。静态进程名实验的测试与字段已撤销，不作为交付能力。当前仅验证 Windows，没有执行 Linux/macOS 交叉编译。

本机枚举和 ID 查询识别到崩坏 3（2001481），DeltaForce（2001918）仍有下载标记且未进入列表。Node 原有 manifest 缺少独立 workspace 声明，已补充；其锁文件随现有核心依赖重新解析。Rust 构建仍有原有 Riot 类型警告，部分构建出现 Windows 增量缓存目录访问警告，但构建和测试成功。

## 9. 开发顺序与完成标准

1. 元数据解析、内容校验和局部排除标记，完成 fixture 测试。
2. 注册表候选、客户端发现、路径解析、去重，实机验证崩坏 3。
3. 接入 Rust/FFI/Node/Tauri 三方法，限制 GUI 能力，保持 Game 契约。
4. 完成回归、平台文档和已安装实机样本验收。

完成标准：各调用层一致返回已验证的安装候选，不因单纯目录或 EXE 存在误报；没有新增安装任务发现、专属状态报告或库目录配置。
