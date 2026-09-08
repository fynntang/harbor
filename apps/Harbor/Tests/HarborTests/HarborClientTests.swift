import Foundation
import XCTest

@testable import Harbor

final class HarborClientTests: XCTestCase, @unchecked Sendable {
  func testProtocolRejectsVersionMismatchAndFailedExit() throws {
    let json = Data(#"{"api_version":2,"ok":true,"data":{"state":"started","pid":1}}"#.utf8)
    XCTAssertThrowsError(try HarborClient.decode(json, exitCode: 0) as StartedProfile)
    let failed = Data(#"{"api_version":1,"ok":false,"error":"Existing profile"}"#.utf8)
    XCTAssertThrowsError(try HarborClient.decode(failed, exitCode: 1) as StartedProfile) {
      XCTAssertEqual($0.localizedDescription, "Existing profile")
    }
    let inconsistent = Data(#"{"api_version":1,"ok":true,"data":{"state":"started","pid":1}}"#.utf8)
    XCTAssertThrowsError(try HarborClient.decode(inconsistent, exitCode: 1) as StartedProfile)
  }

  func testProcessUsesLiteralArgumentsAndDrainsBothPipes() async throws {
    let dir = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: false)
    defer { try? FileManager.default.removeItem(at: dir) }
    let executable = dir.appendingPathComponent("mock harbor")
    let script = #"""
      #!/bin/sh
      test "$1" = '--json' || exit 2
      test "$2" = '--root' || exit 3
      test "$3" = '/tmp/space and $literal' || exit 4
      test "$4" = 'show' || exit 5
      test "$5" = 'literal; $(false)' || exit 6
      i=0
      while [ "$i" -lt 5000 ]; do
        printf 'not protocol stderr; intentionally ignored\n' >&2
        i=$((i + 1))
      done
      printf '{"event":"progress","stage":"copy"}\n' >&2
      printf '{"api_version":1,"ok":true,"data":{"state":"started","pid":42}}\n'
      """#
    try script.write(to: executable, atomically: true, encoding: .utf8)
    try FileManager.default.setAttributes([.posixPermissions: 0o700], ofItemAtPath: executable.path)
    let progress = expectation(description: "progress event")
    let client = HarborClient(executable: executable, root: "/tmp/space and $literal")
    let result: StartedProfile = try await client.request(["show", "literal; $(false)"]) { stage in
      if stage == "copy" { progress.fulfill() }
    }
    XCTAssertEqual(result.pid, 42)
    await fulfillment(of: [progress], timeout: 2)
  }

  func testMissingBundledCLIHasActionableError() async {
    let client = HarborClient(executable: URL(fileURLWithPath: "/nonexistent/harbor-test"))
    do {
      let _: StartedProfile = try await client.request(["start", "work"])
      XCTFail("Missing CLI accepted")
    } catch { XCTAssertTrue(error.localizedDescription.contains("内置命令行组件")) }
  }

  func testNamesFollowCoreContract() {
    for name in ["work", "work-2", "0"] { XCTAssertTrue(validProfileName(name)) }
    for name in ["", "../work", "Work", "a_b", "a b", "a\n", String(repeating: "a", count: 49)] {
      XCTAssertFalse(validProfileName(name), name)
    }
  }
}
