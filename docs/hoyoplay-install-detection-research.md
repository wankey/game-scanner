# 米哈游启动器安装状态检测调研

日期：2026-10-29。仅调研，无检测实现。样本为国服启动器 1.19.0.382、绝区零首次下载与安装完成前后对照。

## 当前结论

已经取得绝区零首次下载与安装完成的正反样本。游戏目录版本记录、官方配置指定启动文件及下载组件本地版本提交出现一致变化，可作为后续验证方向；尚未验证其他游戏、更新及卸载残留，不能视为通用完成规则。不能仅依据卸载项、GameInstallPath、文件夹存在、pkg_version、version_info 或游戏资源存在认定已安装。

官方来源与游戏标识见 [官方来源核查](hoyoplay-official-sources-research.md)。下载期脱敏快照见 [下载期证据](hoyoplay-downloading-evidence.json)。快照未保存账户、设备 ID、下载事务 ID 或完整日志。

## 本机一手证据

### 启动器定位

读取 `HKLM\Software\Microsoft\Windows\CurrentVersion\Uninstall\HYP_1_1_cn`：

- DisplayName：米哈游启动器。
- DisplayVersion：1.19.0.382。
- InstallLocation：`C:\Program Files\miHoYo Launcher`。
- DisplayIcon：安装目录下 `launcher.exe`。

安装根目录 `config.ini` 的 `[hyp]` 包含 channel=1、sub_channel=1、primary_game=hyp_cn。`HKCU\Software\miHoYo\HYP\1_1` 也包含启动器 InstallPath。仅验证了本机此渠道和注册表位置，尚未验证所有视图及国际服。

### 下载期已经出现的游戏注册信息

`HKCU\Software\miHoYo\HYP\1_1\nap_cn`：GameBiz=nap_cn、GameInstallPath=`F:\ZenlessZoneZero Game`、GamePackageName 为空。

机器卸载项 `nap_cn_1_1_jGHBHlcOq1_production` 已出现，DisplayName=绝区零。其 InstallLocation 指向启动器目录，并非游戏目录；UninstallString 为启动器 `launcher.exe --uninstall_game=nap_cn`。

因此：游戏卸载项是候选来源，不能作为完成判据；不得把此 InstallLocation 直接映射成游戏路径。

### 游戏目录中的下载期记录

`F:\ZenlessZoneZero Game\config.ini` 实测为：

```ini
[General]
game_version=
channel=1
sub_channel=1
cps=hyp_mihoyo
uapc={"hyp":{"uapc":""},"nap_cn":{"uapc":""}}
downloading_mode=chunk
```

目录已经包含 `pkg_version`、`file_category_launcher`、值为 `CNPRODWin3.3.0` 的 `version_info`、`ZenlessZoneZero_Data`、大量 DLL，以及 chunk、staging 子目录。采样时配置中的启动文件 `ZenlessZoneZero.exe` 尚不存在。

这证明版本描述和游戏文件会在下载未结束时存在。`game_version` 为空是本次下载样本的现象；非空是否能证明已完成、更新中是否保留旧版本，必须继续验证。chunk/staging 是否在完成后保留也未验证，不能直接当作通用排除标记。

### 启动器本地数据

`%APPDATA%\miHoYo\HYP\1_1\data\gamedata.dat` 为二进制数据，本次 SQLite 只读打开明确返回“file is not a database”。旁有 gamedata.dat.crc；尚未确认其完整存储协议及当前键值读取规则。

为取证临时从字节中提取可解码 JSON 片段，发现：

- 原神、崩坏3、星铁均存在默认路径、名称和 gameExeName，即使用户尚未安装这些游戏。不能把条目存在认定为已安装。
- 绝区零包含 gameBiz、installPath、persistentInstallPath、version、downloading、gameInstallStatus、playStatus、selectPackageInfo 等字段。
- 同一个 nap_cn 有多个旧片段；包括 downloading=false 与 true，persistentInstallPath 从空变为实际路径，version 始终为空。
- playStatus.installTime 在首次下载期间已非空，因此不是安装完成时间的可靠证据。
- gameInstallStatus 是包含配置的对象，不是已验证的布尔安装完成标记。

JSON 片段搜索仅用于发现字段，不是有效的当前状态解析器。不得将“首个匹配”“最后一个匹配”或合并片段作为实现；仍需确认存储格式、有效记录、删除语义、完整性及运行中一致性。

### 下载组件数据库

以 SQLite URI mode=ro 读取 `%APPDATA%\miHoYo\HYP\1_1\modules\sophon\sophon_db`：

- chunk_config.db/config：有 package_id、branch、depot_id、matching_field、install_dir、local_version/server_version、local_build_id/server_build_id 等列；本次表为空。
- chunk.db/depot_file_data：有包/构建/文件标识、file_status、install_dir；采样聚合结果为包 FfGDa3bsvp、目标游戏目录、file_status=1、1647 条记录。
- chunk_manifest.db/depot_manifest：存储远端 manifest 标识、版本及校验/下载信息。

file_status 数值含义尚未验证。空配置表不阻止正在下载；远端 manifest 存在也不证明本地完成。不得直接据此构造安装完成判断。

## 下一轮验证

1. 保留当前下载样本；等待用户确认启动器显示安装完成/可启动，再读取同一组来源，比较 game_version、启动文件、有效 gamedata 记录、chunk_config 提交内容及 staging 的变化。
2. 分别验证启动器运行和关闭后的读取结果；任何暂停/退出、修复、更新、卸载等操作需在后续获用户授权后进行，不改变当前下载。
3. 确认 gamedata 的正式存储结构，或证明仅靠更稳定的注册表候选和游戏目录提交记录即可可靠检测，避免依赖历史 JSON 片段或日志。
4. 用未安装默认条目、首次下载、安装完成、更新中、目录缺失/卸载残留验证规则。首次安装未完成与已安装游戏更新中需要分别处理。
5. 第二款游戏和其他渠道需独立验证；本机绝区零单一样本不能证明所有游戏均适用。

当前不新增 API、Game 字段、平台实现或 GUI 入口，也不恢复任何引擎专用识别逻辑。

## 安装完成后的实测对照

用户确认安装完成后，以相同来源再次只读采样。脱敏快照见 [完成期证据](hoyoplay-installed-evidence.json)。

| 证据 | 首次下载阶段 | 用户确认安装完成后 |
| --- | --- | --- |
| 注册表 GameInstallPath | 已存在 | 保持原路径 |
| config.ini / game_version | 空 | 3.3.0 |
| config.ini / downloading_mode | chunk | 空 |
| 官方配置指定的 ZenlessZoneZero.exe | 不存在 | 存在，10,443,040 字节 |
| chunk、staging 根子目录 | 存在 | 均不存在 |
| chunk_config.db / config | 空表 | 20 条 depot 记录，本地/远端版本均为 3.3.0，本地/远端构建均为 4EEwCYcQ9GWt |
| chunk.db / file_status=1 | 1647 条 | 11342 条 |
| gamedata.dat 提取片段的 version | 空 | 仍为空 |

另读取 `ZenlessZoneZero_Data/Persistent/file_verify`，内容为 game_version=3.3.0、verify_result=success、verify_time=2026-10-29T18:06:26。下载期未单独采样此文件，因此不能断言其此前不存在。官方配置仅部分游戏启用此记录，不能作为全平台必要条件。

补充检查 gamedata.dat：文件长 32768 字节，首 4 字节按小端解释为 9426。在该长度所对应的候选范围内仍有同一游戏的多个片段，version 为空、downloading 有 true/false。此探索未证明头部语义，也未完成键值协议解析；不能从文件末尾、片段顺序或 version 字段直接推断完成状态。完成后该文件仍无法提供与 game_version 一致的明确版本片段，因此优先研究目录提交记录及下载组件数据库，而非将其作为唯一真相。

### 当前可验证方向及边界

候选来自注册表 GameBiz/GameInstallPath；以真实目录 config.ini 的已提交 game_version、官方配置指定的启动文件、适用时的下载组件 local_version/local_build_id 相互核验。此方向不依赖 Unity 特征，不通过扫描任意 EXE 猜测游戏身份。

目前仅证明上述记录随本机绝区零首次安装发生了对应变化，尚未证明：

- game_version 和本地构建是否在首次安装的最后一步才提交。
- 不同游戏、整包与 chunk 下载模式、国际服是否采用同样记录。
- 已安装游戏更新、预下载、修复期间哪些记录保留，如何区别“已有可用安装”与“首次未完成”。
- 下载组件数据库在启动器退出或清理缓存后是否仍保留，包 ID 与 GameBiz 如何稳定关联。
- 注册表残留、目录被手动删除/替换、启动文件缺失时的结果。

因此不在此次调研中实现临时判断。下一步应验证第二款游戏及上述状态边界，再确定最小必要判据；无需以 UnityPlayer.dll、GameAssembly.dll 等引擎文件作为条件。

## 第二款游戏：星穹铁道下载期

2026-10-29 19:34（UTC+08:00），用户开启第二款游戏下载后只读采样，详见 [第二款下载期证据](hoyoplay-second-game-downloading-evidence.json)。

- 注册表 hkrpg_cn/GameInstallPath 已指向 `F:\Star Rail Game`。
- config.ini：game_version 为空，channel/sub_channel 均为 1，downloading_mode=chunk。
- 官方配置指定的 StarRail.exe 不存在，StarRail_Data/Persistent/launcher_verify 不存在。
- 根目录已存在 StarRail_Data、pkg_version、UnityPlayer.dll、UnityCrashHandler64.exe、mhypbase.dll、chunk 和 staging。
- 下载组件 chunk.db 中出现包 Je5DM3y6Q2、该安装路径、file_status=1 的 293 条记录；chunk_config.db 未出现此游戏的本地版本提交记录。
- 同时检查已安装绝区零：game_version 仍为 3.3.0，指定 EXE 存在，本地版本/构建提交记录保持。启动器存在另一个下载任务不应使已安装游戏被排除。

第二款游戏的早期下载样本与第一款一致，也进一步证明引擎文件/资源存在不能代表安装完成。目前仍缺星铁接近完成、解包/校验及完成后的状态；不能从当前一次负样本推断中间所有阶段的提交时机。

## downloading 实现前的存储格式验证

用户要求支持 downloading 后，继续核查 Tencent/MMKV 一手源码，确认本机 gamedata.dat 符合未加密 v4 记录格式：

- [MMKVMetaInfo.hpp](https://github.com/Tencent/MMKV/blob/master/Core/MMKVMetaInfo.hpp)：元数据含 CRC、版本、sequence、IV、actualSize 和 flags。
- [MMKV_IO.cpp](https://github.com/Tencent/MMKV/blob/master/Core/MMKV_IO.cpp)：数据有效范围为头部之后 actualSize 字节，读取前校验 CRC；v3 起实际长度也保存在元数据中。
- [MiniPBCoder.cpp](https://github.com/Tencent/MMKV/blob/master/Core/MiniPBCoder.cpp)：先消费容器前缀，顺序解码键值，相同键覆盖，零长度值删除；不能采用历史 JSON 搜索。
- [CodedInputData.cpp](https://github.com/Tencent/MMKV/blob/master/Core/CodedInputData.cpp)：键值由 varint 长度界定，存储的字符串值再包含自己的长度前缀。

以上是 MMKV 格式来源，不是米哈游对本地格式的兼容性承诺。2026-10-29 本机实测：元数据版本 4、IV 全零、flags=0，数据头长度与元数据长度相同，实际数据 CRC32 与元数据完全一致。用上述覆盖/删除语义重建当前键值后，hkrpg_cn 的 downloading=true，nap_cn=false。身份和路径均与注册表实际候选相符。见 [当前状态脱敏证据](hoyoplay-downloading-state-evidence.json)。

实现采用有界、只读解码并校验前后元数据，失败时有限重试，不退回搜索 JSON 片段；只接受已知未加密且无扩展标志的 v3/v4 子集。数据格式变化会报告错误。

这项证据支持读取“启动器当前持久化的下载标志”，没有证明暂停/恢复/完成时的实时转换，也不等同于网卡正在收包；异常退出可能保留旧记录。已安装证据和 downloading 独立，首次下载可返回 installed=false。暂不新增下载进度/字节数。
