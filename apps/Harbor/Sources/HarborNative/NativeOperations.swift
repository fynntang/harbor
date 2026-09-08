import AppKit

struct NativeFailure: LocalizedError {
  let message: String
  var errorDescription: String? { message }
}

@MainActor
enum NativeOperations {
  /// NSRunningApplication tracks an application instance, not a reusable saved PID.
  static func stop(pid: pid_t, bundle: URL, executable: URL, identifier: String) throws {
    let deadline = Date().addingTimeInterval(15)
    var candidate: NSRunningApplication?
    repeat {
      candidate = NSRunningApplication(processIdentifier: pid)
      if let app = candidate, app.isFinishedLaunching, app.bundleURL != nil,
        app.executableURL != nil, app.bundleIdentifier != nil
      {
        break
      }
      RunLoop.current.run(until: Date().addingTimeInterval(0.1))
    } while Date() < deadline
    guard let app = candidate, !app.isTerminated,
      app.bundleURL?.resolvingSymlinksInPath() == bundle.resolvingSymlinksInPath(),
      app.executableURL?.resolvingSymlinksInPath() == executable.resolvingSymlinksInPath(),
      app.bundleIdentifier == identifier
    else {
      throw NativeFailure(
        message:
          "The app is not yet registered with macOS or its identity changed. Wait for its window and retry; nothing was stopped."
      )
    }
    guard app.isFinishedLaunching else {
      throw NativeFailure(
        message: "The app is still starting. Wait for its window and retry; nothing was deleted.")
    }
    var sent = app.terminate()
    while !sent && !app.isTerminated && Date() < deadline {
      RunLoop.current.run(until: Date().addingTimeInterval(0.1))
      sent = app.terminate()
    }
    guard sent || app.isTerminated else {
      throw NativeFailure(
        message: "The app did not accept a normal quit request. Save tasks and quit it manually.")
    }
    while !app.isTerminated && Date() < deadline {
      RunLoop.current.run(until: Date().addingTimeInterval(0.1))
    }
    guard app.isTerminated else {
      throw NativeFailure(
        message:
          "The app has not exited. Complete or cancel its save/quit dialog; no deletion was performed."
      )
    }
  }

  /// Trash supports recovery; if a later item fails, restore earlier items without overwriting.
  static func trash(_ paths: [URL], bundle: URL) throws -> [[String: String]] {
    guard
      !NSWorkspace.shared.runningApplications.contains(where: {
        guard let url = $0.executableURL?.resolvingSymlinksInPath() else { return false }
        return url.path.hasPrefix(bundle.path + "/") && !$0.isTerminated
      })
    else { throw NativeFailure(message: "The selected app is still running; nothing was deleted.") }
    return try trashTransaction(paths)
  }

  static func trashTransaction(_ paths: [URL]) throws -> [[String: String]] {
    var moved: [(URL, URL)] = []
    do {
      for path in paths {
        var destination: NSURL?
        try FileManager.default.trashItem(at: path, resultingItemURL: &destination)
        guard let destination else {
          throw NativeFailure(
            message:
              "Trash did not return a recovery path for \(path.path). Inspect Trash before retrying."
          )
        }
        moved.append((path, destination as URL))
      }
    } catch {
      var unrestored: [String] = []
      for (original, trashed) in moved.reversed() {
        do { try FileManager.default.moveItem(at: trashed, to: original) } catch {
          unrestored.append("\(trashed.path) -> \(original.path)")
        }
      }
      if !unrestored.isEmpty {
        throw NativeFailure(
          message: "Trash operation failed and some items need manual recovery: "
            + unrestored.joined(separator: "; "))
      }
      throw error
    }
    return moved.map { ["original": $0.0.path, "trashed": $0.1.path] }
  }
}
