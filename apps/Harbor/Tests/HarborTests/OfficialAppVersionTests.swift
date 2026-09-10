import Foundation
import XCTest
@testable import Harbor

final class OfficialAppVersionTests: XCTestCase {
  func testReadsReplacedPlistAndClearsMissingOrInvalidSource() throws {
    let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    let contents = root.appendingPathComponent("Contents")
    try FileManager.default.createDirectory(at: contents, withIntermediateDirectories: true)
    defer { try? FileManager.default.removeItem(at: root) }
    let plist = contents.appendingPathComponent("Info.plist")
    func write(_ id: String, _ build: String) throws {
      let data = try PropertyListSerialization.data(fromPropertyList: [
        "CFBundleIdentifier": id, "CFBundleShortVersionString": "26.9", "CFBundleVersion": build
      ], format: .xml, options: 0)
      try data.write(to: plist, options: .atomic)
    }
    XCTAssertNil(OfficialAppVersion.read(at: root))
    try write("com.openai.codex", "9")
    XCTAssertEqual(OfficialAppVersion.read(at: root)?.build, 9)
    try write("com.openai.codex", "10")
    XCTAssertEqual(OfficialAppVersion.read(at: root)?.build, 10)
    try write("com.openai.codex.harbor.work", "11")
    XCTAssertNil(OfficialAppVersion.read(at: root))
    try write("com.openai.codex", "unknown")
    XCTAssertNil(OfficialAppVersion.read(at: root))
    try FileManager.default.removeItem(at: plist)
    XCTAssertNil(OfficialAppVersion.read(at: root))
  }

  func testOnlyNewerNumericBuildsNotifyManagedCopiesEvenWhenRunning() throws {
    func item(_ build: String?, managed: Bool = true) throws -> ProfileItem {
      let data = try JSONSerialization.data(withJSONObject: [
        "profile": ["name": "work", "bundle_id": "com.openai.codex.harbor.work",
          "adopted_data": !managed, "app_bundle": "/Work.app", "registered_app_version": "1",
          "codex_home": "/work/codex", "gui_home": "/work/gui"],
        "status": ["state": "running", "pids": [123]],
        "current_build": build as Any? ?? NSNull()
      ])
      return try JSONDecoder().decode(ProfileItem.self, from: data)
    }
    let source = OfficialAppVersion(version: "26.9", build: 10)
    XCTAssertTrue(source.isNewer(than: try item("9")))
    XCTAssertFalse(source.isNewer(than: try item("10")))
    XCTAssertFalse(source.isNewer(than: try item("11")))
    XCTAssertFalse(source.isNewer(than: try item(nil)))
    XCTAssertFalse(source.isNewer(than: try item("invalid")))
    XCTAssertFalse(source.isNewer(than: try item("9", managed: false)))
  }
}
