"""First-launch instructions shared by release notes and the DMG readme."""

FIRST_LAUNCH = '''### 首次打开 / First launch

从本项目 GitHub Release 下载并核对 SHA256SUMS.txt，将应用放入 /Applications/Harbor.app。若提示无法验证开发者，先尝试打开一次，再到“系统设置 → 隐私与安全 → 仍要打开”。
Download from this project's GitHub Release, verify SHA256SUMS.txt and install at /Applications/Harbor.app. If the developer cannot be verified, try opening once, then use System Settings → Privacy & Security → Open Anyway.

若确认来源可信、校验和一致，且阻止原因是下载隔离标记，可在终端使用以下备选命令。它仅移除 Harbor 的隔离属性，使该应用不再触发基于此标记的首次下载检查；不会修复损坏文件或补充 Apple 公证。权限不足时才在命令前加 sudo。
If you trust the source, the checksum matches and download quarantine is the cause, use this optional Terminal command. It removes quarantine only from Harbor, bypassing the first-download check based on that flag; it does not repair damaged files or add Apple notarization. Prefix with sudo only if permission is denied.

```bash
xattr -dr com.apple.quarantine "/Applications/Harbor.app"
```

若系统明确提示恶意软件或会损害电脑，不要使用该命令绕过；校验和不一致时重新下载。无需全局关闭 Gatekeeper。
Do not use this command to bypass an explicit malware or “will damage your computer” alert. Redownload if the checksum differs. There is no need to disable Gatekeeper globally.

[Apple 首次打开说明 / Apple guidance](https://support.apple.com/en-us/102445)
'''
