import SwiftUI

struct HarborMenuView: View {
  let store: HarborStore
  @Environment(\.openWindow) private var openWindow

  var body: some View {
    Button(store.text("打开 Harbor")) { showWindow() }
    Button(store.text("创建副本…")) {
      showWindow()
      store.creating = true
    }
    .disabled(store.busy)
    LanguagePicker(language: store.language)
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
