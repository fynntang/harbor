import SwiftUI

struct HarborCommands: Commands {
  let store: HarborStore
  @ObservedObject var updater: HarborUpdater
  @Environment(\.openWindow) private var openWindow

  var body: some Commands {
    CommandGroup(after: .appSettings) {
      LanguagePicker(language: store.language)
      HarborUpdateMenu(store: store, updater: updater)
    }
    CommandGroup(replacing: .newItem) {
      Button(store.text("创建副本…")) {
        openWindow(id: "main", value: "main")
        NSApp.activate(ignoringOtherApps: true)
        store.creating = true
      }.keyboardShortcut("n").disabled(store.busy)
      Button(store.text("刷新实例")) { Task { await store.refresh() } }
        .keyboardShortcut("r").disabled(store.busy)
    }
  }
}
