import SwiftUI
import UniformTypeIdentifiers

struct ChangeIconView: View {
  let store: HarborStore
  let item: ProfileItem
  @Environment(\.dismiss) private var dismiss
  @State private var image: IconImage?
  @State private var error: GUIMessage?
  @State private var useArtwork = false
  @State private var letterPNG: Data?

  var body: some View {
    VStack(alignment: .leading, spacing: 20) {
      Text(store.text("更换 \(item.id) 的图标")).font(.title2.bold())
      HStack(spacing: 20) {
        if let image {
          Image(nsImage: image.preview).resizable().scaledToFit().frame(width: 96, height: 96)
        } else {
          Image(systemName: "photo").font(.system(size: 48)).foregroundStyle(.secondary)
            .frame(width: 96, height: 96)
        }
        VStack(alignment: .leading, spacing: 8) {
          Button(store.text("选择图片…"), action: chooseImage).disabled(store.busy)
          Text(store.text("PNG、JPEG 或 ICNS，最大 16 MB。\n非正方形图片会保留比例并补透明边。"))
            .font(.caption).foregroundStyle(.secondary)
        }
      }
      HStack(spacing: 16) {
        Picker(store.text("菜单栏"), selection: $useArtwork) {
          Text(store.text("实例首字母")).tag(false)
          Text(store.text("所选图片轮廓")).tag(true)
        }.disabled(store.busy)
        if let data = useArtwork ? image?.png : letterPNG, let icon = NSImage(data: data) {
          Image(nsImage: icon).resizable().renderingMode(.template).scaledToFit()
            .frame(width: 18, height: 18).padding(8).background(
              .quaternary, in: RoundedRectangle(cornerRadius: 6))
        }
      }
      Text(store.text("应用到 Finder、Dock 和顶部菜单栏。请先退出该实例，更换后重新启动生效。"))
      if useArtwork {
        Text(store.text("轮廓由透明度决定；不透明照片会显示为实心方块，建议使用透明背景的简洁图形。"))
          .font(.caption).foregroundStyle(.secondary)
      }
      if let error = error ?? store.error {
        ScrollView { Text(store.text(error)).foregroundStyle(.orange).textSelection(.enabled) }
          .frame(maxHeight: 100)
      }
      HStack {
        if store.busy {
          ProgressView().controlSize(.small)
          Text(store.text(store.activity))
        }
        Spacer()
        Button(store.text("取消")) {
          store.error = nil
          dismiss()
        }.keyboardShortcut(.cancelAction).disabled(store.busy)
        Button(store.text("应用图标")) {
          guard let image, let tray = useArtwork ? image.png : letterPNG else { return }
          Task {
            if await store.changeIcon(item: item, png: image.png, trayPNG: tray) { dismiss() }
          }
        }
        .keyboardShortcut(.defaultAction).buttonStyle(.borderedProminent)
        .disabled(image == nil || letterPNG == nil || store.busy)
      }
    }
    .padding(28).frame(width: 540)
    .interactiveDismissDisabled(store.busy)
    .onAppear {
      store.error = nil
      do { letterPNG = try IconImage.menuBarLetter(item.id) } catch {
        self.error = GUIMessage(error: error)
      }
    }
  }

  private func chooseImage() {
    let panel = NSOpenPanel()
    panel.title = store.text("选择实例图标")
    panel.allowedContentTypes = [.png, .jpeg, .icns]
    panel.allowsMultipleSelection = false
    panel.canChooseDirectories = false
    guard panel.runModal() == .OK, let url = panel.url else { return }
    do {
      image = try IconImage(url: url)
      error = nil
    } catch { self.error = GUIMessage(error: error) }
  }
}
