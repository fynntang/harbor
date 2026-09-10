import SwiftUI

struct HarborMenuView: View {
  let store: HarborStore
  @ObservedObject var updater: HarborUpdater
  @Environment(\.openWindow) private var openWindow

  var body: some View {
    Button(store.text("打开 Harbor")) { showWindow() }
    Button(store.text("创建副本…")) {
      showWindow()
      store.creating = true
    }
    .disabled(store.busy)
    if store.profiles.contains(where: { store.updateAvailable(for: $0) }) {
      Button(store.text("官方原版有更新"), systemImage: "arrow.down.circle.fill") {
        store.selection = store.profiles.first(where: { store.updateAvailable(for: $0) })?.id
        showWindow()
      }
    }
    LanguagePicker(language: store.language)
    HarborUpdateMenu(store: store, updater: updater)
    if store.busy {
      Divider()
      Text(store.text(store.activity))
    }
    Divider()
    Button(store.text("退出 Harbor")) { NSApplication.shared.terminate(nil) }
      .keyboardShortcut("q")
      .disabled(store.busy)
  }

  private func showWindow() {
    openWindow(id: "main", value: "main")
    NSApp.activate(ignoringOtherApps: true)
  }
}
