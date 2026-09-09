import AppKit
import Observation

@MainActor @Observable
final class HarborStore {
  let language: GUILocalization
  func text(_ message: GUIMessage) -> String { language.text(message) }
  func text(key: String) -> String { language.text(GUIMessage(key: key)) }

  let client: HarborClient
  var profiles: [ProfileItem] = []
  var selection: String?
  var registry = ""
  var busy = false
  var activity: GUIMessage = ""
  var message: GUIMessage = ""
  var error: GUIMessage?
  var report: CheckReport?
  var creating = false
  var iconRevision = 0
  var removalNotice: GUIMessage?
  var retainedData: String?

  init(client: HarborClient, language: GUILocalization = GUILocalization()) {
    self.client = client
    self.language = language
  }
  var selected: ProfileItem? { profiles.first { $0.id == selection } }

  private func load() async throws {
    let result: ProfileList = try await client.request(["list"])
    profiles = result.profiles
    registry = result.root
    if !profiles.contains(where: { $0.id == selection }) { selection = profiles.first?.id }
  }

  func refresh() async {
    guard !busy else { return }
    busy = true
    activity = "正在读取实例…"
    if let retainedData, !FileManager.default.fileExists(atPath: retainedData) {
      dismissRemovalNotice()
    }
    defer { busy = false }
    do {
      try await load()
      error = nil
    } catch { self.error = GUIMessage(error: error) }
  }

  func create(name: String, source: String, iconPNG: Data? = nil, trayPNG: Data? = nil) async {
    guard !busy, validProfileName(name) else { return }
    busy = true
    activity = "正在准备…"
    error = nil
    message = ""
    report = nil
    defer { busy = false }
    do {
      let result: CreatedProfile = try await client.request(["clone", "--source", source, "--", name]) {
        [weak self] stage in
        Task { @MainActor in self?.activity = Self.stageTitle(stage) }
      }
      creating = false
      selection = result.profile.name
      message = "副本已创建。点击“启动”，在新窗口登录工作账号。"
      try await load()
      if let iconPNG, let trayPNG {
        activity = "正在准备图标并验证副本签名…"
        try await applyIcon(name: result.profile.name, png: iconPNG, trayPNG: trayPNG)
        iconRevision += 1
      }
    } catch { self.error = GUIMessage(error: error) }
  }

  func start() async {
    guard !busy, let selected else { return }
    busy = true
    activity = "正在启动并检查进程…"
    error = nil
    report = nil
    defer { busy = false }
    do {
      let result: StartedProfile = try await client.request(["start", selected.id])
      message =
        result.state == "already_running" ? "该实例已在运行，没有重复启动。" : "启动成功，请在副本窗口继续（进程 \(result.pid)）。"
      try await load()
    } catch {
      self.error = GUIMessage(error: error)
      message = ""
    }
  }

  func stop() async {
    guard !busy, let selected else { return }
    busy = true
    activity = "正在请求正常退出…"
    error = nil
    message = ""
    report = nil
    defer { busy = false }
    do {
      let result: StoppedProfile = try await client.request(["stop", selected.id])
      guard ["stopped", "already_stopped"].contains(result.state) else {
        throw CLIError(message: "无法确认实例已停止，请检查后重试。")
      }
      message = "实例已停止，账号数据保留。"
      try await load()
    } catch { self.error = GUIMessage(error: error) }
  }

  func remove(item: ProfileItem, deleteData: Bool) async -> Bool {
    guard !busy else { return false }
    busy = true
    activity = "正在检查删除条件…"
    error = nil
    message = ""
    report = nil
    removalNotice = nil
    retainedData = nil
    defer { busy = false }
    do {
      do {
        activity = "正在请求正常退出…"
        let stopped: StoppedProfile = try await client.request(["stop", item.id])
        guard ["stopped", "already_stopped"].contains(stopped.state) else {
          throw CLIError(message: "无法确认实例已停止，未执行删除。")
        }
      }
      activity = "正在移到废纸篓…"
      var args = ["remove", item.id, "--yes"]
      if deleteData { args.append("--delete-data") }
      let result: RemovedProfile = try await client.request(args)
      retainedData = result.retained_data
      removalNotice = result.retained_data == nil ? "实例和账号数据已移到废纸篓。" : "实例已删除，账号数据已保留。"
      try await load()
      return true
    } catch {
      self.error = GUIMessage(error: error)
      return false
    }
  }

  func check() async {
    guard !busy, let selected else { return }
    busy = true
    activity = "正在检查路径、签名与进程…"
    error = nil
    message = ""
    report = nil
    defer { busy = false }
    do {
      report = try await client.request(["doctor", selected.id])
      try await load()
    } catch { self.error = GUIMessage(error: error) }
  }

  func changeIcon(item: ProfileItem, png: Data, trayPNG: Data) async -> Bool {
    guard !busy else { return false }
    busy = true
    error = nil
    message = ""
    report = nil
    activity = "正在准备图标并验证副本签名…"
    defer { busy = false }
    do {
      try await applyIcon(name: item.id, png: png, trayPNG: trayPNG)
      iconRevision += 1
      message = "图标已更新，重新启动该实例后生效。Dock 中缓存的旧图标可能需要重新固定。"
      try await load()
      return true
    } catch {
      self.error = GUIMessage(error: error)
      return false
    }
  }

  private func applyIcon(name: String, png: Data, trayPNG: Data) async throws {
    let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    try FileManager.default.createDirectory(
      at: directory, withIntermediateDirectories: false,
      attributes: [.posixPermissions: 0o700])
    defer { try? FileManager.default.removeItem(at: directory) }
    let image = directory.appendingPathComponent("icon.png")
    try png.write(to: image, options: .atomic)
    let tray = directory.appendingPathComponent("tray.png")
    try trayPNG.write(to: tray, options: .atomic)
    let result: IconResult = try await client.request([
      "icon", name, "--image", image.path, "--tray-image", tray.path,
    ])
    guard result.updated else { throw CLIError(message: "图标未更新，请重试。") }
  }

  func dismissRemovalNotice() {
    removalNotice = nil
    retainedData = nil
  }

  func reveal(_ path: String) {
    guard FileManager.default.fileExists(atPath: path) else {
      if path == retainedData { dismissRemovalNotice() }
      error = "路径不存在：\(path)"
      return
    }
    NSWorkspace.shared.activateFileViewerSelecting([URL(fileURLWithPath: path)])
  }

  static func stageTitle(_ stage: String) -> GUIMessage {
    switch stage {
    case "verify_source": "正在验证原版签名…"
    case "copy": "正在复制应用…"
    case "verify_copy": "正在验证副本完整性…"
    case "sign": "正在为本地副本签名…"
    case "verify_signature": "正在复验副本签名…"
    case "complete": "正在读取新实例…"
    default: "正在准备…"
    }
  }
}
