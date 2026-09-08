import AppKit

@main
struct HarborNative {
  @MainActor
  static func main() {
    do {
      let args = Array(CommandLine.arguments.dropFirst())
      var response: [String: Any] = ["ok": true]
      switch args.first {
      case "stop":
        guard args.count == 5, let pid = Int32(args[1]), pid > 0 else {
          throw NativeFailure(message: "Invalid native stop arguments")
        }
        try NativeOperations.stop(
          pid: pid, bundle: URL(fileURLWithPath: args[2]),
          executable: URL(fileURLWithPath: args[3]), identifier: args[4])
      case "trash":
        guard args.count == 2 || args.count == 3,
          args[1].hasSuffix(".app"), args.dropFirst().allSatisfy({ $0.hasPrefix("/") })
        else {
          throw NativeFailure(message: "Invalid native trash arguments")
        }
        let paths = args.dropFirst().map { URL(fileURLWithPath: $0) }
        response["items"] = try NativeOperations.trash(paths, bundle: paths[0])
      default: throw NativeFailure(message: "Unsupported native operation")
      }
      let data = try JSONSerialization.data(withJSONObject: response, options: [.sortedKeys])
      FileHandle.standardOutput.write(data)
    } catch {
      FileHandle.standardError.write(Data((error.localizedDescription + "\n").utf8))
      exit(1)
    }
  }
}
