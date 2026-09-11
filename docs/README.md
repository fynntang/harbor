# Documentation

English | [简体中文](zh-CN/README.md) · [Project README](../README.md)

Choose a topic below for usage, implementation details or test records. Each page links to its English and Simplified Chinese versions.

| Topic | English | 简体中文 |
|---|---|---|
| Overview and CLI quick start | [README](../README.md) | [README](../README.zh-CN.md) |
| macOS GUI, icons, deletion and JSON | [GUI](GUI.md) | [GUI](zh-CN/GUI.md) |
| Intel / Apple Silicon installers and releases | [Release builds](GUI.md#release-build) | [发布构建](zh-CN/GUI.md#release-build) |
| Next milestone: Windows support | [Roadmap](ROADMAP.md) | [路线图](zh-CN/ROADMAP.md) |
| Implementation and limitations | [Architecture](ARCHITECTURE.md) | [实现与边界](zh-CN/ARCHITECTURE.md) |
| Test records and manual acceptance | [Testing](TESTING.md) | [测试与证据](zh-CN/TESTING.md) |
| References and provenance | [References](REFERENCES.md) | [参考与来源](zh-CN/REFERENCES.md) |

## Language layout

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

The GUI supports English and Simplified Chinese with an immediate, persistent language selector. CLI help, backend diagnostics and system-owned dialogs retain their original/system language.

## Maintaining both languages

Check the code before editing docs, and update both languages together. Keep commands, paths, flags and technical meaning equivalent. Check links, anchors and code blocks after editing.

Usage pages describe existing behavior. Keep dated results in [Testing](TESTING.md), rather than presenting historical results as current status.
