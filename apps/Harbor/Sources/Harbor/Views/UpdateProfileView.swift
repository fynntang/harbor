import SwiftUI
import UniformTypeIdentifiers

struct UpdateProfileView: View {
  let store: HarborStore
  let item: ProfileItem
  @Environment(\.dismiss) private var dismiss
  @State private var source = "/Applications/ChatGPT.app"
  @State private var sourceVersion = ""

  var body: some View {
    VStack(alignment: .leading, spacing: 18) {
      Text(store.text("更新副本 \(item.profile.displayName)")).font(.title2.bold())
      LabeledContent(store.text("登记版本"), value:
        "\(item.profile.registered_app_version) (\(item.profile.registered_app_build_version ?? "—"))")
      LabeledContent(store.text("官方原版")) {
        Text(source).lineLimit(2).truncationMode(.middle)
        Button(store.text("选择…")) {
          let panel = NSOpenPanel()
          panel.allowedContentTypes = [.applicationBundle]
          panel.allowsMultipleSelection = false
          if panel.runModal() == .OK, let url = panel.url { source = url.path }
        }.disabled(store.busy)
      }
      LabeledContent(store.text("来源版本"), value: sourceVersion.isEmpty ? store.text("无法读取") : sourceVersion)
      Text(store.text("保留账号数据、名称和图标。请先退出副本及浏览器辅助进程；更新完成后不会自动启动。"))
      Text(store.text("更新只替换应用程序，不备份账号数据库。新版首次启动可能迁移数据，不支持自动降级。"))
        .font(.caption).foregroundStyle(.secondary)
      if let error = store.error {
        ScrollView { Text(store.text(error)).foregroundStyle(.orange).textSelection(.enabled) }
          .frame(maxHeight: 120)
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
        Button(store.text("更新副本")) {
          Task { if await store.update(item: item, source: source) { dismiss() } }
        }.buttonStyle(.borderedProminent).keyboardShortcut(.defaultAction)
          .disabled(store.busy || source.isEmpty)
      }
    }
    .padding(28).frame(width: 560)
    .interactiveDismissDisabled(store.busy)
    .onAppear { store.error = nil }
    .task(id: source) {
      let url = URL(fileURLWithPath: source).appendingPathComponent("Contents/Info.plist")
      if let data = try? Data(contentsOf: url),
        let info = try? PropertyListSerialization.propertyList(from: data, format: nil) as? [String: Any],
        let version = info["CFBundleShortVersionString"] as? String,
        let build = info["CFBundleVersion"] as? String {
        sourceVersion = "\(version) (\(build))"
      } else { sourceVersion = "" }
    }
  }
}
