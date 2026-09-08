import AppKit
import ImageIO

/// Decode a thumbnail rather than expanding an arbitrarily large source image.
@MainActor
struct IconImage {
  let png: Data
  let preview: NSImage

  init(url: URL) throws {
    let values = try url.resourceValues(forKeys: [.fileSizeKey])
    guard let size = values.fileSize, size <= 16 * 1024 * 1024,
      let source = CGImageSourceCreateWithURL(url as CFURL, nil),
      let thumbnail = CGImageSourceCreateThumbnailAtIndex(
        source, 0,
        [
          kCGImageSourceCreateThumbnailFromImageAlways: true,
          kCGImageSourceCreateThumbnailWithTransform: true,
          kCGImageSourceThumbnailMaxPixelSize: 1024,
        ] as CFDictionary),
      let bitmap = NSBitmapImageRep(
        bitmapDataPlanes: nil, pixelsWide: 1024, pixelsHigh: 1024,
        bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false,
        colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0),
      let context = NSGraphicsContext(bitmapImageRep: bitmap)
    else { throw CLIError(message: "请选择有效的 PNG、JPEG 或 ICNS 图片，文件不超过 16 MB。") }
    NSGraphicsContext.saveGraphicsState()
    NSGraphicsContext.current = context
    let image = NSImage(
      cgImage: thumbnail, size: NSSize(width: thumbnail.width, height: thumbnail.height))
    let ratio = min(1024.0 / image.size.width, 1024.0 / image.size.height)
    let width = image.size.width * ratio
    let height = image.size.height * ratio
    image.draw(
      in: NSRect(x: (1024 - width) / 2, y: (1024 - height) / 2, width: width, height: height),
      from: .zero, operation: .copy, fraction: 1)
    NSGraphicsContext.restoreGraphicsState()
    guard let png = bitmap.representation(using: .png, properties: [:]),
      let preview = NSImage(data: png)
    else {
      throw CLIError(message: "无法生成图标预览，请换一张图片。")
    }
    self.png = png
    self.preview = preview
  }
  static func menuBarLetter(_ name: String) throws -> Data {
    guard
      let bitmap = NSBitmapImageRep(
        bitmapDataPlanes: nil, pixelsWide: 36, pixelsHigh: 36,
        bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false,
        colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0),
      let context = NSGraphicsContext(bitmapImageRep: bitmap)
    else { throw CLIError(message: "无法生成菜单栏图标。") }
    NSGraphicsContext.saveGraphicsState()
    NSGraphicsContext.current = context
    let text = NSAttributedString(
      string: String(name.prefix(1)).uppercased(),
      attributes: [
        .font: NSFont.systemFont(ofSize: 28, weight: .bold), .foregroundColor: NSColor.black,
      ])
    let size = text.size()
    text.draw(at: NSPoint(x: (36 - size.width) / 2, y: (36 - size.height) / 2))
    NSGraphicsContext.restoreGraphicsState()
    guard let png = bitmap.representation(using: .png, properties: [:]) else {
      throw CLIError(message: "无法生成菜单栏图标。")
    }
    return png
  }

}
