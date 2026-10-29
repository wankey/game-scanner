# WeGame 游戏安装状态检测可行性

调研日期：2026-10-29。

后续设计：[WeGame 安装状态检测实现方案](superpowers/specs/2026-10-29-wegame-support-design.md)。设计中的状态规则与接口属于待实现决策，不应与下文实机事实混为一谈。

当前实施范围：按用户确认，仅实现通过卸载项发现的已安装候选。本文保留下载阶段的历史观察和探索方向，供排除误报参考；任务发现、库目录配置和专属扫描报告不属于本次实现范围，具体以设计文档为准。

## 结论

已在本机崩坏 3 样本验证一条 Windows 本地检测路线：枚举卸载项 → 从卸载程序路径定位游戏目录 → 读取 Rail 身份与构建 JSON → 校验游戏文件。它不依赖解析客户端的二进制数据库。当前证据覆盖一款 Rail 游戏的安装中与安装完成状态；其他游戏、更新中、卸载残留和非 Rail 游戏仍需验证。

建议先实现范围明确的 Rail 游戏检测，再补充其他游戏及状态样本；不能把单款游戏的目录布局直接当作 WeGame 全平台协议。

## 已确认的信息和证据范围

- WeGame 的第一方入口是 [WeGame 官网](https://www.wegame.com.cn/)；本次成功访问。官网本身不能证明客户端本地数据格式，也不能证明存在公开的本机安装状态 API。
- Windows 为安装程序提供卸载注册表信息，包括 `DisplayName`、`InstallLocation` 等字段，见 [Microsoft：Uninstall Registry Key](https://learn.microsoft.com/en-us/windows/win32/msi/uninstall-registry-key)。这份文档讨论 Windows Installer，不能据此断言 WeGame 或它的每款游戏都会写这些值。
- 64 位 Windows 存在不同注册表视图，见 [Microsoft：Registry Redirector](https://learn.microsoft.com/en-us/windows/win32/winprog64/registry-redirector)。客户端发现阶段需要考虑用户/机器作用域和适用的注册表视图。
- 首次检查时尚未发现客户端；用户随后安装了 WeGame，以下为安装后的只读实机证据。

## 安装后的实机证据

- 客户端：`F:\Program Files (x86)\WeGame\wegame.exe`，文件版本与产品版本均为 `6.7.0.10150`。
- 卸载项：`HKLM\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\WeGame`。`DisplayName=WeGame`，`DisplayIcon` 指向上述 EXE；未发现 `InstallLocation` 和 `DisplayVersion`，不能依赖它们定位客户端。
- 协议入口：`HKLM\Software\Classes\wegame\shell\open\command` 的默认值为 `"F:\Program Files (x86)\WeGame\wegame.exe" "%1"`。可作为客户端定位的补充，但尚未验证游戏启动参数。
- 候选文件：安装目录与 `%APPDATA%\Tencent\WeGame` 下均存在 `config\game_library.info`；读取内容为二进制，尚未确认编码、压缩或加密方式及其语义。
- 安装目录的 `data\json_db` 下存在 `base_game_oss.local`、`public_game_oss.local` 和用户子目录中的 `private_game_oss.local`。前两个文件可通过允许共享读写的只读文件句柄读取头部，头部并非 SQLite 标准文件头；不能据文件名判断其格式或把它们认定为已安装游戏清单。
- 以上证据来自本机文件和注册表；未修改客户端数据，未读取登录凭据。仍需要实际游戏样本与安装状态变化来确定记录来源。

## 实机状态对照与推荐路线

### 崩坏 3 下载阶段样本

用户确认崩坏 3 正在安装时，只读观察到：

- `F:\WeGameApps\downloading\2001481` 中存在 `BH3_Data`、`TCLS` 等目录；一次采样包含 50 个文件，总逻辑长度 4,203,039,270 字节。这不是客户端下载进度，可能涉及预分配，不能映射为 `received_bytes`。
- 目标目录 `F:\WeGameApps\rail_apps\崩坏3(2001481)` 已经存在，包含 `rail_files\download_cache.dat`（当次大小为 0）与 `tiny_cache\Game_2001481.local`（当次大小为 6,376,448 字节）。后者头部并非 SQLite 标准头；未解析其字段。
- 目录名共同出现 `2001481`，且下载目录有 `BH3_Data`，构成当前样本的游戏标识关联证据；尚未证明所有游戏遵循相同布局。
- **目标目录在安装完成前就存在**，因此不能把 `rail_apps` 下目录存在直接映射为 `installed=true`。
- `downloading` 中存在目录是下载任务候选证据，尚不能区分下载中、暂停、失败或残留。需要安装完成后的对照，确认文件迁移与完成标记。

### 崩坏 3 安装完成样本

用户确认安装完成后，同机只读检查结果：

| 信号 | 安装中 | 安装完成 |
| --- | --- | --- |
| `downloading\2001481` | 存在，有游戏数据 | 已不存在 |
| 目标目录 | 存在，包含缓存 | 存在，包含 `BH3.exe`、`BH3_Data`、`TCLS` 等 |
| `rail_files\download_cache.dat` | 存在 | 已不存在 |
| `rail_files\rail_game_identify.json` | 当次未发现 | 存在，可解析 JSON |
| `rail_files\rail_game_build_info.dat` | 当次未发现 | 存在，可解析 JSON |
| 游戏卸载项 | 当次按显示名查询未发现 | 存在 |

身份文件内容：

```json
{"game_id":2001481}
```

构建文件内容（虽然扩展名是 `.dat`，实为 JSON）：

```json
{"repo_build_id":"222817","version_description":"","version_name":"V9.1.0"}
```

新增卸载项 `HKLM\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\崩坏3`：`DisplayName=崩坏3`，`UninstallString=F:\WeGameApps\rail_apps\崩坏3(2001481)\崩坏3卸载.exe`；`InstallLocation`、`DisplayIcon` 未提供。只依赖 `InstallLocation` 会漏检本样本。

已执行一次不预设游戏名、ID或库目录的只读扫描：遍历用户与机器卸载项（含 32 位视图），提取卸载程序所在目录，再查找并解析上述 Rail JSON。成功输出崩坏 3、ID `2001481`、实际路径、构建 `222817`、版本 `V9.1.0`；另确认卸载程序与 `BH3.exe` 实际存在。扫描未执行任何卸载命令。

### 第二款游戏：DeltaForce 下载阶段

2026-10-29 14:14（Asia/Shanghai），用户启动第二款游戏下载后，只读观察到：

- 暂存目录 `F:\WeGameApps\downloading\2001918` 存在，其下已有 `DeltaForce` 子目录和 `DeltaForceClient.exe`（当次大小 247,336 字节）。**下载阶段也可能出现 EXE，不能据此判断安装完成。**
- 目标目录 `F:\WeGameApps\rail_apps\DeltaForce(2001918)` 已存在，`rail_files` 中当次只有零字节 `download_cache.dat`；没有 `rail_game_identify.json` 或 `rail_game_build_info.dat`。
- 用户级、机器级及机器 32 位卸载项中，按显示名 `三角洲|Delta|2001918` 和卸载命令 `DeltaForce|2001918` 检索，未发现对应项；按 `WeGameApps` 路径筛选仍仅发现已安装的崩坏 3。这是当次检索结果，不是对所有可能注册方式的证明。
- `tiny_cache` 下存在 `.wgj`、多份 `.manifest`、`Game_2001918.dat` 与 `Game_2001918.db`。对后两者读取头部，`.db` 并非 SQLite 标准文件头，尚未解析格式；不能因扩展名直接交给普通 SQLite 读取。

第二个样本支持两条分开的发现路线：

1. **已安装候选**：卸载项定位目录，再读取 Rail 身份/构建信息并校验文件。
2. **安装任务候选**：已知游戏库根目录下，对照 `downloading\<id>` 与 `rail_apps\<name>(<id>)\rail_files\download_cache.dat`。当前 ID 来自目录名，尚无已验证元数据确认其语义；此路线需要可靠的库根目录来源，当前实机路径不能硬编码为通用规则。

两款游戏的下载阶段都未出现已安装身份/构建文件，有助于避免首次安装误报；但安装任务候选仍不能区分传输中、暂停或失败，更新旧游戏时这些标记的生命周期也未验证。DeltaForce 安装完成后的对照尚待采集。

### 首版范围与限制

- 首版：卸载项定位候选目录，Rail JSON 确认游戏 ID 与已落盘构建，校验目录和文件。卸载项缺失的游戏可由调用方指定库根目录补充发现。
- `BH3.exe` 仅为本样本的文件校验项；通用实现仍需可靠的入口信息或可验证文件列表，不能硬编码所有游戏主程序，也不能把 JSON 存在当成文件完整性保证。
- `installed` 可采用上述多信号作为已安装的检测规则，但需要更新、手动删文件与卸载残留样本检验误报。首次安装完成后仍可能有游戏内资源下载，本次未启动游戏验证可玩性。
- `downloading` 目录的有无不能单独表示正在传输；不能据本次观察实现精确下载进度或更新状态。
- 暂不声称覆盖传统腾讯网游、导入游戏和所有 WeGame 游戏。已确认的范围是当前客户端版本下的崩坏 3 Rail 安装。

### 后续扩展方向

1. **发现客户端**：注册表/卸载项作为候选入口，也接受调用方提供的客户端根目录。硬编码常见安装目录只能作为补充。
2. **解析本地记录**：实机定位客户端保存的游戏 ID、安装路径与状态。优先结构化文件、配置或数据库；不要先假设存在某个 JSON/manifest 文件。若数据库会被客户端锁定，评估只读快照方式。
3. **校验文件**：存在元数据只表示发现了记录。结合有效安装目录与游戏入口或完成标记，避免把已卸载的残留记录、空目录和未完成下载判为已安装。
4. **明确状态边界**：当前可优先支持 `Installed` 的可靠判定。只有确认可靠的下载元数据后才映射 `Downloading`；无法分辨时不要猜测。未发现安装记录也不等于证明用户从未拥有或从未安装过游戏。

注册表中的独立游戏卸载项可作为补充信号，但不足以证明游戏归属 WeGame。全盘搜 EXE 和进程检测也无法可靠识别已安装但未运行的游戏及平台来源，不宜作为核心方案。

## 最小实机验证清单

### 静态进程名：适用性修正

已撤销的 `process_name` 实验根据 Unity 的 `Foo.exe + Foo_Data + UnityPlayer.dll` 布局推断，属于局部启发式，不是通用 WeGame 入口解析。按用户要求已删除该识别逻辑、字段和展示，在找到可验证方案前不提供静态进程名。崩坏 3 返回 BH3.exe 仅验证当前样本，不能推出所有 Unity 游戏或非 Unity 游戏都可识别；单元测试也只证明该规则按预期执行。

用户指出跨引擎覆盖问题后，重新只读检查了 DeltaForce 的 `tiny_cache\Game_2001918.dat`。当次文件中可提取多个 EXE 路径，包括 `DeltaForceClient.exe`、`DeltaForce\Binaries\Win64\DeltaForceClient-Win64-Shipping.exe`、反作弊程序及 WebView 程序。这里只确认了文件里有这些字符串，尚未证明其字段语义、文件格式或哪一项是启动入口/实际游戏进程。

因此不能用“提取文件中的 EXE 字符串”、唯一 EXE、目录名称或引擎特征替代平台启动配置。后续通用路线应定位并验证 WeGame 为每款游戏保存的启动入口字段，同时区分启动器、实际游戏程序及多进程游戏；启动入口文件名不一定等于最终运行的游戏进程名。找不到权威字段时应返回未知，不能声称已实现全平台静态进程名检测。当前已安装候选检测与此问题独立，不受影响。

- 同一款游戏：未安装、下载中、暂停、安装完成、客户端卸载游戏。
- 覆盖自定义盘符与路径、中文路径、客户端退出/运行两种情况。
- 额外检查手动删除游戏目录、移动目录或卸载后残留记录。
- 保留脱敏样本及客户端版本，作为解析兼容性与状态判定测试资料。

在完成采样前，`TGP` 仅作为潜在旧客户端名称用于探索，不视为已验证兼容要求。本次未核实官方 SDK、稳定公开接口或本地数据库协议，不能声称这些接口存在，也不能声称不存在。
