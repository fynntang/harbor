import Foundation
import XCTest

@testable import Harbor

final class HarborStoreTests: XCTestCase, @unchecked Sendable {
  @MainActor
  func testCreatedCopyGetsBadgeBeforeFlowFinishes() async throws {
    let fixture = try Fixture()
    defer { fixture.remove() }
    let store = HarborStore(client: HarborClient(executable: fixture.cli))
    store.creating = true
    await store.create(
      name: "work", source: "/Applications/Original.app", iconPNG: Data([1]), trayPNG: Data([2]))
    XCTAssertNil(store.error)
    XCTAssertFalse(store.creating)
    XCTAssertFalse(store.busy)
    XCTAssertEqual(store.iconRevision, 1)
    XCTAssertTrue(
      FileManager.default.fileExists(atPath: fixture.dir.appendingPathComponent("icon-called").path)
    )
  }

  @MainActor
  func testDisplayNameIsSentLiterallyWhileActionsUseIdentifier() async throws {
    let fixture = try Fixture()
    defer { fixture.remove() }
    let store = HarborStore(client: HarborClient(executable: fixture.cli))
    await store.create(name: "-工作账号（Toobit）", source: "/Applications/Original.app")
    XCTAssertNil(store.error)
    XCTAssertEqual(try String(contentsOf: fixture.dir.appendingPathComponent("submitted-name"), encoding: .utf8), "-工作账号（Toobit）")
    XCTAssertEqual(store.selection, "work")
    XCTAssertEqual(store.selected?.profile.displayName, "工作账号（Toobit）")
    XCTAssertEqual(store.selected?.profile.supportsCustomIcon, true)
    await store.start()
    XCTAssertNil(store.error)
  }

  @MainActor
  func testIconFailureKeepsCreatedProfileWithoutOfferingDuplicateCreation() async throws {
    let fixture = try Fixture()
    defer { fixture.remove() }
    try Data().write(to: fixture.dir.appendingPathComponent("fail-icon"))
    let store = HarborStore(client: HarborClient(executable: fixture.cli))
    store.creating = true
    await store.create(
      name: "work", source: "/Applications/Original.app", iconPNG: Data([1]), trayPNG: Data([2]))
    XCTAssertEqual(store.error?.rendered(language: .english), "Icon failed")
    XCTAssertEqual(store.selection, "work")
    XCTAssertEqual(store.profiles.count, 1)
    XCTAssertFalse(store.creating)
    XCTAssertFalse(store.busy)
    XCTAssertEqual(store.iconRevision, 0)
  }

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
      profile='{"name":"work","display_name":"工作账号（Toobit）","bundle_id":"com.openai.codex.harbor.work","adopted_data":false,"app_bundle":"/test/Work.app","registered_app_version":"1","registered_app_build_version":"1","codex_home":"/test/codex","gui_home":"/test/gui"}'
      case "$2" in
        clone)
          if [ -f fail ]; then
            printf '{"api_version":1,"ok":false,"error":"Target already exists"}\n'
            exit 1
          fi
          test "$3" = --source || exit 3
          test "$5" = -- || exit 4
          printf %s "$6" > submitted-name
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
          test "$3" = work || exit 5
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
        icon)
          test -f created || exit 2
          test -f "$5" || exit 3
          test -f "$7" || exit 4
          touch icon-called
          if [ -f fail-icon ]; then
            printf '{"api_version":1,"ok":false,"error":"Icon failed"}\n'
            exit 1
          fi
          printf '{"api_version":1,"ok":true,"data":{"updated":true}}\n'
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
