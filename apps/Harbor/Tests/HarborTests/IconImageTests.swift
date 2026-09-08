import AppKit
import XCTest

@testable import Harbor

final class IconImageTests: XCTestCase {
  @MainActor
  func testMenuLetterHasRetinaSizeAndTransparentBackground() throws {
    let data = try IconImage.menuBarLetter("work")
    let rep = try XCTUnwrap(NSBitmapImageRep(data: data))
    XCTAssertEqual(rep.pixelsWide, 36)
    XCTAssertEqual(rep.pixelsHigh, 36)
    XCTAssertEqual(rep.colorAt(x: 0, y: 0)?.alphaComponent, 0)
    var hasInk = false
    for x in 0..<36 {
      for y in 0..<36 {
        if (rep.colorAt(x: x, y: y)?.alphaComponent ?? 0) > 0.5 { hasInk = true }
      }
    }
    XCTAssertTrue(hasInk)
  }

  @MainActor
  func testSelectedNonSquareImageIsPaddedWithoutStretching() throws {
    let url = FileManager.default.temporaryDirectory.appendingPathComponent(
      UUID().uuidString + ".png")
    defer { try? FileManager.default.removeItem(at: url) }
    let source = try XCTUnwrap(
      NSBitmapImageRep(
        bitmapDataPlanes: nil, pixelsWide: 40, pixelsHigh: 20,
        bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false,
        colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0))
    for x in 0..<40 {
      for y in 0..<20 {
        source.setColor(NSColor(deviceRed: 1, green: 0, blue: 0, alpha: 1), atX: x, y: y)
      }
    }
    try XCTUnwrap(source.representation(using: .png, properties: [:])).write(to: url)
    let icon = try IconImage(url: url)
    let result = try XCTUnwrap(NSBitmapImageRep(data: icon.png))
    XCTAssertEqual(result.pixelsWide, 1024)
    XCTAssertEqual(result.pixelsHigh, 1024)
    XCTAssertEqual(result.colorAt(x: 512, y: 0)?.alphaComponent, 0)
    XCTAssertGreaterThan(result.colorAt(x: 512, y: 512)?.alphaComponent ?? 0, 0.9)
  }
}
