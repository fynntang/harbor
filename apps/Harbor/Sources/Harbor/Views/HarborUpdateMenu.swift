import SwiftUI

struct HarborUpdateMenu: View {
  let store: HarborStore
  @ObservedObject var updater: HarborUpdater

  var body: some View {
    Button(store.text("检查 Harbor 更新…")) { updater.check() }
      .disabled(store.busy || !updater.canCheck)
    Toggle(store.text("自动检查 Harbor 更新"), isOn: Binding(
      get: { updater.automaticChecks }, set: { updater.setAutomaticChecks($0) }))
      .disabled(!updater.enabled)
    if !updater.enabled {
      Text(store.text("开发版未启用自动更新"))
    }
  }
}
