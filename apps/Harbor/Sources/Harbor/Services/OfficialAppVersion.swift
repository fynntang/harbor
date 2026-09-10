import Foundation

struct OfficialAppVersion: Equatable {
  static let defaultURL = URL(fileURLWithPath: "/Applications/ChatGPT.app")
  let version: String
  let build: UInt64

  static func read(at app: URL) -> Self? {
    guard let data = try? Data(contentsOf: app.appendingPathComponent("Contents/Info.plist")),
      let info = try? PropertyListSerialization.propertyList(from: data, format: nil) as? [String: Any],
      info["CFBundleIdentifier"] as? String == "com.openai.codex",
      let version = info["CFBundleShortVersionString"] as? String, !version.isEmpty,
      let rawBuild = info["CFBundleVersion"] as? String, let build = UInt64(rawBuild)
    else { return nil }
    return Self(version: version, build: build)
  }

  func isNewer(than item: ProfileItem) -> Bool {
    guard item.profile.supportsCustomIcon, let current = item.current_build,
      let build = UInt64(current) else { return false }
    return self.build > build
  }
}
