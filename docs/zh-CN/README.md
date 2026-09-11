# 文档目录

[English](../README.md) | 简体中文 · [项目 README](../../README.zh-CN.md)

选择下面的主题查看使用方法、实现说明或测试记录。各页顶部可切换英文和简体中文。

| 主题 | English | 简体中文 |
|---|---|---|
| 概览与 CLI 快速开始 | [README](../../README.md) | [README](../../README.zh-CN.md) |
| macOS GUI、图标、删除与 JSON | [GUI](../GUI.md) | [GUI](GUI.md) |
| Intel / Apple Silicon 安装包与发布 | [Release builds](../GUI.md#release-build) | [发布构建](GUI.md#release-build) |
| 下一阶段：Windows 支持 | [Roadmap](../ROADMAP.md) | [路线图](ROADMAP.md) |
| 实现与限制 | [Architecture](../ARCHITECTURE.md) | [实现与边界](ARCHITECTURE.md) |
| 测试记录与手工验收 | [Testing](../TESTING.md) | [测试与证据](TESTING.md) |
| 参考与来源 | [References](../REFERENCES.md) | [参考与来源](REFERENCES.md) |

## 语言布局

```text
README.md
README.zh-CN.md
docs/
├── README.md
├── GUI.md
├── ROADMAP.md
├── ARCHITECTURE.md
├── TESTING.md
├── REFERENCES.md
└── zh-CN/
    ├── README.md
    ├── GUI.md
    ├── ROADMAP.md
    ├── ARCHITECTURE.md
    ├── TESTING.md
    └── REFERENCES.md
```

GUI 支持英文和简体中文，可即时切换并记住选择。CLI 帮助、底层诊断原文和系统控制的对话框保留原始/系统语言。

## 双语维护

修改文档前先核对代码，同一次改动更新中英文。两种语言保持相同的命令、路径、参数和技术含义，并检查链接、锚点及代码块。

使用说明描述现有行为；带日期的测试结果放在[测试记录](TESTING.md)，不要将历史结果写成当前状态。
