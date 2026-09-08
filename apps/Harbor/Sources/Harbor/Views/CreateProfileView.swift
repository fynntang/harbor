import SwiftUI
import UniformTypeIdentifiers

struct CreateProfileView: View {
  @Bindable var store: HarborStore
  @Environment(\.dismiss) private var dismiss
  @State private var name = ""
  @State private var source = "/Applications/ChatGPT.app"

  @State private var useBadge = true
  @State private var badge = BadgeSettings()
  @State private var badgeImage: IconImage?
  @State private var badgeTray: Data?

  var body: some View {
    VStack(alignment: .leading, spacing: 20) {
      Text(store.text("创建独立副本")).font(.title2.bold())
      Text(store.text("从官方原版创建一个新实例，首次启动后登录自己的账号。"))
        .foregroundStyle(.secondary)
      Form {
        TextField(store.text("实例名称"), text: $name, prompt: Text(store.text("例如 work")))
          .accessibilityIdentifier("profile-name")
        Text(store.text("1–48 位小写字母、数字或连字符，以字母或数字开头。"))
          .font(.caption).foregroundStyle(.secondary)
        LabeledContent(store.text("官方原版")) {
          Text(source).lineLimit(2).truncationMode(.middle).help(source)
          Button(store.text("选择…"), action: chooseSource)
        }
      }.disabled(store.busy)
      if validProfileName(name) {
        Text(store.text("副本：~/Applications/Harbor/ChatGPT-\(name).app"))
          .font(.caption).foregroundStyle(.secondary)
      }
      Toggle(store.text("使用名称角标"), isOn: $useBadge).disabled(store.busy)
      if useBadge {
        BadgeEditor(
          store: store, name: name,
          originalURL: URL(fileURLWithPath: source).appendingPathComponent(
            "Contents/Resources/icon-chatgpt.png"),
          settings: $badge, image: $badgeImage, trayPNG: $badgeTray)
      }
      Text(store.text("新副本使用本地签名，数据目录为空。现有副本和账号不会被覆盖。"))
        .font(.callout)
      if let error = store.error {
        ScrollView {
          Text(store.text(error)).foregroundStyle(.orange).textSelection(.enabled)
            .frame(maxWidth: .infinity, alignment: .leading)
        }.frame(maxHeight: 120)
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
        }
        .keyboardShortcut(.cancelAction).disabled(store.busy)
        Button(store.text("创建副本")) {
          Task {
            await store.create(
              name: name, source: source,
              iconPNG: useBadge ? badgeImage?.png : nil, trayPNG: useBadge ? badgeTray : nil)
            if useBadge, store.error == nil, let app = store.selected?.profile.app_bundle {
              badge.save(for: app)
            }
          }
        }
        .keyboardShortcut(.defaultAction).buttonStyle(.borderedProminent)
        .disabled(
          store.busy || !validProfileName(name) || source.isEmpty
            || (useBadge && (badgeImage == nil || badgeTray == nil)))
      }
    }
    .padding(28).frame(width: 570)
    .interactiveDismissDisabled(store.busy)
    .onAppear { store.error = nil }
  }

  private func chooseSource() {
    let panel = NSOpenPanel()
    panel.title = store.text("选择官方 ChatGPT / Codex 原版应用")
    panel.allowedContentTypes = [.applicationBundle]
    panel.canChooseDirectories = false
    panel.allowsMultipleSelection = false
    panel.directoryURL = URL(fileURLWithPath: "/Applications")
    if panel.runModal() == .OK, let url = panel.url { source = url.path }
  }
}
