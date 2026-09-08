# 文档目录

[English](../README.md) | 简体中文 · [项目 README](../../README.zh-CN.md)

Harbor 文档提供英文和简体中文。现有顶层文档路径继续可用，内容改为英文；每页直接链接到对应的中文版本。

| 主题 | English | 简体中文 |
|---|---|---|
| 概览与 CLI 快速开始 | [README](../../README.md) | [README](../../README.zh-CN.md) |
| macOS GUI、图标、删除与 JSON | [GUI](../GUI.md) | [GUI](GUI.md) |
| 实现、约束与核查结果 | [Architecture](../ARCHITECTURE.md) | [实现与边界](ARCHITECTURE.md) |
| 本轮检查、历史与手工验收 | [Testing](../TESTING.md) | [测试与证据](TESTING.md) |
| 参考与来源 | [References](../REFERENCES.md) | [参考与来源](REFERENCES.md) |

## 语言布局

```text
README.md
README.zh-CN.md
docs/
├── README.md
├── GUI.md
├── ARCHITECTURE.md
├── TESTING.md
├── REFERENCES.md
└── zh-CN/
    ├── README.md
    ├── GUI.md
    ├── ARCHITECTURE.md
    ├── TESTING.md
    └── REFERENCES.md
```

GUI 支持英文和简体中文，可即时切换并记住选择。CLI 帮助、底层诊断原文和系统控制的对话框保留原始/系统语言。

## 双语维护规则

1. 修改描述前检查对应的当前实现。文档不一致时先修正文档，记录重要行为差异，不通过悄悄实现另一项功能来满足旧描述。
2. 同一次改动更新两种语言。保持章节顺序、命令、路径、参数、响应字段、限制和验证状态等价。翻译解释文字，不翻译命令/API 标识。
3. 页首保留语言链接，主题链接尽量留在读者当前语言内，使用仓库相对路径。跨语言章节保留共同显式锚点，例如 `environment` 和 `removal-and-recovery`。
4. 区分源码推导行为、本轮复跑与历史实验。不要把旧截图、旧版本或个人路径变成当前要求或验收声明。
5. 修改后检查本地链接/锚点及代码块命令一致性。只有实际完成翻译后才增加对应语言目录/页面；仅添加语言链接不代表支持。

[2026-09-08 实现核查](ARCHITECTURE.md)记录了修正的不一致项。[测试记录](TESTING.md)说明本轮验证和未验证范围。不需要文档生成器或运行时本地化依赖。
