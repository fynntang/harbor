import Foundation
import XCTest

@testable import Harbor

final class HarborStoreTests: XCTestCase, @unchecked Sendable {
  @MainActor
  func testCreateStartCheckFlowKeepsSelectionAndPreventsDuplicateLaunch() async throws {
    let fixture = try Fixture()
    defer { fixture.remove() }
    let store = HarborStore(client: HarborClient(executable: fixture.cli))
    await store.refresh()
    XCTAssertTrue(store.profiles.isEmpty)
    store.creating = true
    await store.create(name: "work", source: "/Applications/Original.app")
    XCTAssertFalse(store.busy)
    XCTAssertFalse(store.creating)
    XCTAssertEqual(store.selection, "work")
    XCTAssertEqual(store.profiles.count, 1)
    XCTAssertTrue(store.message.rendered(language: .simplifiedChinese).contains("副本已创建"))
    await store.start()
    XCTAssertEqual(store.selected?.status.state, "running")
    XCTAssertTrue(store.message.rendered(language: .simplifiedChinese).contains("启动成功"))
    await store.start()
    XCTAssertTrue(store.message.rendered(language: .simplifiedChinese).contains("没有重复启动"))
    await store.check()
    XCTAssertEqual(store.report?.passed, false)
    XCTAssertEqual(store.report?.error, "Signature mismatch")
    XCTAssertFalse(store.busy)
  }

  @MainActor
  func testFailedCreatePreservesFormAndAllowsRetry() async throws {
    let fixture = try Fixture()
    defer { fixture.remove() }
    try Data().write(to: fixture.dir.appendingPathComponent("fail"))
    let store = HarborStore(client: HarborClient(executable: fixture.cli))
    store.creating = true
    await store.create(name: "work", source: "/Applications/Original.app")
    XCTAssertTrue(store.creating)
    XCTAssertFalse(store.busy)
    XCTAssertEqual(store.error?.rendered(language: .english), "Target already exists")
    XCTAssertTrue(store.profiles.isEmpty)
    try FileManager.default.removeItem(at: fixture.dir.appendingPathComponent("fail"))
    await store.create(name: "work", source: "/Applications/Original.app")
    XCTAssertNil(store.error)
    XCTAssertEqual(store.selection, "work")
  }
  @MainActor
  func testStopThenDeleteKeepsDataByDefault() async throws {
    let fixture = try Fixture()
    defer { fixture.remove() }
    let store = HarborStore(client: HarborClient(executable: fixture.cli))
    await store.create(name: "work", source: "/Applications/Original.app")
    await store.start()
    let item = try XCTUnwrap(store.selected)
    let removed = await store.remove(item: item, deleteData: false)
    XCTAssertTrue(removed)
    XCTAssertTrue(store.profiles.isEmpty)
    XCTAssertEqual(store.retainedData, "/test/retained/work")
    XCTAssertFalse(store.busy)
  }

  @MainActor
  func testQuitRefusalNeverCallsRemove() async throws {
    let fixture = try Fixture()
    defer { fixture.remove() }
    let store = HarborStore(client: HarborClient(executable: fixture.cli))
    await store.create(name: "work", source: "/Applications/Original.app")
    await store.start()
    try Data().write(to: fixture.dir.appendingPathComponent("refuse-stop"))
    let item = try XCTUnwrap(store.selected)
    let removed = await store.remove(item: item, deleteData: true)
    XCTAssertFalse(removed)
    XCTAssertEqual(store.profiles.count, 1)
    XCTAssertFalse(
      FileManager.default.fileExists(atPath: fixture.dir.appendingPathComponent("removed").path))
    XCTAssertEqual(store.error?.rendered(language: .english), "Quit cancelled")
  }

}

private struct Fixture {
  let dir: URL
  let cli: URL
  init() throws {
    dir = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    cli = dir.appendingPathComponent("harbor")
    try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: false)
    let script = #"""
      #!/bin/sh
      cd "$(dirname "$0")" || exit 2
      profile='{"name":"work","app_bundle":"/test/Work.app","registered_app_version":"1","registered_app_build_version":"1","codex_home":"/test/codex","gui_home":"/test/gui"}'
      case "$2" in
        clone)
          if [ -f fail ]; then
            printf '{"api_version":1,"ok":false,"error":"Target already exists"}\n'
            exit 1
          fi
          touch created
          printf '{"api_version":1,"ok":true,"data":{"profile":%s}}\n' "$profile"
          ;;
        list)
          if [ -f created ]; then
            state=stopped
            if [ -f started ]; then state=running; fi
            printf '{"api_version":1,"ok":true,"data":{"root":"/test","profiles":[{"profile":%s,"status":{"state":"%s","pids":[]},"current_version":"1","current_build":"1"}]}}\n' "$profile" "$state"
          else
            printf '{"api_version":1,"ok":true,"data":{"root":"/test","profiles":[]}}\n'
          fi
          ;;
        start)
          state=started
          if [ -f started ]; then state=already_running; fi
          touch started
          printf '{"api_version":1,"ok":true,"data":{"state":"%s","pid":42}}\n' "$state"
          ;;
        stop)
          if [ -f refuse-stop ]; then
            printf '{"api_version":1,"ok":false,"error":"Quit cancelled"}\n'
            exit 1
          fi
          rm -f started
          printf '{"api_version":1,"ok":true,"data":{"state":"stopped"}}\n'
          ;;
        remove)
          test ! -f started || exit 2
          touch removed
          rm -f created
          printf '{"api_version":1,"ok":true,"data":{"retained_data":"/test/retained/work"}}\n'
          ;;
        doctor)
          printf '{"api_version":1,"ok":true,"data":{"passed":false,"report":"FAIL: codesign","error":"Signature mismatch"}}\n'
          ;;
        *) exit 2 ;;
      esac
      """#
    try script.write(to: cli, atomically: true, encoding: .utf8)
    try FileManager.default.setAttributes([.posixPermissions: 0o700], ofItemAtPath: cli.path)
  }
  func remove() { try? FileManager.default.removeItem(at: dir) }
}
