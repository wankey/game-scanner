# 米哈游启动器官方来源核查

调研日期：2026-10-29。范围：官方公开页面与官方域名返回的服务端配置；不读取用户账户信息，不添加实现代码。

## 结论与边界

官方服务端配置可以提供游戏 ID、业务标识、显示名称、默认安装子目录和配置的启动 EXE。这给游戏身份校验提供了可验证的数据来源，但不能据此判断本机已经安装游戏。安装完成、首次下载中、更新中、修复中等本机状态，仍需本机记录与状态转换的对照证据。

本次检查的官方网页及接口响应未提供本地数据库/配置格式的公开规范，也未提供本机安装状态枚举的正式定义。这是本次检索范围内的证据缺口，不代表断言官方绝无相关文档。

## 来源

1. [米哈游启动器官网](https://launcher.mihoyo.com/)：页面标题为“米哈游启动器 | miHoYo 一站式游戏平台”。
2. [HoYoPlay 官网](https://hoyoplay.hoyoverse.com/)：页面标题为“HoYoPlay | HoYoverse's One-Stop Game Platform”，说明为“Access all of HoYoverse's PC-playable games in one place.”。
3. [国服 getGames 实测响应](https://hyp-api.mihoyo.com/hyp/hyp-connect/api/getGames?launcher_id=jGHBHlcOq1&language=zh-cn)：本次通过 HTTPS GET 无认证读取，返回 `retcode: 0`、`message: "OK"`。`data.games` 包含 `id`、`biz`、`display.name`、`display_status` 等字段。
4. [国服 getGameConfigs 实测响应](https://hyp-api.mihoyo.com/hyp/hyp-connect/api/getGameConfigs?launcher_id=jGHBHlcOq1&language=zh-cn)：本次通过 HTTPS GET 无认证读取，返回 `retcode: 0`、`message: "OK"`。`data.launch_configs` 包含 `game.id`、`game.biz`、`exe_file_name`、`installation_dir`、`default_download_mode` 等字段。

来源 3、4 是官方域名当前返回的真实数据，不是已公开承诺兼容性的开发者 API 文档。参数和响应可能变化；本次未证明离线可用性、历史版本兼容性或国际服等其他渠道兼容性。国际服接口的一次请求发生 TLS 连接错误，本次未据此作国际服数据结论。

## 已核验的国服启动配置

以下表格由来源 3、4 按 `game.id` 关联，均为调研时返回值。

| 游戏 | ID | biz | exe_file_name | installation_dir |
| --- | --- | --- | --- | --- |
| 原神 | `1Z8W5NHUQb` | `hk4e_cn` | `YuanShen.exe` | `Genshin Impact Game` |
| 崩坏：星穹铁道 | `64kMb5iAWu` | `hkrpg_cn` | `StarRail.exe` | `Star Rail Game` |
| 绝区零 | `x6znKlJ0xK` | `nap_cn` | `ZenlessZoneZero.exe` | `ZenlessZoneZero Game` |
| 崩坏3 | `osvnlOc0S8` | `bh3_cn` | `BH3.exe` | `Honkai Impact 3rd Game` |
| 星布谷地 | `ERYDdvf0w8` | `hyg_cn` | `PetitPlanet.exe` | `PetitPlanet Game` |
| 崩坏：因缘精灵 | `m8TGERqknS` | `abc_cn` | `NexusAnima.exe` | `NexusAnima Game` |

不能将 `installation_dir` 当成本机实际安装绝对路径，也不能将 `exe_file_name` 当作文件已存在或安装已完成的证据。它描述的是服务端配置的启动文件名，未证明所有游戏的实际主进程与其一致。

来源 3 同时返回预约游戏和可用游戏；字段 `display_status` 分别可见 `LAUNCHER_GAME_DISPLAY_STATUS_RESERVATION_ENABLED` 和 `LAUNCHER_GAME_DISPLAY_STATUS_AVAILABLE`。无认证的远端响应说明的是服务端展示状态，不能解释为本机安装状态。

## 与安装检测有关但尚不能采用的字段

来源 4 中，本次六项配置的 `default_download_mode` 均为 `DOWNLOAD_MODE_CHUNK`。这一字段不能说明当前本机是否存在下载任务。

来源 4 中，绝区零配置为 `enable_write_verify_result: true`，`write_verify_result_path: "ZenlessZoneZero_Data/Persistent/file_verify"`；星穹铁道配置为 `enable_write_verify_result: true`，`write_verify_result_path: "StarRail_Data\\Persistent\\launcher_verify"`；表中其他四项对应开关为 `false`、路径为空。因此校验结果文件不能直接假定为所有游戏通用的安装完成标记。本次没有核实这些文件内容、创建时机及更新/修复失败后的行为。

## 后续验证条件

以下是研究建议，不是已验证的检测规则：

1. 对照同一台机器、同一游戏的首次下载中与安装完成后记录，确定实际路径来源和明确的状态变化。
2. 确认记录中的游戏 ID/biz 能否与来源 3、4 关联，避免按显示名称或 Unity 目录结构推测身份。
3. 在更新、修复和卸载残留情形下，核实安装记录是否仍有效；游戏“已安装”和“当前可启动”应分别解释。
4. 验证是否有本地缓存可提供必要身份信息，避免让已安装列表依赖网络。

在完成这些核验前，不应仅根据 EXE 存在、默认目录存在或远端游戏列表新增临时安装检测逻辑。
