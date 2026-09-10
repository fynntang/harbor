import Foundation
import Observation

/// Keep templates and arguments separate so saved results can change language in place.
struct GUIMessage: ExpressibleByStringLiteral, ExpressibleByStringInterpolation, Equatable, Sendable
{
  let key: String
  let arguments: [String]
  let verbatim: Bool

  init(stringLiteral value: String) { self.init(key: value) }
  init(key: String) {
    self.key = key
    arguments = []
    verbatim = false
  }
  init(verbatim value: String) {
    key = value
    arguments = []
    verbatim = true
  }
  init(error: Error) {
    self = (error as? CLIError)?.message ?? GUIMessage(verbatim: error.localizedDescription)
  }
  init(stringInterpolation: StringInterpolation) {
    key = stringInterpolation.key
    arguments = stringInterpolation.arguments
    verbatim = false
  }
  var isEmpty: Bool { key.isEmpty }
  func rendered(language: GUILanguage) -> String {
    let template = !verbatim && language == .english ? GUILocalization.english[key] ?? key : key
    guard !verbatim, !arguments.isEmpty else { return template }
    // Substitution never interprets percent signs or placeholder text inside user values.
    let parts = template.components(separatedBy: "%@")
    guard parts.count == arguments.count + 1 else { return key }
    return zip(parts.dropLast(), arguments).map { $0 + $1 }.joined() + parts.last!
  }
  struct StringInterpolation: StringInterpolationProtocol {
    var key = ""
    var arguments: [String] = []
    init(literalCapacity: Int, interpolationCount: Int) {}
    mutating func appendLiteral(_ literal: String) { key += literal }
    mutating func appendInterpolation<T>(_ value: T) {
      key += "%@"
      arguments.append(String(describing: value))
    }
  }
}

enum GUILanguage: String, CaseIterable, Identifiable, Sendable {
  case english = "en"
  case simplifiedChinese = "zh-Hans"
  var id: String { rawValue }
  var title: String { self == .english ? "English" : "简体中文" }
  var locale: Locale { Locale(identifier: rawValue) }
  static func initial(saved: String?, preferred: [String]) -> Self {
    if let saved, let language = Self(rawValue: saved) { return language }
    for language in preferred {
      let code = language.lowercased()
      if code == "en" || code.hasPrefix("en-") { return .english }
      if code == "zh" || code.hasPrefix("zh-") { return .simplifiedChinese }
    }
    return .english
  }
}

@MainActor @Observable
final class GUILocalization {
  static let preferenceKey = "HarborGUILanguage"
  private let defaults: UserDefaults
  var selection: GUILanguage {
    didSet { defaults.set(selection.rawValue, forKey: Self.preferenceKey) }
  }
  init(defaults: UserDefaults = .standard, preferred: [String] = Locale.preferredLanguages) {
    self.defaults = defaults
    selection = .initial(saved: defaults.string(forKey: Self.preferenceKey), preferred: preferred)
  }
  func text(_ message: GUIMessage) -> String { message.rendered(language: selection) }

  // Compiled into the executable, including SwiftPM builds and the packaged app.
  nonisolated static let english: [String: String] = [
    "%@ 个实例": "Instances: %@",
    "1–48 个字符，支持中文、大小写、空格和括号；首尾不能有空白。":
      "1–48 characters, including Chinese, uppercase/lowercase letters, spaces and parentheses; no surrounding whitespace.",
    "Harbor 应用缺少内置命令行组件，请重新构建或安装应用。":
      "The bundled Harbor CLI is missing. Rebuild or reinstall Harbor.",
    "Harbor 组件返回了无法识别的结果（退出码 %@），请重新构建应用。":
      "Harbor returned an unrecognized response (exit code %@). Rebuild the app.",
    "PNG、JPEG 或 ICNS，最大 16 MB。\n非正方形图片会保留比例并补透明边。":
      "PNG, JPEG or ICNS, up to 16 MB.\nNon-square images keep their proportions with transparent padding.",
    "仅支持 Harbor 创建的本地副本": "Only local copies created by Harbor are supported",
    "仅支持删除 Harbor 创建的本地副本；账号数据默认保留":
      "Only local copies created by Harbor can be removed; account data is kept by default",
    "从一个新副本开始": "Start with a new copy",
    "从官方原版创建一个新实例，首次启动后登录自己的账号。":
      "Create an instance from the official app, then sign in with your account after launching it.",
    "以下目录包含登录状态、对话和设置；清空废纸篓后将无法通过 Harbor 恢复。":
      "These folders contain sign-in state, conversations and settings. Harbor cannot recover them after the Trash is emptied.",
    "会先请求正常退出。退出被取消、超时或仍有辅助进程运行时，不会删除。":
      "Harbor will request a normal quit first. Removal stops if quitting is cancelled, times out or leaves helpers running.",
    "例如 work": "e.g. work",
    "清理残留": "Clean Up Helpers",
    "副本已退出，但辅助进程仍在运行。清理残留会暂停该实例的浏览器连接并请求已知辅助进程退出，不会退出浏览器；通过 Harbor 启动副本时恢复连接注册。": "The copy has quit, but helpers are still running. Cleanup pauses this instance’s browser integration and asks known helpers to exit without quitting the browser. Launching the copy through Harbor restores its connection registration.",
    "检查 Harbor 更新…": "Check for Harbor Updates…",
    "自动检查 Harbor 更新": "Automatically Check for Harbor Updates",
    "开发版未启用自动更新": "Automatic updates are disabled in development builds",
    "请等当前操作完成后再更新 Harbor。": "Wait for the current operation to finish before updating Harbor.",
    "停止": "Stop",
    "停止并删除": "Stop and Remove",
    "创建副本": "Create Copy",
    "创建副本…": "Create Copy…",
    "创建独立副本": "Create an Independent Copy",
    "创建第一个副本": "Create Your First Copy",
    "删除实例 %@？": "Remove instance %@?",
    "删除实例…": "Remove Instance…",
    "刷新": "Refresh",
    "刷新实例": "Refresh Instances",
    "副本位置": "App copy",
    "副本已创建。点击“启动”，在新窗口登录工作账号。":
      "Copy created. Click Start, then sign in with your work account in the new window.",
    "取消": "Cancel",
    "同时将账号数据移到废纸篓": "Also move account data to the Trash",
    "启动": "Start",
    "启动冲突": "Launch conflict",
    "启动成功，请在副本窗口继续（进程 %@）。": "Started successfully. Continue in the copy's window (process %@).",
    "图标已更新，重新启动该实例后生效。Dock 中缓存的旧图标可能需要重新固定。":
      "Icon updated. Restart the instance to apply it. You may need to re-pin its Dock item to clear the cached icon.",
    "图标未更新，请重试。": "The icon was not updated. Try again.",
    "在 Finder 中显示%@": "Show %@ in Finder",
    "在 Finder 中显示日志目录": "Show Log Folder in Finder",
    "官方原版": "Official app",
    "实例": "Instances",
    "实例标识": "Instance Identifier",
    "将自动生成小写实例标识，用于 Bundle ID 和数据目录。": "A lowercase identifier will be generated for the Bundle ID and data directories.",
    "实例名称": "Instance name",
    "实例和账号数据已移到废纸篓。": "The instance and account data were moved to the Trash.",
    "实例已停止，账号数据保留。": "The instance has stopped. Account data is preserved.",
    "实例已删除，账号数据已保留。": "The instance was removed. Account data is preserved.",
    "实例首字母": "Instance initial",
    "应用": "Application",
    "应用到 Finder、Dock 和顶部菜单栏。请先退出该实例，更换后重新启动生效。":
      "Applies to Finder, the Dock and the menu bar. Quit the instance first, then restart it after changing the icon.",
    "应用副本会移到系统废纸篓，并从 Harbor 列表中移除。":
      "The app copy will be moved to the Trash and removed from Harbor's list.",
    "应用图标": "Apply Icon",
    "当前使用临时数据目录，正式使用前请将实例迁移到持久目录。":
      "This data folder is temporary. Move your instances to a persistent folder before regular use.",
    "当前版本": "Current version",
    "所选图片轮廓": "Selected image silhouette",
    "打开 Harbor": "Open Harbor",
    "操作仍在进行": "Operation in Progress",
    "操作未完成": "Operation Incomplete",
    "操作未完成（退出码 %@）。": "The operation did not complete (exit code %@).",
    "新副本使用本地签名，数据目录为空。现有副本和账号不会被覆盖。":
      "The new copy uses a local signature and empty data folders. Existing copies and accounts are preserved.",
    "无法生成图标预览，请换一张图片。": "Could not create an icon preview. Choose another image.",
    "无法生成菜单栏图标。": "Could not create the menu-bar icon.",
    "无法确认实例已停止，未执行删除。": "Could not confirm that the instance stopped. Nothing was removed.",
    "无法确认实例已停止，请检查后重试。": "Could not confirm that the instance stopped. Check it and try again.",
    "无法读取": "Unavailable",
    "更换 %@ 的图标": "Change Icon for %@",
    "更换图标…": "Change Icon…",
    "未运行": "Stopped",
    "未知构建号": "unknown build",
    "未记录构建号": "build not recorded",
    "更新副本 %@": "Update copy %@",
    "更新副本…": "Update Copy…",
    "更新副本": "Update Copy",
    "来源版本": "Source Version",
    "正在准备更新…": "Preparing update…",
    "请先退出副本及其辅助进程，再从官方原版更新": "Quit the copy and its helpers before updating from the official app",
    "保留账号数据、名称和图标。请先退出副本及浏览器辅助进程；更新完成后不会自动启动。":
      "Keep account data, name and icons. Quit the copy and browser helpers first; the updated copy will not start automatically.",
    "更新只替换应用程序，不备份账号数据库。新版首次启动可能迁移数据，不支持自动降级。":
      "Only the app is replaced; account databases are not backed up. The new app may migrate data on first launch. Automatic downgrade is not supported.",
    "副本已更新，账号目录保持不变。点击启动后使用新版。":
      "The copy is up to date and account paths are unchanged. Click Start to use it.",
    "关闭提示": "Dismiss notice",
    "查看保留的数据": "Show Retained Data",
    "检查": "Check",
    "检查发现问题": "Issues Found",
    "检查完成": "Check Complete",
    "正在为本地副本签名…": "Signing the local copy…",
    "正在准备…": "Preparing…",
    "正在准备图标并验证副本签名…": "Preparing icons and verifying the copy's signature…",
    "正在启动并检查进程…": "Starting and checking processes…",
    "正在复制应用…": "Copying the app…",
    "正在复验副本签名…": "Rechecking the copy's signature…",
    "正在检查删除条件…": "Checking removal requirements…",
    "正在检查路径、签名与进程…": "Checking paths, signatures and processes…",
    "正在移到废纸篓…": "Moving to the Trash…",
    "正在请求正常退出…": "Requesting a normal quit…",
    "正在读取实例…": "Loading instances…",
    "正在读取新实例…": "Loading the new instance…",
    "正在验证副本完整性…": "Verifying the copy's integrity…",
    "正在验证原版签名…": "Verifying the official app's signature…",
    "状态未知": "Unknown status",
    "独立数据目录": "Independent Data Folders",
    "界面与 Harbor 组件版本不匹配，请重新构建应用。": "The GUI and Harbor CLI versions do not match. Rebuild the app.",
    "界面数据": "GUI data",
    "登记版本": "Registered version",
    "移到废纸篓": "Move to Trash",
    "继续等待": "Keep Waiting",
    "菜单栏": "Menu bar",
    "该实例已在运行，没有重复启动。": "This instance is already running. No duplicate was started.",
    "请先退出该实例，再更换图标": "Quit this instance before changing its icon",
    "请检查上述原因，调整后重试；也可以使用“检查”查看详情。":
      "Review the reason above and try again. Use Check for more details.",
    "请等当前操作完成后再退出 Harbor。": "Wait for the current operation to finish before quitting Harbor.",
    "请选择有效的 PNG、JPEG 或 ICNS 图片，文件不超过 16 MB。":
      "Choose a valid PNG, JPEG or ICNS image no larger than 16 MB.",
    "账号数据保留在 Harbor 的 retained 目录，完成后会显示具体位置。保留的数据可以用于重新接管。":
      "Account data will be kept in Harbor's retained folder. Its location will be shown when removal finishes. You can adopt the retained data again.",
    "账号登录在副本窗口中完成。HOME、SSH、Docker 和系统钥匙串仍共享。":
      "Sign in inside the copy's window. HOME, SSH, Docker and the system Keychain are still shared.",
    "路径不存在：%@": "Path does not exist: %@",
    "轮廓由透明度决定；不透明照片会显示为实心方块，建议使用透明背景的简洁图形。":
      "The silhouette follows image transparency. Opaque photos appear as solid squares; use a simple shape on a transparent background.",
    "辅助进程未退出": "Helpers still running",
    "运行中": "Running",
    "退出 Harbor": "Quit Harbor",
    "选择…": "Choose…",
    "选择图片…": "Choose Image…",
    "选择官方 ChatGPT / Codex 原版应用": "Choose the Official ChatGPT / Codex App",
    "选择官方原版，Harbor 会创建独立副本和空的数据目录。":
      "Choose the official app. Harbor will create an independent copy with empty data folders.",
    "选择实例图标": "Choose an Instance Icon",
    "需要检查": "Needs attention",
    "语言": "Language",
    "蓝色": "Blue", "绿色": "Green", "橙色": "Orange", "紫色": "Purple", "粉色": "Pink", "黄色": "Yellow",
    "角标简称": "Badge text", "角标颜色": "Badge color", "自动分配": "Automatic",
    "最多 4 个字符；留空时根据副本名称生成。": "Up to 4 characters; leave blank to derive from the instance name.",
    "菜单栏简称": "Menu-bar initials",
    "使用角标的前两个字符，保持透明背景。": "Uses the first two badge characters on a transparent background.",
    "无法读取原版图标，请确认官方应用仍在所选位置。":
      "Cannot read the original icon. Check that the official app is still at the selected location.",
    "角标简称不能超过 4 个字符，也不能包含换行。": "Badge text must be at most 4 characters with no line breaks.",
    "无法生成角标图标。": "Could not generate the badge icon.",
    "使用名称角标": "Use a Name Badge", "图标样式": "Icon style",
    "原版加角标": "Original with Badge", "自选图片": "Custom Image",
  ]
}
