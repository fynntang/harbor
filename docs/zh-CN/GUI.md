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
| 更新副本 / Update Copy | 从所选官方来源更新已停止的托管应用，保留身份、图标和账号路径。 |
| 启动 / Start | 调用 `start`；应用检查存在问题或启动冲突时禁用。 |
| 停止 / Stop | 在 `running` 或 `helpers_running` 状态可用，请求正常退出。 |
| 删除实例… / Delete instance | 为符合条件的受管副本显示范围确认；总是在 `remove` 前调用 `stop`。 |
| 检查 / Check | 展示 `doctor` 结果，包括警告。 |
| 更换图标… / Change icon | 副本符合条件、检查无问题且状态为 `stopped` 时可用。 |
| 刷新 / Refresh | 重新读取登记与状态；窗口激活和 ⌘R 也会触发。 |
| 文件夹控件 | 在 Finder 显示路径。日志目录按钮目前打开 `codex_home` 的父目录；接管数据时可能不是 Harbor 日志所在目录。 |

状态层通过 busy 状态串行执行修改操作和刷新。错误会显示，不推断操作成功。身份或版本差异会标为需要检查。GUI 没有 `--accept-version-change`、环境变量编辑器或恢复按钮。无效的 registry 条目可能导致整个列表读取失败；此界面不负责修复登记。

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

`--json` 支持 `list`、`show`、`status`、`clone`、`update`、`icon`、`start`、`stop`、`remove`、`doctor`，不支持 `create`、`adopt`、`logs`、`shortcut`。[json.rs](../../crates/harbor-cli/src/json.rs) 定义协议：

- stdout：一个对象，包含 `api_version: 1`、`ok`，以及 `data` 或 `error`。
- 普通操作错误退出 1；Clap 参数语法错误输出 stderr、退出 2，不使用 JSON envelope。
- `doctor`：Store 初始化成功后，即使诊断失败也返回成功 envelope/退出 0，并设置 `data.passed=false`；必须检查该字段。仅版本或冲突警告不一定将它设为 false。
- clone/update 进度：stderr JSON 行，包含 `event: progress` 与 `stage`。
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

## 从官方原版更新副本

官方客户端更新后，先退出副本及全部辅助进程，包括浏览器集成。在实例详情中点击**更新副本…**，核对来源路径和版本后确认。来源默认 `/Applications/ChatGPT.app`，也可选择其他位置的官方原版。Harbor 不自动关闭浏览器，也不自动启动更新后的副本。若仍有辅助进程，请完全退出使用该实例扩展的浏览器后刷新。

```bash
harbor update work --source /Applications/ChatGPT.app
```

仅支持未接管、登记身份/版本一致且数据位于 Harbor 自有路径的托管副本。要求数字构建号递增；短版本和构建号均相同时不做替换。拒绝降级或可执行文件结构改变。校验原版/复制件签名与兼容性标记，保留名称、Bundle ID、应用位置、环境配置和数据路径。自定义图标文件沿用；未自定义的副本使用新版官方图标。原版图标快照随来源更新。

此操作不复制、删除或迁移账号目录。新版首次启动可能迁移数据库，因此本功能不是数据库备份或自动降级机制。普通准备、校验或清单发布错误会恢复旧应用。异常退出或回滚失败可能留下应用旁的 `.harbor-update-*` 暂存目录，请保留它，先检查应用与清单版本再恢复。不能把 `--accept-version-change` 当作更新流程。

### 直接退出副本后的辅助进程

使用 Cmd-Q 直接退出副本后，可能仍有辅助进程运行。返回 Harbor 或刷新后会显示“辅助进程未退出”；点击“清理残留”会执行与 `harbor stop <id>` 相同的安全清理。即使其他进程无法清理，已知的孤立辅助进程仍会收到 SIGTERM。活跃或未知进程会被保留，并显示名称、PID 和父进程 PID；浏览器集成进程需要先退出所属浏览器，再重试。清理失败也会刷新状态；有残留时仍禁止更新和删除。刷新本身不会自动终止辅助进程。

## Harbor 自动更新

发行构建集成 Sparkle 2.9.6。“Harbor → 检查 Harbor 更新…”及顶部菜单栏均提供手动检查和“自动检查 Harbor 更新”开关。默认在 Harbor 运行期间每天检查一次，安装需要用户确认。Sparkle 负责下载、校验、替换 Harbor 并重新启动。Harbor 有操作进行时拒绝检查，重启会等待空闲，原有退出保护继续生效。此机制不更新托管副本或账号目录。Debug 构建禁用更新器。Harbor 菜单文字跟随所选语言；Sparkle 标准对话框跟随系统支持的语言。

稳定版订阅地址为 `https://github.com/fynntang/harbor/releases/latest/download/appcast.xml`。草稿和预发布版本不属于此通道。更新清单和 ZIP 均须通过 `config/sparkle-public-key.txt` 中公钥对应的 Ed25519 签名校验；签名无效时拒绝安装，解压前完成验证。网络检查失败不会替换应用。更新要求项目版本递增；macOS 两个版本字段使用相同的 Cargo 版本。目前已发布的 0.0.1 没有更新器，用户需要先手动安装一次含更新器的版本。在稳定发布附带更新清单之前，手动检查可能提示无法获取订阅。

### 准备和发布更新

签名私钥保存在 macOS 登录钥匙串的 Sparkle 服务中，账号为 `local.harbor.desktop`。Git 中只保存公钥。换电脑前应安全备份该签名身份；丢失私钥会导致既有 ad-hoc 安装无法信任未来更新。不要每次发布重新生成密钥。普通 CI 工作流没有签名密钥；独立发布工作流需要配置 `SPARKLE_PRIVATE_KEY` Actions Secret。

1. 递增 Cargo 中的日历版本并更新 Cargo.lock（例如 `26.9.101046` → `26.9.101047`）。保留已发布标签和附件不变。
2. 构建发行版，再向新目录打包 DMG 和签名更新：

```sh
./scripts/build_and_run.sh --release-adhoc
python3 scripts/package_dmg.py dist/release-adhoc/Harbor.app --output-dir dist/next-release
python3 scripts/package_update.py dist/release-adhoc/Harbor.app --output-dir dist/next-release
```

Developer ID 构建使用 `--release-only`，并向 `package_update.py --team TEAM_ID` 传入签名团队；最终应用应先公证并装订票据，再打包。Ed25519 更新签名不替代 Developer ID 或公证。当前 DMG 脚本只验证 ad-hoc 构建。

3. 验证附件并在测试 Mac 上安装、重启，然后将生成的 ZIP、DMG、`appcast.xml` 和 `SHA256SUMS.txt` 附到对应 `v<version>` GitHub Release。以稳定版发布并标记 latest。先在草稿中上传全部附件，齐全后再公开。固定订阅地址随后解析到新版签名清单。签名后不要再修改 XML 或 ZIP 字节。

`package_update.py` 检查最终应用、内嵌公钥/订阅/版本及钥匙串公钥一致性，为 ZIP 和清单签名并验证，再写入校验和。它不上传、不修改 Git 标签、不导出私钥。更新器行为及密钥恢复限制见 [Sparkle 文档](https://sparkle-project.org/documentation/)。

### 日历版本标签

标签采用 `vYY.M.DHHmm`，使用发布时 Asia/Shanghai 的日期与 24 小时时间。月份和日期不补前导零，末尾时间始终保留四位。9 月 1 日午夜为 `v26.9.10000`，9 月 10 日 11:56 为 `v26.9.101156`。`0000` 是有效的午夜时间，不使用循环相加编码。Cargo 是唯一版本来源，GUI、CLI、构建号、更新清单和附件名称使用同一数字版本（只有标签带 `v`）。当前配置为 `26.9.101046`，即 2026-09-10 10:46。`scripts/version.py` 验证日期、闰年和时分范围。按数字分段比较可保持跨分钟/小时/日/月/年的顺序，不能直接按字符串字典序排序。同一分钟只能对应一个唯一标签；再次发布应等待下一分钟，不复用标签。支持年份 2000–2099。准备发布时设定版本并更新 Cargo.lock，构建时不会自动改写。

### GitHub Actions 发布工作流

`.github/workflows/release.yml` 在后续包含该工作流的提交推送 `v*` 标签时运行。它验证标签与 Cargo 一致，执行 Rust/Swift/Python 检查，在 arm64 macOS runner 构建并验证 DMG，为 ZIP/清单签名，最后发布附件齐全的稳定版 Release 并标记 latest。需要仓库 Actions Secret `SPARKLE_PRIVATE_KEY`，内容为既有 Sparkle 私钥的 base64 导出值。必须沿用已内嵌公钥对应的密钥，不能为 CI 新建签名身份，也不能将私钥写入源码。只能通过安全的 Secret 输入配置，不放入命令行参数或日志。

`ci_sign_update.py` 使用权限 0600 的临时文件将 Secret 导入临时 runner 钥匙串，由 `package_update.py` 检查公钥一致性，再删除文件和钥匙串条目。构建/测试步骤不接收密钥。`publish_release.py` 拒绝已有 Release 或旧于既有稳定版的版本，要求四项附件及正确校验和，先上传至草稿，检查附件大小后才发布。创建草稿后若失败，保留草稿供人工检查，重试不覆盖。发布串行执行。当前自动流程生成 ad-hoc 构建，不包含 Developer ID/公证；未配置 Secret 会在构建前失败。既有 `v26.9.101046` 标签早于此工作流，本次从本地发布，不移动标签来触发自动化。

工作流还支持在 main 上点击 **Run workflow**（`workflow_dispatch`）仅做验证：使用已配置的 Secret 构建、签名并检查完整附件，但不创建或修改 Release。只有推送 `v*` 标签才会实际发布。仓库签名 Secret 已经授权配置，应仅供可信的发布代码使用。
