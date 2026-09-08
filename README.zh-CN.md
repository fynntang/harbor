# Harbor

[English](README.md) | 简体中文

Harbor 是用于创建、启动独立 ChatGPT/Codex 桌面实例的 macOS GUI 和 Rust CLI。它为每个实例指定独立的 Codex 与 GUI 数据目录。它**不是安全沙箱**：HOME、Keychain、SSH、Git、Docker、系统权限和项目访问仍然共享。

当前适配器检查已测试的 Chromium/Codex 应用结构，不支持所有名为 ChatGPT 的应用或任意 AI 客户端。本地副本使用 ad-hoc 签名，不保留厂商身份或公证。OAuth、Browser/Computer Use 和完整账号隔离不作保证。

[文档目录](docs/zh-CN/README.md) · [GUI 指南](docs/zh-CN/GUI.md) · [实现与边界](docs/zh-CN/ARCHITECTURE.md) · [测试记录](docs/zh-CN/TESTING.md) · [参考来源](docs/zh-CN/REFERENCES.md)

## 使用 GUI 开始

构建要求：macOS 14+、Rust 1.89+ 与 Cargo、Swift 6+ 与 macOS SDK/工具、Bash、Python 3。打包脚本使用 Python 3；打包后的应用不需要另行安装 Harbor CLI 或 Python 解释器。

先安装官方应用，然后在仓库根目录执行：

```bash
./scripts/build_and_run.sh
```

打开 `dist/Harbor.app`，选择**创建副本**，填写 `work`，选择官方来源应用并创建。点击**启动**后，在新客户端窗口内登录。创建副本不会自动启动或登录。

GUI 支持创建、启动、停止、删除、刷新状态、检查、实例图标和在 Finder 打开路径。关闭窗口会保留 Harbor 菜单栏入口；退出 Harbor 不会结束已启动的客户端。没有开机启动项或后台守护服务。可在工具栏语言选择器、Harbor 应用菜单或顶部菜单栏入口中切换 English / 简体中文，立即生效并在重启后保留。底层诊断原文和系统控制的对话框保留原始/系统语言。

应用内置 `harbor` 和 `harbor-native`，请保持整个 bundle 完整。Codex 的 Run 操作使用同一个构建脚本。

## 使用 CLI 开始

假设官方应用位于 `/Applications/ChatGPT.app`：

```bash
sh scripts/check.sh
cargo install --path crates/harbor-cli --locked --force
export PATH="$HOME/.cargo/bin:$PATH"

harbor clone work --source /Applications/ChatGPT.app
harbor start work --dry-run
harbor doctor work
harbor start work
harbor status work
```

`clone` 验证来源的 `com.openai.codex` 身份和 OpenAI Team ID `2DC432GLL2`，准备本地副本并创建空数据目录。名称或目标已存在时拒绝操作。自定义目标使用 `--app`，必须是不存在的绝对 `.app` 路径。

默认位置：

```text
~/Applications/Harbor/ChatGPT-work.app
~/Library/Application Support/Harbor/
├── registry.lock
├── profiles/
│   └── work/
│       ├── profile.json
│       ├── codex/
│       ├── gui/
│       └── launch.log
└── retained/                 # created when removal keeps data
```

副本 Bundle ID 为 `com.openai.codex.harbor.work`。`launch.log` 在启动时生成。`--root` 改变注册与数据根目录，不改变默认应用目标。实例名称为 1–48 个 ASCII 小写字母、数字或连字符，并以字母或数字开头。

## 登记已有副本

接管数据前先保存任务并退出选中的客户端。请把示例替换为实际准备好的应用和已有目录：

```bash
harbor adopt work \
  --app "/absolute/path/ChatGPT-Work.app" \
  --codex-home "/absolute/path/existing-codex" \
  --gui-home "/absolute/path/existing-gui"
```

`adopt` 原地记录引用，不复制凭据、不移动数据、不签名应用。两个数据目录必须存在，不能与 Profile 注册目录重叠。元数据和日志位于 `profiles/work/`，账号数据留在传入路径。旧实验目录不是使用前提。

已准备好的应用需要**新的空数据目录**时使用 `create`：

```bash
harbor create another --app "/absolute/path/ChatGPT-Another.app"
```

与 `clone` 不同，`create` 和 `adopt` 不准备应用、不验证厂商信任链；它们检查支持的结构、身份、版本字段，以及登记和数据路径冲突。每个已登记应用需要独立路径和 Bundle ID。默认账号目录被保留。

## 命令

| 命令 | 当前行为 |
|---|---|
| `clone` | 创建本地签名副本和空 Profile。 |
| `create` / `adopt` | 为准备好的应用登记新数据 / 已有数据。 |
| `list` / `show` | 列出登记 / 显示路由元数据。 |
| `start` | 校验身份与版本、指定数据路径、脱离终端启动并识别重复启动。 |
| `status` | 检查主进程命令行；JSON 还会报告残留辅助进程。 |
| `doctor` | 检查路径、身份、版本警告和本地签名，不执行修复。 |
| `stop` | 请求正常退出并处理支持的孤儿辅助进程；需要 `harbor-native`。 |
| `remove` | 将已停止的受管副本移到废纸篓，默认保留数据；需要 `harbor-native` 和 `--yes`。 |
| `icon` | 在停止状态下替换支持的本地副本图标，随后签名并验证。 |
| `logs` | 读取客户端启动日志尾部，内容可能敏感。 |
| `shortcut` | 新建 `.command` 文件，指向当前 CLI 的固定路径。 |

常用调用：

```bash
harbor list
harbor show work
harbor logs work -n 80
harbor shortcut work --output "$HOME/Desktop/ChatGPT Work.command"
harbor icon work --image /absolute/path/icon.png --tray-image /absolute/path/tray.png
```

CLI 图标输入必须是正方形 PNG，边长最多 4096 像素、文件最多 16 MiB。`--tray-image` 可选。GUI 还会转换 PNG/JPEG/ICNS，并提供首字母模板。详见[图标与生命周期操作](docs/zh-CN/GUI.md)。

仅执行 Rust `cargo install` **不会**安装 `harbor-native`。停止和删除请使用 GUI 或内置 CLI：

```bash
dist/Harbor.app/Contents/Helpers/harbor stop work
dist/Harbor.app/Contents/Helpers/harbor remove work --yes
```

先核对操作范围。只有希望将整个 Profile 目录一起移到废纸篓时才添加 `--delete-data`。否则数据保留在 `<root>/retained/<name>-<random>/profile`，GUI 会显示返回的路径。CLI `remove` 拒绝仍有进程运行的实例；GUI 删除总会先执行 `stop`，即使上次状态显示已停止。退出被拒绝或超时会阻止删除。没有永久删除或 GUI 恢复命令；[恢复说明](docs/zh-CN/GUI.md#removal-and-recovery)解释了保留清单仍记录原路径的问题。

## 路由、版本与边界

`start` 固定设置 `CODEX_HOME`、`CODEX_ELECTRON_USER_DATA_PATH`、`CODEX_SPARKLE_ENABLED=false` 和单个 `--user-data-dir` 参数。`--cwd` 默认是用户 HOME，不是调用时的仓库。环境变量采用允许名单；登记时可通过重复的 `--pass-env` 增加变量**名称**。值在启动时读取，不写入 `profile.json`。保留名称和注入敏感名称会被拒绝。详见[环境变量行为](docs/zh-CN/ARCHITECTURE.md#environment)。

`start --dry-run` 检查已登记路径并显示路由，不执行完整启动、版本、签名检查。`doctor` 把版本变化和旧清单缺少构建号作为警告，仅这些警告不会使命令失败。`start` 默认仍会阻止这些情况。审查版本变化后，可通过 CLI 单次允许：

```bash
harbor start work --accept-version-change
```

这是单次例外，不更新清单、不迁移数据库，也不绕过无效的当前版本字段或已改变的应用身份。GUI 不提供此例外。普通 `start` 不运行 `codesign`；需要检查签名时执行 `doctor`。

通过 Harbor 启动才能保留这些检查。clone 还设置了供 Launch Services 使用的 `LSEnvironment`，但双击副本 `.app` 会绕过 Harbor 校验和环境允许名单。`create`/`adopt` 不修改这些字段。

Harbor 不管理官方更新、不仲裁 OAuth URL 回调、不隔离全部 Skills/MCP 存储、不阻止访问其他项目，也不重写客户端配置覆盖。两个桌面端专用 `CODEX_*` 开关属于版本相关兼容控制。`remove` 可能整体移动账号目录；Harbor 不解析或复制凭据内容。

## 开发与文档

| 路径 | 职责 |
|---|---|
| `apps/Harbor/` | SwiftUI GUI、原生辅助程序和图标资源。 |
| `crates/harbor-cli/` | Cargo 包 `harbor-cli`；可执行命令 `harbor`；文本/JSON 接口。 |
| `crates/harbor-core/` | 登记、复制、路由、图标和生命周期逻辑。 |
| `scripts/check.sh` | Rust 格式检查、测试、Clippy 和 release 构建。 |
| `scripts/build_and_run.sh` | GUI 构建、bundle 签名和启动。 |
| `scripts/generate_app_icon.swift` | 生成透明帆船图标。 |
| `docs/` / `docs/zh-CN/` | 英文 / 简体中文文档。 |

```bash
cargo run -p harbor-cli -- --help
swift test --package-path apps/Harbor --scratch-path target/swift-harbor
```

CLI、核心包和 GUI 应用版本统一为 `0.0.1`。GUI 打包从 Cargo metadata 读取 CLI 包版本，macOS 独立构建号为 `1`。包元数据声明 MIT 许可。本地 ad-hoc 打包不等于经过公证的发行版。[测试记录](docs/zh-CN/TESTING.md)区分当前检查、历史客户端实验，以及待完成的账号/GUI 验收。`SOURCE_CHECKS.txt` 是历史快照，不是当前验收证据。

Developer ID 构建方式见[发布构建与签名](docs/zh-CN/GUI.md#release-build)。开发构建继续使用 ad-hoc 签名；已签名发布构建仍需完成公证后再正常分发。
