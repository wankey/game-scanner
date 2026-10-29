# 米哈游启动器 / HoYoPlay

Windows 国服统一启动器的只读、离线安装及下载状态检测。当前目录仅识别 `nap_cn`（绝区零）与 `hkrpg_cn`（崩坏：星穹铁道），channel/sub_channel 均为 1；不承诺国际服、其他渠道或旧版独立启动器。

```rust
let games = game_scanner::hoyoplay::games()?;
let game = game_scanner::hoyoplay::find("nap_cn")?;
let launcher = game_scanner::hoyoplay::executable()?;
```

Node 使用 `scanner.hoyoplay.games()/find(id)/executable()`；C FFI 继续使用现有函数，launcher 参数为 `hoyoplay`。GUI 支持 List、Find、Executable。

## 检测规则

从当前用户 `Software\miHoYo\HYP\1_1\<biz>` 获取匹配的 GameBiz 和实际 GameInstallPath，核验 config.ini 中非空 game_version、匹配渠道，以及官方配置指定的非空启动文件。读取后重新核验注册信息和配置。32/64 位视图去重，Game.id 使用 GameBiz，Game.path 为游戏目录。重复有效路径按稳定顺序处理。

只读取登记目录，不扫描磁盘或执行游戏/启动器。不要求客户端进程存在。启动器可执行路径独立定位，启动器被删除不必然影响已有游戏检测。

已安装是本地提交记录及指定文件存在检查，不是全量资源校验或可玩性保证。不猜测任意 EXE、不依赖引擎布局。下载目录、版本描述文件或卸载项的存在都不足以单独确认安装。

Game 结构未变化；commands 为空，match_identity 为空。needs_update 的默认 false 表示此平台未检测更新，字节字段为空。GUI 展示 Downloading，更新仍显示为不支持。

## 下载状态

从 `%APPDATA%\miHoYo\HYP\1_1\data\gamedata.dat` 及其 .crc 元数据读取 MMKV 当前键值。仅支持未加密、无过期标志的 v3/v4 格式，实机样本为 v4。核验有效长度、CRC32 和前后元数据一致性，按顺序覆盖键值、应用删除记录，忽略有效长度外的旧数据。读取期间变化或校验失败最多重试三次；持续损坏/未知格式返回错误，不用历史 JSON 片段或目录标记猜测状态。

当前有效记录的 gameBiz、installPath 必须与注册表候选及规范化实际目录一致，downloading 必须为 JSON 布尔值。数据库缺失时不发现下载任务，已安装检测继续；有效记录缺失时 downloading=false。

| 本地安装证据 | 匹配的 downloading | 返回结果 |
| --- | --- | --- |
| 无 | true | 返回，installed=false、downloading=true |
| 有 | true | 返回，installed=true、downloading=true |
| 有 | false/无记录 | 返回，installed=true、downloading=false |
| 无 | false/无记录 | 不返回 |

首次下载进入现有 games/find，不新增任务接口。暂停的首次安装若记录变为 false，将不出现在列表中。此值是启动器最后持久化的下载标志，不是实时网络吞吐；进程异常退出可能留下旧状态。暂停、恢复和完成时标志的实际转换仍需进一步实机验证，不把构造测试当作这些转换的实测。

## 实测范围与限制

绝区零 3.3.0 已验证首次下载及安装完成对照。星穹铁道已采集首次下载样本；加入 downloading 支持后返回 installed=false、downloading=true，完成期仍待现机对照。自动化合成完成记录不代表星铁完成状态已实测。

用户在第二款下载样本采集后要求开始实现，因此先落地当前判据。其他游戏不会因共享引擎或拥有同名配置就自动支持；更新、修复、预下载、接近完成时版本提交顺序、启动器退出后的读取行为及其他渠道仍有验证缺口。不能宣称覆盖所有安装状态或所有 HoYoPlay 游戏。

若后续实测发现版本和启动文件在首次安装完成前同时存在，应修正判据并新增回归证据；不以临时引擎规则兜底。详细证据及方案见 [调研](hoyoplay-install-detection-research.md)和[方案](superpowers/specs/2026-10-29-hoyoplay-support-design.md)。

## 初始安装检测验证（下载支持前）

- workspace 测试 72 项通过，包含下载/完成配置 fixture、重复/无效字段、渠道不符、空/缺失启动文件、损坏候选隔离、登记变化以及 GUI 能力矩阵。
- GUI check 无错误，生产前端构建通过；保留原有 FindDialog 可访问性等警告。
- C FFI 和 Node 构建、真实注册表运行验证通过：只返回 nap_cn，路径为 `F:\ZenlessZoneZero Game`；find(nap_cn) 与列表一致，仍在下载的 hkrpg_cn 查询失败；客户端路径为 `C:\Program Files\miHoYo Launcher\launcher.exe`。
- 构建中的既有 Riot clone 警告和增量编译缓存目录访问警告未影响测试或构建成功。

## downloading 支持验证

新增后 workspace 79 项测试通过；覆盖 MMKV 当前值覆盖、删除记录、有效范围外历史尾部、损坏最新值、CRC/长度/格式标志拒绝、路径身份匹配及 installed/downloading 独立映射。GUI check/build 通过；Node/C FFI 构建和本机运行均返回星铁 installed=false/downloading=true、绝区零 installed=true/downloading=false，find 与列表一致。运行中的开发 GUI 已自动重编译。
