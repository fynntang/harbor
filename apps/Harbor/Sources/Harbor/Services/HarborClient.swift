import Foundation

/// Calls only the bundled CLI with separate argv entries, never a shell.
struct HarborClient: Sendable {
  let executable: URL
  let root: String?

  init(executable: URL? = nil, root: String? = nil) {
    self.executable =
      executable ?? Bundle.main.bundleURL.appendingPathComponent("Contents/Helpers/harbor")
    self.root = root
  }

  func request<T: Decodable & Sendable>(
    _ arguments: [String],
    progress: @escaping @Sendable (String) -> Void = { _ in }
  ) async throws -> T {
    let executable = self.executable
    let argv = ["--json"] + (root.map { ["--root", $0] } ?? []) + arguments
    let (data, exitCode): (Data, Int32) = try await withCheckedThrowingContinuation {
      continuation in
      DispatchQueue.global(qos: .userInitiated).async {
        do {
          guard FileManager.default.isExecutableFile(atPath: executable.path) else {
            throw CLIError(message: "Harbor 应用缺少内置命令行组件，请重新构建或安装应用。")
          }
          let process = Process()
          process.executableURL = executable
          process.arguments = argv
          process.standardInput = FileHandle.nullDevice
          let stdout = Pipe()
          let stderr = Pipe()
          process.standardOutput = stdout
          process.standardError = stderr
          try process.run()
          // Drain both pipes concurrently so verbose failures cannot deadlock.
          let readers = DispatchGroup()
          readers.enter()
          DispatchQueue.global(qos: .userInitiated).async {
            defer { readers.leave() }
            var pending = Data()
            while true {
              let chunk = stderr.fileHandleForReading.availableData
              if chunk.isEmpty { break }
              pending.append(chunk)
              while let newline = pending.firstIndex(of: 10) {
                let line = pending.prefix(upTo: newline)
                if let object = try? JSONSerialization.jsonObject(with: line) as? [String: String],
                  object["event"] == "progress", let stage = object["stage"]
                {
                  progress(stage)
                }
                pending.removeSubrange(...newline)
              }
              // Non-protocol stderr is neither displayed nor persisted.
              if pending.count > 65_536 { pending.removeAll() }
            }
          }
          let data = stdout.fileHandleForReading.readDataToEndOfFile()
          process.waitUntilExit()
          readers.wait()
          continuation.resume(returning: (data, process.terminationStatus))
        } catch { continuation.resume(throwing: error) }
      }
    }
    return try Self.decode(data, exitCode: exitCode)
  }

  static func decode<T: Decodable>(_ data: Data, exitCode: Int32) throws -> T {
    let response: CLIResponse<T>
    do { response = try JSONDecoder().decode(CLIResponse<T>.self, from: data) } catch {
      throw CLIError(message: "Harbor 组件返回了无法识别的结果（退出码 \(exitCode)），请重新构建应用。")
    }
    guard response.api_version == 1 else {
      throw CLIError(message: "界面与 Harbor 组件版本不匹配，请重新构建应用。")
    }
    guard response.ok, exitCode == 0, let result = response.data else {
      throw CLIError(
        message: response.error.map { GUIMessage(verbatim: $0) } ?? "操作未完成（退出码 \(exitCode)）。")
    }
    return result
  }
}
