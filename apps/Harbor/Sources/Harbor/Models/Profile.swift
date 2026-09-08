import Foundation

struct Profile: Decodable, Sendable {
  let name: String
  let bundle_id: String?
  let adopted_data: Bool?

  var supportsCustomIcon: Bool {
    adopted_data == false && bundle_id == "com.openai.codex.harbor.\(name)"
  }
  let app_bundle: String
  let registered_app_version: String
  let registered_app_build_version: String?
  let codex_home: String
  let gui_home: String
}

struct ProcessStatus: Decodable, Sendable {
  let state: String
  let pids: [Int]
  let error: String?

  var title: String {
    switch state {
    case "running": "运行中"
    case "stopped": "未运行"
    case "helpers_running": "辅助进程未退出"
    case "conflict": "启动冲突"
    default: "状态未知"
    }
  }
}

struct ProfileItem: Decodable, Identifiable, Sendable {
  let profile: Profile
  let status: ProcessStatus
  let current_version: String?
  let current_build: String?
  let issue: String?
  var id: String { profile.name }
}

struct ProfileList: Decodable, Sendable {
  let profiles: [ProfileItem]
  let root: String
}
struct CreatedProfile: Decodable, Sendable { let profile: Profile }
struct StartedProfile: Decodable, Sendable {
  let state: String
  let pid: Int
}
struct StoppedProfile: Decodable, Sendable { let state: String }
struct RemovedProfile: Decodable, Sendable { let retained_data: String? }

struct IconResult: Decodable, Sendable { let updated: Bool }

struct CheckReport: Decodable, Sendable {
  let passed: Bool
  let report: String
  let error: String?
}

struct CLIResponse<T: Decodable>: Decodable {
  let api_version: Int
  let ok: Bool
  let data: T?
  let error: String?
}

struct CLIError: LocalizedError {
  let message: GUIMessage
  var errorDescription: String? { message.rendered(language: .simplifiedChinese) }
}

func validProfileName(_ name: String) -> Bool {
  name.range(of: "\\A[a-z0-9][a-z0-9-]{0,47}\\z", options: .regularExpression) != nil
}
