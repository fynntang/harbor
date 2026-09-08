import AppKit
import XCTest

@testable import Harbor

final class BadgeIconTests: XCTestCase {
  func testAutomaticLabelsAndSavedOverrides() throws {
    XCTAssertEqual(BadgeSettings().text(for: "work"), "WORK")
    XCTAssertEqual(BadgeSettings().text(for: "toobit"), "TOO")
    XCTAssertFalse(BadgeSettings(label: "abcde").isValid)
    XCTAssertFalse(BadgeSettings(label: "a\nb").isValid)
    let suite = "harbor-badge-test-\(UUID().uuidString)"
    let defaults = try XCTUnwrap(UserDefaults(suiteName: suite))
    defer { defaults.removePersistentDomain(forName: suite) }
    let settings = BadgeSettings(label: "TB", color: .purple)
    settings.save(for: "/test/work.app", defaults: defaults)
    XCTAssertEqual(BadgeSettings.load(for: "/test/work.app", defaults: defaults), settings)
    XCTAssertEqual(BadgeSettings.load(for: "/other/work.app", defaults: defaults), BadgeSettings())
  }

  @MainActor
  func testBadgePreservesBaseAndRepeatedEditsDoNotAccumulate() throws {
    let base = NSImage(size: NSSize(width: 1024, height: 1024), flipped: false) { _ in
      NSColor.white.setFill()
      NSBezierPath(rect: NSRect(x: 80, y: 80, width: 864, height: 864)).fill()
      return true
    }
    let original = IconImage(png: Data([1, 2, 3]), preview: base)
    let blue = try original.badged(text: "WORK", color: .blue)
    let yellow = try original.badged(text: "TOO", color: .yellow)
    let again = try original.badged(text: "WORK", color: .blue)
    XCTAssertEqual(original.png, Data([1, 2, 3]))
    XCTAssertEqual(blue.png, again.png)
    XCTAssertNotEqual(blue.png, yellow.png)
    let bitmap = try XCTUnwrap(NSBitmapImageRep(data: blue.png))
    XCTAssertEqual(bitmap.pixelsWide, 1024)
    XCTAssertEqual(bitmap.pixelsHigh, 1024)
    XCTAssertEqual(bitmap.colorAt(x: 0, y: 0)?.alphaComponent, 0)
    XCTAssertGreaterThan(bitmap.colorAt(x: 200, y: 200)?.redComponent ?? 0, 0.95)
  }

  @MainActor
  func testMenuInitialsFitWithoutClipping() throws {
    for label in ["WW", "工作", "TOO"] {
      let data = try IconImage.menuBarText(label)
      let bitmap = try XCTUnwrap(NSBitmapImageRep(data: data))
      XCTAssertEqual(bitmap.pixelsWide, 36)
      for n in 0..<36 {
        XCTAssertEqual(bitmap.colorAt(x: 0, y: n)?.alphaComponent, 0)
        XCTAssertEqual(bitmap.colorAt(x: 35, y: n)?.alphaComponent, 0)
      }
    }
  }

  @MainActor
  func testBaseResolutionNeverUsesAnAlreadyPaintedIcon() throws {
    let folder = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    defer { try? FileManager.default.removeItem(at: folder) }
    let resources = folder.appendingPathComponent("Contents/Resources")
    try FileManager.default.createDirectory(at: resources, withIntermediateDirectories: true)
    try Data([1]).write(to: resources.appendingPathComponent("icon-chatgpt.png"))
    XCTAssertEqual(
      IconImage.originalURL(for: folder.path).path,
      "/Applications/ChatGPT.app/Contents/Resources/icon-chatgpt.png")
    let saved = resources.appendingPathComponent("harbor-original-icon.png")
    try Data([2]).write(to: saved)
    XCTAssertEqual(IconImage.originalURL(for: folder.path), saved)
  }
}
