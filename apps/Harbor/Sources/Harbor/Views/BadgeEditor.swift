import SwiftUI

struct BadgeEditor: View {
  let store: HarborStore
  let name: String
  let originalURL: URL
  @Binding var settings: BadgeSettings
  @Binding var image: IconImage?
  @Binding var trayPNG: Data?
  @State private var base: IconImage?
  @State private var error: GUIMessage?

  var body: some View {
    VStack(alignment: .leading, spacing: 12) {
      HStack(spacing: 18) {
        if let image {
          Image(nsImage: image.preview).resizable().scaledToFit().frame(width: 96, height: 96)
          Image(nsImage: image.preview).resizable().scaledToFit().frame(width: 32, height: 32)
        } else {
          Image(systemName: "photo").frame(width: 96, height: 96)
        }
        VStack(alignment: .leading, spacing: 8) {
          TextField(
            store.text("角标简称"), text: $settings.label,
            prompt: Text(settings.text(for: name.isEmpty ? "work" : name))
          )
          .accessibilityIdentifier("badge-label")
          Picker(store.text("角标颜色"), selection: $settings.color) {
            Text(store.text("自动分配")).tag(nil as BadgeColor?)
            ForEach(BadgeColor.allCases) { color in
              Text(store.text(color.title)).tag(Optional(color))
            }
          }.accessibilityIdentifier("badge-color")
          Text(store.text("最多 4 个字符；留空时根据副本名称生成。"))
            .font(.caption).foregroundStyle(settings.isValid ? Color.secondary : Color.orange)
        }
      }
      HStack {
        Text(store.text("菜单栏简称"))
        if let trayPNG, let icon = NSImage(data: trayPNG) {
          Image(nsImage: icon).resizable().renderingMode(.template).scaledToFit().frame(
            width: 18, height: 18)
        }
        Text(store.text("使用角标的前两个字符，保持透明背景。"))
          .font(.caption).foregroundStyle(.secondary)
      }
      if let error { Text(store.text(error)).font(.caption).foregroundStyle(.orange) }
    }
    .disabled(store.busy)
    .task(id: originalURL) {
      base = nil
      do {
        base = try IconImage(url: originalURL)
        render()
      } catch {
        image = nil
        trayPNG = nil
        self.error = "无法读取原版图标，请确认官方应用仍在所选位置。"
      }
    }
    .onChange(of: settings) { _, _ in render() }
    .onChange(of: name) { _, _ in render() }
  }

  private func render() {
    image = nil
    trayPNG = nil
    guard let base else { return }
    guard settings.isValid else {
      error = "角标简称不能超过 4 个字符，也不能包含换行。"
      return
    }
    let name = name.isEmpty ? "work" : name
    do {
      let text = settings.text(for: name)
      image = try base.badged(text: text, color: settings.resolvedColor(for: name))
      trayPNG = try IconImage.menuBarText(text)
      error = nil
    } catch { self.error = GUIMessage(error: error) }
  }
}
