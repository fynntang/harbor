# 参考与来源

[English](../REFERENCES.md) | 简体中文 · [文档目录](README.md)

以下是实现 Harbor 时参考的文档和项目。具体行为见[架构说明](ARCHITECTURE.md)。

## 官方文档

| 来源 | 主题 |
|---|---|
| [Codex 环境变量](https://developers.openai.com/codex/config-file/environment-variables) | CODEX_HOME 与环境/配置边界。 |
| [Codex Skills](https://developers.openai.com/codex/skills/) | 多个 Skills 加载位置；分流 CODEX_HOME 不代表完整隔离 Skills。 |
| [Rust CommandExt](https://doc.rust-lang.org/std/os/unix/process/trait.CommandExt.html) | Unix pre_exec 与执行约束。 |
| [Rust File::try_lock](https://doc.rust-lang.org/std/fs/struct.File.html#method.try_lock) | 文件锁 API；仓库声明最低 Rust 版本为 1.89。 |
| [Apple flock 手册](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/flock.2.html) | 共享锁句柄、复制及进程继承。 |
| [Apple TN2206](https://developer.apple.com/library/archive/technotes/tn2206/_index.html) | 代码签名、嵌套代码与验证。 |
| [Disable Library Validation 权限](https://developer.apple.com/documentation/bundleresources/entitlements/com.apple.security.cs.disable-library-validation) | 保留厂商签名框架的副本在本地签名时使用的权限。 |
| [Launch Services keys](https://developer.apple.com/library/archive/documentation/General/Reference/InfoPlistKeyReference/Articles/LaunchServicesKeys.html) | LSEnvironment 和 Launch Services 启动。 |
| [NSRunningApplication](https://developer.apple.com/documentation/appkit/nsrunningapplication) | 应用身份、就绪、结束状态和正常退出请求。 |
| [FileManager](https://developer.apple.com/documentation/foundation/filemanager) | 原生废纸篓与文件移动操作。 |

原生退出处理还参考了本机 macOS SDK 头文件。Harbor 检查应用代码中的 `CODEX_ELECTRON_USER_DATA_PATH` 和 `CODEX_SPARKLE_ENABLED` 标记；标记检查和此前实验不意味着它们是稳定公共 API。

## 第三方设计参考

- [Doppel 仓库](https://github.com/thomast8/doppel)：历史上用于参考按 Profile 分流桌面应用及更新器控制。
- [Doppel engine](https://github.com/thomast8/doppel/blob/main/engine/doppel-engine.zsh)：指向可变分支，不是固定版本的兼容性规范。

Harbor 参考了 Doppel 的设计，使用自己的 Rust/Swift 实现，不打包其 engine 或厂商应用，也不修改 ASAR 或处理 OAuth 回调。本地签名会过滤依赖厂商身份的权限，保留嵌套框架签名，具体见 [clone.rs](../../crates/harbor-core/src/clone.rs)。

## 本机证据与资源

用户提供的 plist 字段、进程/窗口观察及此前 Python 启动实验用于选择适配器，不是可移植的兼容性证明。[测试记录](TESTING.md)区分历史客户端试验与当前本机检查。测试使用合成数据和临时可执行文件，账号文件与认证材料不作为夹具。

Harbor 自身图标由 [generate_app_icon.swift](../../scripts/generate_app_icon.swift) 使用 Apple 的 `sailboat` 系统符号生成，与菜单栏图形对应。它不同于核心图标测试使用的合成 PNG 夹具。用户截图不作为账号测试夹具。
