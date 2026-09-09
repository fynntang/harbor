# macOS GUI

[English](../GUI.md) | 简体中文 · [文档目录](README.md) · [项目主页](../../README.zh-CN.md)

## 构建与运行

要求：macOS 14+、Rust 1.89+/Cargo、Swift 6+/macOS SDK 和工具、Bash、Python 3。SwiftPM 没有第三方包依赖。

在仓库根目录执行：

```bash
./scripts/build_and_run.sh
./scripts/build_and_run.sh --build-only
./scripts/build_and_run.sh --verify
swift test --package-path apps/Harbor --scratch-path target/swift-harbor
```

产物为 `dist/Harbor.app`，内置 debug Swift GUI/原生辅助程序和 release Rust CLI。采用本地 ad-hoc 签名，不是 Developer ID 签名或公证版本。移动时应复制整个 bundle。仅 `cargo install` 不会提供停止/删除所需的原生辅助程序。

所有构建模式，包括 `--build-only`，都会检查本 checkout 的内置 helper 是否正在执行操作，存在操作时拒绝重建，否则停止该 checkout 现有的 Harbor GUI。不会停止客户端实例。`--verify` 启动 Harbor 并检查精确可执行路径对应的进程一秒后仍存在，不是界面或账号测试。`--debug` 使用 LLDB；`--logs` 和 `--telemetry` 打开系统日志流，不保证每个操作都有日志事件。

Codex Run 指向同一个脚本。可以指定独立 registry：

```bash
./scripts/build_and_run.sh --verify --registry /absolute/path/test-registry
```

`--registry` 前必须提供模式。GUI 显示 registry 路径，对 `/tmp` 或 `/private/tmp` 显示警告。副本默认目标仍为 `~/Applications/Harbor/ChatGPT-<identifier>.app`，只改变 registry 不会改变应用副本位置。临时数据不适合持久账号；没有自动迁移按钮。

## 控件与状态

可在工具栏语言选择器、**Harbor → 语言** 或顶部菜单栏的 **语言** 子菜单中选择 **English** / **简体中文**。自定义窗口、表单、菜单、状态、进度和 GUI 错误提示即时更新，保留表单输入和操作状态。选择保存在应用 `UserDefaults` 的 `HarborGUILanguage` 中。首次启动使用系统首个受支持的语言（中文映射为简体中文），否则使用英文。设置仅影响 Harbor，不改变客户端副本或系统语言。

底层诊断、CLI 帮助、技术检查报告及系统控制的菜单/对话框文字保留原始/系统语言。双语文案编译进 GUI 可执行文件，无需单独的本地化资源包。

| 控件 | 行为 |
|---|---|
| 创建副本 / Create copy | 选择官方来源和新名称，调用 `clone`；没有 GUI `create`/`adopt` 表单。 |
| 启动 / Start | 调用 `start`；应用检查存在问题或启动冲突时禁用。 |
| 停止 / Stop | 在 `running` 或 `helpers_running` 状态可用，请求正常退出。 |
| 删除实例… / Delete instance | 为符合条件的受管副本显示范围确认；总是在 `remove` 前调用 `stop`。 |
| 检查 / Check | 展示 `doctor` 结果，包括警告。 |
| 更换图标… / Change icon | 副本符合条件、检查无问题且状态为 `stopped` 时可用。 |
| 刷新 / Refresh | 重新读取登记与状态；窗口激活和 ⌘R 也会触发。 |
| 文件夹控件 | 在 Finder 显示路径。日志目录按钮目前打开 `codex_home` 的父目录；接管数据时可能不是 Harbor 日志所在目录。 |

状态层通过 busy 状态串行执行修改操作和刷新。错误会显示，不推断操作成功。身份或版本差异会标为需要检查。GUI 没有 `--accept-version-change`、环境变量编辑器、更新器或恢复按钮。无效的 registry 条目可能导致整个列表读取失败；此界面不负责修复登记。

顶部帆船菜单支持打开 Harbor、创建副本和退出。主窗口使用固定值复用。关闭全部窗口保留菜单栏入口，操作进行时会阻止正常退出。客户端仍为独立进程。没有开机启动项、launchd 守护程序或定时轮询。

## 图标

Harbor 自身的 Dock/Finder 图标使用与菜单栏相同的 `sailboat` SF Symbol：白色线条、透明背景。菜单栏模板跟随系统外观，打包的 Dock 图片固定为白色。`scripts/generate_app_icon.swift` 生成 iconset/PNG；打包复制 `Resources/Harbor.icns` 并设置 `CFBundleIconFile`，应用启动时还会显式加载它。

实例图标在**更换图标…**中选择 PNG、JPEG 或 ICNS，文件最多 16 MiB。GUI 将图像等比例放入透明的 1024 像素正方形。菜单栏图像可选实例首字母或图片轮廓。模板忽略颜色，不透明照片会形成不透明轮廓。客户端自身必须开启菜单栏入口才会显示该图标；Harbor 不切换该设置。

CLI 接受边长最多 4096 像素、文件最多 16 MiB 的正方形 PNG，`--tray-image` 可选。条件为 `adopted_data=false`、Harbor 风格 Bundle ID、身份/版本匹配及预期的资源/签名结构。通过 `create` 登记的准备好应用也可能满足条件；没有独立的密码学来源记录证明它由 `clone` 创建。

GUI 在报告残留辅助进程时禁用图标更新。核心 `icon` 命令在暂存前和交换前只检查匹配的主可执行程序，不使用停止/删除的完整辅助进程扫描。建议先点击**停止**再改图标。图标暂存使用 `cp -cR`，没有 clone 命令的 `ditto` 回退。

操作替换支持的 Dock/Finder 资源及可选的 18/36 像素菜单栏模板，重签外层副本、验证后原子交换。最终验证失败会尝试回滚；回滚失败会保留旧应用的暂存路径。不改变 ASAR、嵌套程序或账号目录。重新启动客户端后使用新资源。没有内置的恢复原厂图标操作。

<a id="removal-and-recovery"></a>
## 删除与恢复

原生 helper 对 macOS 登记/就绪、发送正常退出请求和观察退出共设置 15 秒时限。之后 Rust 最多再等待五秒，检查 app、`codex_home`、`gui_home` 中的辅助进程。已知路径的孤儿 Crashpad、Computer Use 和快捷键监视程序，在核对属主、父进程及启动时间后可能收到 SIGTERM。未知或仍有活动父进程的 helper 会阻止完成，不回退到 SIGKILL。失败或超时会阻止 GUI 删除。

删除默认将整个 Profile 目录保留到 `<root>/retained/<identifier>-<random>/profile`，仅将应用移到系统废纸篓。登记会从列表移除，界面显示保留路径。勾选**同时将账号数据移到废纸篓**会把整个 Profile 目录也移走，包括元数据、日志和数据。Harbor 不清空废纸篓。 删除结果和错误提示均可点击关闭；关闭只清理提示，不删除文件。若保留目录在 Harbor 外被删除，刷新或切回窗口会清除失效的删除结果提示。这些提示仅保存在内存中，退出 Harbor 后也会消失。

删除要求 `adopted_data=false`、预期的 Harbor Bundle ID，且数据路径必须正好是该 Profile 自己的 `codex`/`gui`。运行中的进程、默认数据重叠、registry 重叠及其他实例路径重叠都会被拒绝。当前路径（包括工作目录）和应用身份仍须通过检查，应用或目录缺失可能阻止删除。这不是通用的损坏登记清理工具。与 start/icon 不同，仅版本差异不会阻止生命周期操作。

打包后，在仓库根目录使用 CLI：

```bash
dist/Harbor.app/Contents/Helpers/harbor stop work
dist/Harbor.app/Contents/Helpers/harbor remove work --yes
```

`--delete-data` 可选，应明确选择。CLI `remove` 本身不停止程序。后续项目移到废纸篓失败时，原生 helper 尝试恢复此前项目，不覆盖现有路径。默认保留模式失败时还会尝试恢复 Profile 目录。多个路径不是一个原子事务；中断或回滚失败后可能需要从 `retained` 或废纸篓手工恢复。

保留的 `profile.json` 仍记录原路径。恢复时先退出相关客户端，并从废纸篓还原应用。如果原登记路径空缺且未被复用，可把整个保留的 Profile 移回原位置。否则用空闲名称执行 `adopt`，传入实际保留的 `codex`/`gui` 路径，检查路由、执行 `doctor` 后再通过 Harbor 启动。接管不会修补旧 `LSEnvironment`，所以移动数据后仍不应直接启动应用。接管的 Profile 不支持 GUI 删除或图标更新。路径被占用或范围不确定时应先检查，不要覆盖。没有自动恢复流程。

## 桥接与 JSON 协议

[HarborClient](../../apps/Harbor/Sources/Harbor/Services/HarborClient.swift) 用独立 argv 调用 `Contents/Helpers/harbor`，不经过 shell；stdout/stderr 在 UI 线程外并发读取。[HarborStore](../../apps/Harbor/Sources/Harbor/Stores/HarborStore.swift) 管理操作顺序，文件和进程校验仍由 Rust 负责。[HarborNative](../../apps/Harbor/Sources/HarborNative/NativeOperations.swift) 处理正常退出和废纸篓。

`--json` 支持 `list`、`show`、`status`、`clone`、`icon`、`start`、`stop`、`remove`、`doctor`，不支持 `create`、`adopt`、`logs`、`shortcut`。[json.rs](../../crates/harbor-cli/src/json.rs) 定义协议：

- stdout：一个对象，包含 `api_version: 1`、`ok`，以及 `data` 或 `error`。
- 普通操作错误退出 1；Clap 参数语法错误输出 stderr、退出 2，不使用 JSON envelope。
- `doctor`：Store 初始化成功后，即使诊断失败也返回成功 envelope/退出 0，并设置 `data.passed=false`；必须检查该字段。仅版本或冲突警告不一定将它设为 false。
- clone 进度：stderr JSON 行，包含 `event: progress` 与 `stage`。
- JSON 状态：`running`、`stopped`、`helpers_running`、`conflict`；列表遇到进程检查错误时可能报告 `unknown`。`pids` 只包含主进程 ID，`helpers_running` 也不例外。

通过 Finder 启动 Harbor 不会读取终端 shell 配置。仅在终端导出的环境值不会自动提供给其启动的实例。详见[路由与环境变量](ARCHITECTURE.md#environment)。

<a id="release-build"></a>
## 发布构建与签名

上面的开发命令仍生成 `dist/Harbor.app`，包含 debug Swift GUI/原生辅助程序、release Rust CLI 和 ad-hoc 签名。独立发布命令以 release 模式编译三个程序，输出到 `dist/release/Harbor.app`，不停止或替换开发版应用。首版仅支持 macOS 14+ / arm64。

没有 Developer ID 证书时，可使用明确的本地签名发布模式免费分发预览版。三个程序均使用 release 编译，校验签名、标识、架构和版本，产物为 `dist/release-adhoc/Harbor.app`。此模式不查询签名身份、不停止 GUI，也不覆盖 Developer ID 产物。它**未经公证**，签名不代表受信任的发布者身份。

```bash
./scripts/build_and_run.sh --release-adhoc
mkdir -p dist/artifacts
ditto -c -k --sequesterRsrc --keepParent dist/release-adhoc/Harbor.app dist/artifacts/Harbor-0.0.1-macos-arm64.zip
python3 scripts/package_dmg.py dist/release-adhoc/Harbor.app
```

从项目 GitHub Release 下载 DMG 和校验文件，核对 DMG 校验值后打开，将 Harbor.app 拖入 Applications 入口，推出磁盘后从“应用程序”启动 Harbor。覆盖安装前请退出 Harbor。ZIP 继续作为备选。打包脚本生成压缩只读 DMG，挂载后验证签名、逐个比对应用文件与输入发布版一致，再更新包含 ZIP（若存在）和 DMG 的 `SHA256SUMS.txt`；不重新构建或签名应用。若只下载一种格式，对照校验文件中的相应行核对即可。另行安装官方 ChatGPT/Codex 应用。若首次打开因无法验证发布者而被拦截，确认下载来源后按 [Apple 官方说明](https://support.apple.com/en-us/102445) 操作，无需全局关闭 Gatekeeper。尚未在另一台 Mac 上完成安装验收。

Developer ID 签名继续使用下面的独立流程。

在构建机器的钥匙串中安装有效的 **Developer ID Application** 证书及其对应私钥。将 `HARBOR_SIGNING_IDENTITY` 设置为该证书的 40 位 SHA-1 指纹；指纹是标识符，不是私钥。私钥及其密码不得进入源码、命令示例或发布附件。

```bash
HARBOR_SIGNING_IDENTITY=YOUR_CERTIFICATE_SHA1 ./scripts/build_and_run.sh --release-only
```

将占位符替换为证书指纹。配置缺失、格式不正确、签名身份不可用或证书类型不符时，命令会在构建或替换应用前停止，不降级为 ad-hoc 签名。两个辅助程序使用稳定签名标识 `local.harbor.desktop.cli` 和 `local.harbor.desktop.native`；GUI 保留 `local.harbor.desktop`，继续读取已有偏好。

脚本先签辅助程序，再签 Harbor.app，使用选定身份、Hardened Runtime 和安全时间戳。写入最终本地产物前，`scripts/signing.py` 验证 Apple 签名信任锚、签名、签名团队、标识、运行时标志、时间戳、三个程序一致的 arm64 架构及 GUI/CLI/工作区版本一致性。检查失败不会替换先前的发布版应用。不额外添加 entitlement。这些配置只用于 Harbor 自身和内置辅助程序；用户创建的客户端副本继续使用已有本地签名流程，不会获得发布者的证书或私钥。

**此命令生成已签名应用，不代表已完成公证分发。** Apple 公证、票据装订、ZIP/校验文件生成、另一台 Mac 的安装验收及 GitHub Release 发布仍是后续步骤，不能将中间产物标记为已公证或已通过 Gatekeeper。当前 CI 不执行正式签名，测试签名门槛并构建、验证本地签名发布版，不访问签名凭据。

## 实例图标的名称角标

创建窗口默认启用 **使用名称角标**，可预览官方图标加彩色角标的大、小尺寸效果及单色菜单栏简称。角标文字留空时自动生成简称（`work` → `WORK`、`toobit` → `TOO`）；名称不超过四个字符时保留全名，更长时取前三个字符。自定义简称最多四个字符，不允许控制字符。可选择六种颜色，也可按名称稳定自动分配。橙色/黄色角标使用黑字，其他颜色使用白字。菜单栏图标取角标前两个字符，背景透明。

已有实例停止后，选择 **更换图标… → 原版加角标**；**自选图片** 保留原有图片上传流程。应用成功的角标设置按应用路径保存在本机，取消编辑不会保存。

新副本会在本地签名前保存 `Contents/Resources/harbor-original-icon.png`，后续换图标保留该文件并始终从它重新绘制。没有该快照的旧副本，GUI 读取 `/Applications/ChatGPT.app/Contents/Resources/icon-chatgpt.png`，不会使用副本中已经自定义过的图标。原图无法读取时角标模式不可用；创建时可以关闭角标或重新选择正确的官方原版，已有实例可以使用自选图片。

GUI 创建流程依次执行 `clone` 和 `icon`，两者为独立操作。如果克隆成功后图标应用失败，已创建实例仍保留在列表中，创建窗口关闭并显示错误。应通过更换图标重试，避免重复克隆。CLI 克隆会保留原图，但不会自动生成角标。更换角标不修改账号数据。

## 显示名称与实例标识

创建表单支持 1–48 个 Unicode 字符，包括中文、大小写、空格和括号，不允许控制字符或首尾空白。列表、详情、确认对话框及生成角标使用完整显示名称；详情另外显示实例标识和 Bundle ID。符合原字母/数字/连字符规则的 ASCII 名称转成小写标识；其他名称使用 ASCII 前缀（或 `profile`）加随机后缀。标识在创建时固定，用于应用文件名、数据路径和 CLI 操作。旧实例回退使用原名称，不改写其路径或 Bundle ID。本功能不包含已有实例重命名。
