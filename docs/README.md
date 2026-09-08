# Documentation

English | [简体中文](zh-CN/README.md) · [Project README](../README.md)

Harbor documentation is available in English and Simplified Chinese. Existing top-level documentation paths remain valid and now contain English; each page links directly to its Chinese counterpart.

| Topic | English | 简体中文 |
|---|---|---|
| Overview and CLI quick start | [README](../README.md) | [README](../README.zh-CN.md) |
| macOS GUI, icons, deletion and JSON | [GUI](GUI.md) | [GUI](zh-CN/GUI.md) |
| Implementation, constraints and audit findings | [Architecture](ARCHITECTURE.md) | [实现与边界](zh-CN/ARCHITECTURE.md) |
| Current checks, history and manual acceptance | [Testing](TESTING.md) | [测试与证据](zh-CN/TESTING.md) |
| References and provenance | [References](REFERENCES.md) | [参考与来源](zh-CN/REFERENCES.md) |

## Language layout

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

The GUI supports English and Simplified Chinese with an immediate, persistent language selector. CLI help, backend diagnostics and system-owned dialogs retain their original/system language.

## Maintaining both languages

1. Inspect the relevant current implementation before changing descriptions. When docs differ, correct the docs; record material behavior gaps instead of silently implementing a different feature.
2. Update both language pages in the same change. Keep section order, commands, paths, flags, response fields, limits and verification status equivalent. Translate explanatory prose, not command/API identifiers.
3. Keep language links at the top and topic links within the reader's language where possible. Use repository-relative links. Keep explicit shared anchors for cross-language sections such as `environment` and `removal-and-recovery`.
4. Separate source-derived behavior, freshly rerun checks and historical experiments. Do not turn old screenshots, version numbers or personal paths into current requirements or acceptance claims.
5. Check local links/anchors and matching fenced command blocks after edits. Add another locale as a matching directory/page set only when its content is actually translated; a language link alone is not support.

The [2026-09-08 implementation audit](ARCHITECTURE.md) records corrected mismatches. [Testing](TESTING.md) states what was and was not revalidated. No documentation generator or runtime localization dependency is required.
