import AppKit

// Fixed sRGB colors keep generated icons identical across system appearances.
enum BadgeColor: String, CaseIterable, Codable, Identifiable {
  case blue, green, orange, purple, pink, yellow
  var id: String { rawValue }
  var title: GUIMessage {
    switch self {
    case .blue: "蓝色"
    case .green: "绿色"
    case .orange: "橙色"
    case .purple: "紫色"
    case .pink: "粉色"
    case .yellow: "黄色"
    }
  }
  var rgb: (CGFloat, CGFloat, CGFloat) {
    switch self {
    case .blue: (0.08, 0.35, 0.85)
    case .green: (0.05, 0.42, 0.30)
    case .orange: (0.95, 0.45, 0.12)
    case .purple: (0.48, 0.24, 0.78)
    case .pink: (0.78, 0.16, 0.40)
    case .yellow: (0.98, 0.76, 0.18)
    }
  }
  var background: NSColor {
    let (r, g, b) = rgb
    return NSColor(srgbRed: r, green: g, blue: b, alpha: 1)
  }
  var foreground: NSColor { self == .orange || self == .yellow ? .black : .white }
  static func automatic(for name: String) -> Self {
    allCases[name.utf8.reduce(0) { ($0 * 31 + Int($1)) % allCases.count }]
  }
}

struct BadgeSettings: Codable, Equatable {
  var label = ""
  var color: BadgeColor?
  var isValid: Bool {
    label.trimmingCharacters(in: .whitespaces).count <= 4
      && !label.unicodeScalars.contains { CharacterSet.controlCharacters.contains($0) }
  }
  func text(for name: String) -> String {
    let custom = label.trimmingCharacters(in: .whitespaces)
    let value = custom.isEmpty ? String(name.prefix(name.count <= 4 ? 4 : 3)) : custom
    return String(value.uppercased().prefix(4))
  }
  func resolvedColor(for name: String) -> BadgeColor { color ?? .automatic(for: name) }
  static func load(for app: String, defaults: UserDefaults = .standard) -> Self {
    guard let data = defaults.data(forKey: "HarborBadge:" + app),
      let value = try? JSONDecoder().decode(Self.self, from: data), value.isValid
    else { return Self() }
    return value
  }
  func save(for app: String, defaults: UserDefaults = .standard) {
    if let data = try? JSONEncoder().encode(self) {
      defaults.set(data, forKey: "HarborBadge:" + app)
    }
  }
}

extension IconImage {
  static func originalURL(for app: String) -> URL {
    let saved = URL(fileURLWithPath: app).appendingPathComponent(
      "Contents/Resources/harbor-original-icon.png")
    if FileManager.default.fileExists(atPath: saved.path) { return saved }
    // Older copies may already have custom icons: never use their painted icon as a base.
    return URL(fileURLWithPath: "/Applications/ChatGPT.app/Contents/Resources/icon-chatgpt.png")
  }

  func badged(text: String, color: BadgeColor) throws -> IconImage {
    guard !text.isEmpty,
      let bitmap = NSBitmapImageRep(
        bitmapDataPlanes: nil, pixelsWide: 1024, pixelsHigh: 1024,
        bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false,
        colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0),
      let context = NSGraphicsContext(bitmapImageRep: bitmap)
    else { throw CLIError(message: "无法生成角标图标。") }
    NSGraphicsContext.saveGraphicsState()
    defer { NSGraphicsContext.restoreGraphicsState() }
    NSGraphicsContext.current = context
    preview.draw(
      in: NSRect(x: 0, y: 0, width: 1024, height: 1024), from: .zero, operation: .copy, fraction: 1)
    let rect = NSRect(x: 470, y: 92, width: 470, height: 240)
    let badge = NSBezierPath(roundedRect: rect, xRadius: 64, yRadius: 64)
    color.background.setFill()
    badge.fill()
    NSColor.white.withAlphaComponent(0.9).setStroke()
    badge.lineWidth = 10
    badge.stroke()
    let font = NSFont.systemFont(ofSize: 140, weight: .heavy)
    let initial = NSAttributedString(string: text, attributes: [.font: font])
    let size = min(140, 390 / max(initial.size().width, 1) * 140)
    let label = NSAttributedString(
      string: text,
      attributes: [
        .font: NSFont.systemFont(ofSize: size, weight: .heavy), .foregroundColor: color.foreground,
      ])
    let bounds = label.size()
    label.draw(at: NSPoint(x: rect.midX - bounds.width / 2, y: rect.midY - bounds.height / 2))
    guard let data = bitmap.representation(using: .png, properties: [:]),
      let image = NSImage(data: data)
    else {
      throw CLIError(message: "无法生成角标图标。")
    }
    return IconImage(png: data, preview: image)
  }
}
