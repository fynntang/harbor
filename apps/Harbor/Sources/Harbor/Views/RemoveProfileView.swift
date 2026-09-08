import SwiftUI

struct RemoveProfileView: View {
  let store: HarborStore
  let item: ProfileItem
  @Environment(\.dismiss) private var dismiss
  @State private var deleteData = false

  var body: some View {
    VStack(alignment: .leading, spacing: 18) {
      Text(store.text("删除实例 \(item.profile.displayName)？")).font(.title2.bold())
      Text(store.text("应用副本会移到系统废纸篓，并从 Harbor 列表中移除。"))
      Text(item.profile.app_bundle).font(.caption).textSelection(.enabled)
      if item.status.state != "stopped" {
        Label(store.text("会先请求正常退出。退出被取消、超时或仍有辅助进程运行时，不会删除。"), systemImage: "info.circle")
          .font(.callout)
      }
      Toggle(store.text("同时将账号数据移到废纸篓"), isOn: $deleteData).disabled(store.busy)
      if deleteData {
        Text(store.text("以下目录包含登录状态、对话和设置；清空废纸篓后将无法通过 Harbor 恢复。"))
          .foregroundStyle(.orange)
        Text(item.profile.codex_home + "\n" + item.profile.gui_home)
          .font(.caption).textSelection(.enabled)
      } else {
        Text(store.text("账号数据保留在 Harbor 的 retained 目录，完成后会显示具体位置。保留的数据可以用于重新接管。"))
          .font(.callout).foregroundStyle(.secondary)
      }
      if let error = store.error {
        ScrollView { Text(store.text(error)).foregroundStyle(.orange).textSelection(.enabled) }
          .frame(
            maxHeight: 120)
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
        Button(
          item.status.state != "stopped" ? store.text("停止并删除") : store.text("移到废纸篓"),
          role: .destructive
        ) {
          Task { if await store.remove(item: item, deleteData: deleteData) { dismiss() } }
        }.disabled(store.busy)
      }
    }
    .padding(28).frame(width: 570)
    .interactiveDismissDisabled(store.busy)
    .onAppear { store.error = nil }
  }
}
