# Harbor

[English](README.md) | 简体中文

Harbor 用来创建和管理 ChatGPT/Codex 桌面副本，让不同账号使用各自的数据目录。提供 macOS 图形界面和 Rust 命令行工具。

[文档目录](docs/zh-CN/README.md) · [GUI 指南](docs/zh-CN/GUI.md) · [实现与边界](docs/zh-CN/ARCHITECTURE.md) · [测试记录](docs/zh-CN/TESTING.md) · [参考来源](docs/zh-CN/REFERENCES.md)

<a href="https://www.nxgntools.com/tools/harbor-1?utm_source=harbor-1" target="_blank" rel="noopener" style="display: inline-block; width: auto;">
    <img src="https://www.nxgntools.com/api/embed/harbor-1?type=FEATURED_ON" alt="Featured on NxGn Tools" style="height: 48px; width: auto;" />
</a>

## 平台与安装包

Harbor 支持 macOS 14+ 的 Apple Silicon 和 Intel Mac。发布构建先生成 Universal 应用，再分别打包：

| Mac 类型 | DMG 文件名 |
|---|---|
| Apple Silicon（M 系列） | `Harbor-<version>-aarch64-apple-darwin.dmg` |
| Intel（x64 / x86_64） | `Harbor-<version>-x86_64-apple-darwin.dmg` |

两个架构共用 `Harbor-<version>-macos-universal.zip` 进行自动更新。Intel 副本要求官方客户端本身包含 Intel 架构，Harbor 不会将 ARM 客户端转换成 x86。当前验证包括 Rosetta 下的 Intel 测试，尚未完成 Intel 实机与官方 Intel 客户端验收。详见[测试记录](docs/zh-CN/TESTING.md)。

## 使用 GUI 开始

1. 从 [GitHub Releases](https://github.com/fynntang/harbor/releases) 下载适合你 Mac 的 DMG，打开后把 Harbor.app 拖入 Applications。
2. 安装官方 ChatGPT/Codex 客户端。
3. 打开 Harbor，点击**创建副本**，填写名称（如 `work`）并选择官方应用。
4. 创建完成后点击**启动**，在副本窗口中登录。

Harbor 使用 ad-hoc 签名，未经 Apple 公证。首次打开遇到开发者验证提示时，参见[首次打开说明](docs/zh-CN/GUI.md#first-launch)。应用内置 CLI 和原生辅助程序，无需另装 Rust 或 Python。

GUI 支持创建、更新、启动、停止和删除副本，也能检查状态、更换图标。图标可以使用官方原图加彩色名称角标。更新本机官方客户端后，Harbor 会提醒可更新的副本；Harbor 自身也支持[自动检查更新](docs/zh-CN/GUI.md#harbor-自动更新)，安装前需要确认。

关闭窗口后 Harbor 继续在菜单栏运行，退出 Harbor 不会结束副本。语言可在工具栏或菜单中切换为 English / 简体中文，选择会保存。底层诊断和系统对话框保留原始或系统语言。

### 从源码运行

需要 macOS 14+、Rust 1.89+ 与 Cargo、Swift 6+ 与 macOS SDK、Bash 和 Python 3。在仓库根目录执行：

```bash
./scripts/build_and_run.sh
```

脚本生成并打开 `dist/Harbor.app`，Codex 的 Run 操作也使用这个脚本。移动应用时请保留完整的 `.app` 目录。

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

副本 Bundle ID 为 `com.openai.codex.harbor.work`。`launch.log` 在启动时生成。`--root` 改变注册与数据根目录，不改变默认应用目标。显示名称为 1–48 个 Unicode 字符，支持大小写、中文、空格和括号，不允许控制字符或首尾空白。Harbor 自动生成独立、固定的小写标识：`Toobit` 转为 `toobit`；其他名称使用 ASCII 前缀（或 `profile`）加随机后缀。Bundle ID、应用文件名和数据目录均使用该标识。仅 ASCII 字母大小写不同的显示名称不能同时存在。`clone`/`create`/`adopt` 接收显示名称；后续 CLI 命令使用创建结果或 `list` 显示的标识（含空格的名称需加引号）。旧实例的标识和数据路径保持不变。

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
| `update` | 从官方原版更新已停止的托管副本，保留身份、图标和账号路径。 |
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
harbor update work --source /Applications/ChatGPT.app
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

Harbor 只分开 Codex 和 GUI 数据目录，不是安全沙箱。HOME、Keychain、SSH、Git、Docker、系统权限和项目访问仍然共享。

当前适配器检查已测试的 Chromium/Codex 应用结构，不支持所有名为 ChatGPT 的应用或任意 AI 客户端。本地副本使用 ad-hoc 签名，不保留厂商身份或公证。OAuth、Browser/Computer Use 和完整账号隔离不作保证。

`start` 固定设置 `CODEX_HOME`、`CODEX_ELECTRON_USER_DATA_PATH`、`CODEX_SPARKLE_ENABLED=false` 和单个 `--user-data-dir` 参数。`--cwd` 默认是用户 HOME，不是调用时的仓库。环境变量采用允许名单；登记时可通过重复的 `--pass-env` 增加变量**名称**。值在启动时读取，不写入 `profile.json`。保留名称和注入敏感名称会被拒绝。详见[环境变量行为](docs/zh-CN/ARCHITECTURE.md#environment)。

`start --dry-run` 检查已登记路径并显示路由，不执行完整启动、版本、签名检查。`doctor` 把版本变化和旧清单缺少构建号作为警告，仅这些警告不会使命令失败。`start` 默认仍会阻止这些情况。审查版本变化后，可通过 CLI 单次允许：

```bash
harbor start work --accept-version-change
```

这是单次例外，不更新清单、不迁移数据库，也不绕过无效的当前版本字段或已改变的应用身份。GUI 不提供此例外。普通 `start` 不运行 `codesign`；需要检查签名时执行 `doctor`。

通过 Harbor 启动才能保留这些检查。clone 还设置了供 Launch Services 使用的 `LSEnvironment`，但双击副本 `.app` 会绕过 Harbor 校验和环境允许名单。`create`/`adopt` 不修改这些字段。

Harbor 不管理官方更新、不仲裁 OAuth URL 回调、不隔离全部 Skills/MCP 存储、不阻止访问其他项目，也不重写客户端配置覆盖。两个桌面端专用 `CODEX_*` 开关属于版本相关兼容控制。`remove` 可能整体移动账号目录；Harbor 不解析或复制凭据内容。

## 下一阶段里程碑

下一阶段计划兼容 **Windows**，优先适配 x64，覆盖实例管理、图形界面、系统托盘、安装与更新。先验证官方客户端能否可靠使用独立数据目录，再推进核心适配和 GUI；Windows 版本范围与 ARM64 支持待评估。

当前支持 macOS，Windows 属于后续计划。具体步骤和完成标准见[路线图](docs/zh-CN/ROADMAP.md)。

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

Harbor 采用 MIT 许可，通过 GitHub Releases 发布更新，标签格式为 `vYY.M.DHHmm`。版本以 [Cargo.toml](Cargo.toml) 为准，GUI、CLI、构建号和附件名称保持一致。详见[发布流程](docs/zh-CN/GUI.md#release-build)。

[测试记录](docs/zh-CN/TESTING.md)按日期保留验证结果与待验证项目。`SOURCE_CHECKS.txt` 是早期开发快照。
