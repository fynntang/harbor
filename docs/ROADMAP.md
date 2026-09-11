# Roadmap

English | [简体中文](zh-CN/ROADMAP.md) · [Documentation](README.md)

Harbor is released for macOS. The next milestone is Windows support, covering the app, CLI and installation workflow. This is planned work; the current macOS Intel build is not a Windows build.

## Next milestone: Windows

Start with Windows x64. Decide the minimum Windows version and whether to include ARM64 after checking the official client's available builds and data routing. No release date is set yet.

| Step | Work | Completion criteria |
|---|---|---|
| 1. Verify the official client | Inspect its installation layout, identity, update behavior and options for separate data directories. | On a Windows machine, two instances can sign in separately and retain their accounts after restarting, without changing the original account's data. Record any feature limitations. |
| 2. Adapt the core and CLI | Separate macOS/Unix-specific paths, permissions, app inspection and process operations from shared profile logic. Implement Windows creation, launch, stop, removal and copy updates. | Windows CLI tests pass. Each operation targets the selected instance; refusal or failure preserves its data. Closing a client directly allows its remaining helpers to be identified and cleaned up. |
| 3. Add the Windows GUI | Choose a Windows-capable UI implementation while retaining the existing macOS app. Provide instance management, tray controls, icons and English/Simplified Chinese switching. | Users can create and manage instances without the terminal. Closing the window keeps the tray entry; quitting Harbor leaves client instances running. |
| 4. Package and release | Add Windows CI, an installer and a verified Harbor update mechanism. Decide the signing and distribution approach. | Test installation, upgrade and uninstall on a clean Windows machine. Harbor updates preserve instance registrations and data; uninstall clearly states how account data is handled. Publish installation and recovery instructions. |

Start with the client compatibility check before choosing the GUI framework or installer format. If the official client cannot use separate data directories reliably, resolve that constraint before building the rest of the Windows workflow.

## Release checks

- Run real Windows tests for sign-in, restart, browser integration, residual helpers and data retention. Compilation and mocked tests alone do not complete this milestone.
- Keep the existing macOS build and tests passing. Windows work should reuse shared profile logic without changing macOS behavior unnecessarily.
- Record supported Windows versions, CPU architectures and client features in the release notes. Features not verified on Windows should be listed explicitly.

Implementation details remain in [Architecture](ARCHITECTURE.md); completed checks belong in [Testing](TESTING.md).
