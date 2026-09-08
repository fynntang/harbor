// Reproduce with: swift scripts/generate_app_icon.swift /absolute/output/directory
// Then: iconutil -c icns /absolute/output/directory/Harbor.iconset
import AppKit

let output = URL(fileURLWithPath: CommandLine.arguments[1], isDirectory: true)
let iconset = output.appendingPathComponent("Harbor.iconset", isDirectory: true)
try FileManager.default.createDirectory(at: iconset, withIntermediateDirectories: true)

// Use the same SF Symbol as HarborApp's MenuBarExtra, with no background tile.
let configuration = NSImage.SymbolConfiguration(pointSize: 800, weight: .regular)
  .applying(NSImage.SymbolConfiguration(paletteColors: [.white]))
guard let sailboat = NSImage(systemSymbolName: "sailboat", accessibilityDescription: nil)?
  .withSymbolConfiguration(configuration) else {
  fatalError("The macOS sailboat symbol is unavailable")
}

func drawIcon() {
  let available: CGFloat = 824
  let scale = min(available / sailboat.size.width, available / sailboat.size.height)
  let size = NSSize(width: sailboat.size.width * scale, height: sailboat.size.height * scale)
  sailboat.draw(in: NSRect(x: (1024 - size.width) / 2, y: (1024 - size.height) / 2,
                          width: size.width, height: size.height))
}

for size in [16, 32, 128, 256, 512] {
  for scale in [1, 2] {
    let pixels = size * scale
    let bitmap = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: pixels, pixelsHigh: pixels,
      bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false,
      colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0)!
    NSGraphicsContext.saveGraphicsState()
    NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: bitmap)
    NSGraphicsContext.current?.cgContext.clear(CGRect(x: 0, y: 0, width: pixels, height: pixels))
    let transform = NSAffineTransform()
    transform.scale(by: CGFloat(pixels) / 1024)
    transform.concat()
    drawIcon()
    NSGraphicsContext.restoreGraphicsState()
    let png = bitmap.representation(using: .png, properties: [:])!
    let suffix = scale == 2 ? "@2x" : ""
    try png.write(to: iconset.appendingPathComponent("icon_\(size)x\(size)\(suffix).png"))
    if pixels == 1024 { try png.write(to: output.appendingPathComponent("Harbor.png")) }
  }
}
