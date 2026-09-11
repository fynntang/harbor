# 实现与边界

[English](../ARCHITECTURE.md) | 简体中文 · [文档目录](README.md)

Harbor 使用 Swift 实现 macOS 界面与原生操作，使用 Rust 管理实例。本页说明数据路径、进程检查和失败处理。

## 架构与分发

开发构建使用本机架构；发布构建通过 Swift 双架构编译和 Rust `aarch64-apple-darwin` / `x86_64-apple-darwin` 编译、合并，生成 Universal GUI、CLI 和原生辅助程序。发布签名检查要求三个程序均包含 arm64 与 x86_64。

DMG 打包复制 Universal 应用，提取三个程序的目标架构并重新 ad-hoc 签名，再校验挂载内容。Sparkle 框架保留 Universal。两份 DMG 使用 `aarch64-apple-darwin` 和 `x86_64-apple-darwin` 后缀；自动更新 ZIP 保留 Universal，因此两个架构共用现有签名更新地址。发布要求两个 DMG、ZIP、appcast 和校验和共五项附件。

浏览器辅助进程仅识别实例插件目录内 `macos/arm64`、`macos/x64` 或 `macos/x86_64` 下的已知 host；其他身份、所有者和进程检查不变。Harbor 的双架构构建不会转换或保证官方客户端的 Intel 兼容性。

## 组件

| 组件 | 职责 |
|---|---|
| [SwiftUI GUI](../../apps/Harbor/Sources/Harbor/) | 视图、操作顺序、图像转换、菜单/窗口生命周期。 |
| [原生辅助程序](../../apps/Harbor/Sources/HarborNative/) | NSRunningApplication 退出请求与 FileManager 废纸篓操作。 |
| [Rust CLI](../../crates/harbor-cli/src/main.rs) | Clap 命令、文本诊断与 JSON 传输。 |
| [核心库](../../crates/harbor-core/src/) | 路径/身份策略、登记、复制、启动、图标、停止/删除。 |

GUI 调用内置 CLI，不重新实现核心策略。原生 helper 从 CLI 可执行文件的同目录查找，单独安装 Rust CLI 不包含它。没有服务器或持续运行的监督进程。

## 登记与路径

[AppInfo](../../crates/harbor-core/src/app.rs) 解析 XML/binary Info.plist，要求可执行名为 `ChatGPT` 或 `Codex`、具备 Chromium 标记（`CrProductDirName` 或 `ElectronAsarIntegrity`）、ID/版本/构建号字符串非空，且可执行文件解析后仍在 bundle 内。这是结构检查，不证明兼容性或厂商信任。

[Store](../../crates/harbor-core/src/store.rs) 默认位于 `~/Library/Application Support/Harbor`。打开 Store 时会按需创建注册根和 `profiles` 目录并设置权限，`list` 等偏读取的命令也一样。保护检查先于这些写入。`--root` 必须为绝对路径、不能含 `..`，不得与 HOME、根目录、默认账号路径或其解析别名重叠。保留的默认数据路径为：

```text
~/.codex
~/Library/Application Support/com.openai.codex
~/Library/Application Support/Codex
~/Library/Application Support/ChatGPT
```

即使磁盘区分大小写，也会保守保留 ASCII 大小写变体。实际路径必须仍解析到登记的位置。Profile 清单使用 schema 版本 `1`，拒绝未知字段。旧清单可以缺少登记构建号，但正常启动随后需要明确审查/例外。

清单中的 `name` 是固定注册标识，可选字段 `display_name` 保存面向用户的 Unicode 文本；缺少该字段时回退为 `name`，兼容旧实例。新登记生成小写标识，沿用原 ASCII 语法或使用 ASCII 前缀加 64 位随机后缀。显示文本不参与路径或 Bundle ID。标识和显示名称都保留 ASCII 大小写变体；clone 的 Bundle ID 为 `com.openai.codex.harbor.<identifier>`。`create`/`adopt` 登记的已准备应用保留原 Bundle ID。

Profile 记录应用/可执行路径/ID、短版本/构建号、两个数据路径、工作目录、额外环境变量名称和 `adopted_data`，不存环境值。Codex/GUI 路径不能互相重叠或与应用重叠；其他 Profile 不能复用相同应用路径/ID 或冲突的数据。`adopt` 要求数据已存在且位于 `profiles` 注册目录外；`create` 为准备好的应用分配空数据。两者均不签名或修改应用。

新 Harbor 目录/文件使用私有权限（通常为 0700/0600，可执行快捷入口为 0700），不递归 chmod 既有账号树。登记使用私有锁和不覆盖式清单提交。`RegistryLock` 在 drop 中先显式解锁再关闭句柄，避免并发继承的句柄延长锁持有时间。`create`/`adopt` 初始化不完整时保留现场，不递归删除。

## 复制与签名策略

[clone.rs](../../crates/harbor-core/src/clone.rs) 在准备和发布期间一直持有注册锁：

1. 校验新名称和目标，拒绝已有目标、来源/registry/默认数据重叠及已有 Profile 冲突。
2. 对原版 `com.openai.codex` 验证包含 Apple anchor 和 OpenAI Team ID `2DC432GLL2` 的签名要求。
3. 在目标父目录暂存，先尝试 `cp -cR`，失败回退 `ditto`。复制后重新检查厂商签名及版本/构建号未变化。
4. 拒绝不支持的签名路径符号链接，在 `app.asar` 中查找桌面数据/更新器控制字符串。存在标记只是兼容性保护，不是执行证明。
5. 修改外层 Info.plist 的身份/显示/产品字段和 `LSEnvironment` 路由。过滤依赖厂商身份的受限权限，未知权限直接拒绝。使用 ad-hoc、hardened runtime、支持的权限及 `disable-library-validation` 签名外层副本。
6. 验证本地签名，以排他 rename 先发布新应用，再发布暂存的空 Profile。Profile 发布失败时尝试仅移除本次新发布应用；清理失败会报告。

不重写嵌套厂商代码/签名、ASAR 或 URL scheme，不禁用隔离属性或 Gatekeeper。产物不保留厂商身份或公证。应用和 registry 的发布不是一个跨文件系统事务，中断可能留下暂存内容或未登记应用；使用 `create` 登记准备好的副本前应先检查现场。

## 启动与版本

[process.rs](../../crates/harbor-core/src/process.rs) 取得注册锁，重新验证路径、当前身份和版本，再比较当前进程命令行。恰好一个可执行路径匹配、GUI 参数完全符合预期的进程会被复用。相同可执行文件的其他参数被视为冲突。这种比较不是经过认证的身份或账号检查。

新启动时清空/重建环境，设置工作目录，stdin 指向 `/dev/null`，stdout/stderr 追加到 `launch.log`，并在 `pre_exec` 中用 `setsid` 新建会话。观察两秒内是否早退，包括退出码 0。通过该时间窗不证明窗口、登录或数据隔离正常。之后没有监督进程持续监控客户端。

身份变化、当前版本/构建号无效始终阻止启动。短版本/构建号变化或旧登记缺少构建号，会阻止启动，除非该次调用带 `--accept-version-change`。例外不更新清单、不验证版本一定更新，也不迁移数据库。`start` 不执行签名验证，`doctor` 执行。`start --dry-run` 仅校验登记路径和显示路由，不做完整 AppInfo/版本/签名/运行时检查。

<a id="environment"></a>
## 环境变量

[environment.rs](../../crates/harbor-core/src/environment.rs) 保留真实 HOME，并明确设置：

```text
HOME=<real home>
PWD=<registered working directory>
CODEX_HOME=<profile codex directory>
CODEX_ELECTRON_USER_DATA_PATH=<profile gui directory>
CODEX_SPARKLE_ENABLED=false
argv: --user-data-dir=<profile gui directory>
```

默认工作目录为 HOME。允许名单包含 PATH、locale 及常用 SSH/Docker/工具链/代理变量。PATH 缺失时使用固定的常用 macOS 工具路径。默认不转发 API key 和任意 MCP token。登记时重复 `--pass-env NAME` 可增加名称，其当前值在启动时读取。

禁止覆盖 HOME/PWD/OLDPWD、BASH_ENV/ENV/NODE_OPTIONS，以及 `CODEX_`、`HARBOR_`、`DYLD_`、`ELECTRON_` 前缀。这不会阻止客户端、工具或登录 shell 从共享 HOME 读取凭据/配置。代理和 Docker 值本身可能敏感，诊断不打印这些值；客户端生成的日志仍可能含敏感内容。

clone 同时为 Launch Services 修改 `LSEnvironment`，直接执行主程序时则明确传入环境。通过 Finder 启动 clone 会绕过 Harbor 检查和允许名单。adopt/create 不更新 LSEnvironment。桌面端专用的数据/更新器开关依赖具体版本，不是稳定 API 或绝对禁更新保障。

## 停止、删除与图标

[lifecycle.rs](../../crates/harbor-core/src/lifecycle.rs) 重新检查路径和应用身份，要求原生 helper，并持有注册锁。主程序正常退出使用 NSRunningApplication 身份核对，就绪/请求/退出共用 15 秒时限，之后最多再花五秒处理辅助进程。

[auxiliary.rs](../../crates/harbor-core/src/auxiliary.rs) 扫描应用和两个数据目录下的可执行路径。已知 Crashpad、Computer Use 和快捷键监视孤儿进程，仅在 UID 为当前用户、父 PID 为 1，且重新核对可执行路径/属主/父进程/启动时间后，才可能接收 SIGTERM。不回退到 SIGKILL。这些进程表和 libproc 检查减少误操作，不是抵御同用户恶意进程的原子保护。

浏览器定向清理：主应用退出后，可向该实例插件缓存中已识别路径的 Chrome native host 发送 SIGTERM，即使其浏览器父进程仍在运行。精确路径 `.plugin-appserver/codex` 仅在已孤立或父进程是同实例 native host 时可清理，先处理 host。保留浏览器、其他实例以及无关活跃进程，并在发信号前重验进程身份。

停止托管副本时，先将 Brave、Chrome 和 Edge 中指向该实例的 native messaging 注册备份到 Profile 目录的 `browser-registrations`，并移除原注册，再请求辅助进程退出。启动时仅恢复到空注册位置，保留其他实例的注册，避免浏览器自动重连且无需退出浏览器。


删除要求扫描结果无进程、`adopted_data=false`、Harbor 风格 ID 和精确的专属数据路径，并检查其他实例与保留目录重叠。默认先排他移动 Profile 到私有 `retained` 目录，再把应用移到废纸篓；明确删除数据时才把应用和 Profile 一起移走。错误会尝试不覆盖式还原，中断或回滚失败可能需要人工恢复。代码不会清空废纸篓。整体移动账号目录与解析/复制凭据内容不同。详见[恢复说明](GUI.md#removal-and-recovery)。

[icon.rs](../../crates/harbor-core/src/icon.rs) 额外要求登记版本/构建号和已知图标结构，使用 `cp -cR` 暂存、签名/验证后原子交换。暂存和交换前只检查主可执行程序，不使用完整辅助进程扫描。GUI 要求 JSON 状态为 `stopped`，因而也会阻止残留进程状态。图标/删除资格由清单和路径检查决定，没有独立的 clone 来源证明。生命周期 stop/remove 不像 icon/start 那样要求版本相等。

## 诊断与限制

文本 `status` 和 `doctor` 的进程部分只检查主进程。JSON status/list 能报告 `helpers_running`，但 `pids` 仍只列主进程。均不读取登录身份或验证运行进程环境。路径/元数据/身份/签名错误会导致 `doctor` 失败，但仅版本差异、缺少旧构建号和已报告的启动冲突不会设置诊断失败标记。JSON doctor 对已完成但失败的诊断返回 `ok=true`、退出 0、`data.passed=false`；Store 初始化错误仍会使 envelope 失败。

系统不隔离 HOME、Keychain、项目、系统权限或全部 Skills/MCP 存储。不实现 OAuth 回调路由、自动官方更新、账号/数据库迁移或任意客户端支持，不能阻止客户端遵循配置访问被分配数据目录以外的路径。

## 版本来源

版本由 [Cargo.toml](../../Cargo.toml) 定义。[构建脚本](../../scripts/build_and_run.sh) 读取 Cargo metadata，设置 GUI 版本和构建号。CLI、GUI 与 Sparkle 使用同一数字版本。

## GUI 语言状态

GUI 在 `HarborStore` 中持有一个可观察的 `GUILocalization`，由 `UserDefaults` 保存语言选择。`GUIMessage` 保留文案模板和独立插值参数，语言变化时重新渲染已有进度、成功提示和 GUI 错误。底层错误明确保留原文。编译内置文案表与语言选择器分别位于 `apps/Harbor/Sources/Harbor/Localization/GUILocalization.swift` 和 `apps/Harbor/Sources/Harbor/Views/LanguagePicker.swift`，不改变 CLI 协议或实例存储。

## 托管副本更新

`update` 持有注册锁，验证现有身份/版本及停止状态，再以原 Profile 身份、新版本快照准备通过厂商签名验证的替换应用。在暂存前和发布前检查应用及数据目录中的全部辅助进程。自定义图标在签名前沿用。原子交换应用、校验最终签名后，用已刷盘的临时清单原子替换 `profile.json`。清单发布前失败会换回旧应用；回滚失败保留暂存并报告路径。应用与清单不是单个文件系统事务：两次写入之间异常退出会留下可检测的版本不匹配，需要检查恢复。不会备份或回滚账号数据库。
