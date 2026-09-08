import AppKit
import XCTest

@testable import HarborNative

final class NativeOperationsTests: XCTestCase {
  @MainActor
  func testWrongIdentityNeverReceivesQuit() {
    XCTAssertThrowsError(
      try NativeOperations.stop(
        pid: getpid(),
        bundle: URL(fileURLWithPath: "/not-the-test-process.app"),
        executable: URL(fileURLWithPath: "/not-the-test-process"), identifier: "not.the.test"))
  }

  @MainActor
  func testTrashCanBeRestoredAndFailureRollsBackEarlierItems() throws {
    let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    try FileManager.default.createDirectory(at: root, withIntermediateDirectories: false)
    defer { try? FileManager.default.removeItem(at: root) }
    let first = root.appendingPathComponent("harbor-test-item")
    try Data("test-only".utf8).write(to: first)
    let result = try NativeOperations.trashTransaction([first])
    XCTAssertFalse(FileManager.default.fileExists(atPath: first.path))
    let location = try XCTUnwrap(result.first?["trashed"])
    try FileManager.default.moveItem(at: URL(fileURLWithPath: location), to: first)
    XCTAssertThrowsError(
      try NativeOperations.trashTransaction([first, root.appendingPathComponent("missing")]))
    XCTAssertEqual(try Data(contentsOf: first), Data("test-only".utf8))
  }
}
