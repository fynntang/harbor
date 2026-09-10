import SwiftUI

struct DetailView: View {
  let store: HarborStore
  let item: ProfileItem
  @State private var changingIcon = false
  @State private var removing = false
  @State private var updating = false

  var body: some View {
    ScrollView {
      VStack(alignment: .leading, spacing: 24) {
        HStack(spacing: 16) {
          ProfileIconView(appPath: item.profile.app_bundle, revision: store.iconRevision).frame(
            width: 56, height: 56)
          VStack(alignment: .leading, spacing: 5) {
            Text(item.profile.displayName).font(.largeTitle.bold())
            Text(store.text(key: item.status.title)).foregroundStyle(
              item.status.state == "running" ? .green : .secondary)
          }
          Spacer()
          Button(store.text("启动"), systemImage: "play.fill") { Task { await store.start() } }
            .buttonStyle(.borderedProminent)
            .disabled(store.busy || item.issue != nil || item.status.state == "conflict")
          Button(store.text(item.status.state == "helpers_running" ? "清理残留" : "停止"), systemImage: "stop.fill") { Task { await store.stop() } }
            .disabled(store.busy || !["running", "helpers_running"].contains(item.status.state))
          Button(store.text("检查"), systemImage: "checkmark.shield") { Task { await store.check() } }
            .disabled(store.busy)
        }
        if let issue = item.issue ?? item.status.error {
          Label(issue, systemImage: "exclamationmark.triangle")
            .foregroundStyle(.orange).textSelection(.enabled)
        }
        if item.status.state == "helpers_running" {
          Text(store.text("副本已退出，但辅助进程仍在运行。清理残留会暂停该实例的浏览器连接并请求已知辅助进程退出，不会退出浏览器；通过 Harbor 启动副本时恢复连接注册。"))
            .font(.callout).foregroundStyle(.orange)
        }
        GroupBox(store.text("应用")) {
          VStack(alignment: .leading, spacing: 12) {
            LabeledContent(store.text("实例标识"), value: item.id)
            LabeledContent("Bundle ID", value: item.profile.bundle_id ?? "—")
            LabeledContent(store.text("当前版本"), value: version)
            LabeledContent(
              store.text("登记版本"),
              value:
                "\(item.profile.registered_app_version) (\(item.profile.registered_app_build_version ?? store.text("未记录构建号")))"
            )
            pathRow(store.text("副本位置"), item.profile.app_bundle)
            Button(store.text("更新副本…")) { updating = true }
              .disabled(store.busy || !item.profile.supportsCustomIcon || item.issue != nil
                || item.status.state != "stopped")
              .help(store.text("请先退出副本及其辅助进程，再从官方原版更新"))
            Button(store.text("更换图标…")) { changingIcon = true }
              .disabled(
                store.busy || !item.profile.supportsCustomIcon || item.issue != nil
                  || item.status.state != "stopped"
              )
              .help(
                item.profile.supportsCustomIcon
                  ? store.text("请先退出该实例，再更换图标") : store.text("仅支持 Harbor 创建的本地副本"))
          }.padding(8)
        }
        GroupBox(store.text("独立数据目录")) {
          VStack(alignment: .leading, spacing: 12) {
            pathRow("Codex", item.profile.codex_home)
            pathRow(store.text("界面数据"), item.profile.gui_home)
            Button(store.text("在 Finder 中显示日志目录")) {
              store.reveal((item.profile.codex_home as NSString).deletingLastPathComponent)
            }.buttonStyle(.link)
          }.padding(8)
        }
        if !store.message.isEmpty {
          Label(store.text(store.message), systemImage: "checkmark.circle").foregroundStyle(.green)
        }
        if let report = store.report {
          GroupBox {
            VStack(alignment: .leading, spacing: 10) {
              Label(
                report.passed ? store.text("检查完成") : store.text("检查发现问题"),
                systemImage: report.passed ? "checkmark.shield" : "exclamationmark.triangle"
              )
              .font(.headline).foregroundStyle(report.passed ? .green : .orange)
              Text(report.report + (report.error.map { "\n\($0)" } ?? ""))
                .font(.system(.caption, design: .monospaced)).textSelection(.enabled)
                .frame(maxWidth: .infinity, alignment: .leading)
            }.padding(8)
          }
        }
        Button(store.text("删除实例…"), role: .destructive) { removing = true }
          .disabled(
            store.busy || !item.profile.supportsCustomIcon
              || !["running", "helpers_running", "stopped"].contains(item.status.state)
          )
          .help(store.text("仅支持删除 Harbor 创建的本地副本；账号数据默认保留"))
        Text(store.text("账号登录在副本窗口中完成。HOME、SSH、Docker 和系统钥匙串仍共享。"))
          .font(.caption).foregroundStyle(.secondary)
      }.padding(28)
    }
    .sheet(isPresented: $changingIcon) { ChangeIconView(store: store, item: item) }
    .sheet(isPresented: $updating) { UpdateProfileView(store: store, item: item) }
    .sheet(isPresented: $removing) { RemoveProfileView(store: store, item: item) }
  }

  private var version: String {
    guard let version = item.current_version else { return store.text("无法读取") }
    return "\(version) (\(item.current_build ?? store.text("未知构建号")))"
  }

  private func pathRow(_ title: String, _ path: String) -> some View {
    HStack(alignment: .top) {
      Text(title).foregroundStyle(.secondary).frame(width: 100, alignment: .leading)
      Text(path).font(.callout).textSelection(.enabled).frame(
        maxWidth: .infinity, alignment: .leading)
      Button {
        store.reveal(path)
      } label: {
        Image(systemName: "folder")
      }
      .help(store.text("在 Finder 中显示\(title)")).buttonStyle(.borderless)
    }
  }
}
