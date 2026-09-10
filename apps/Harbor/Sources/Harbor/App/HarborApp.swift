import SwiftUI

@main
struct HarborApp: App {
  @NSApplicationDelegateAdaptor(AppDelegate.self) private var delegate
  @State private var store: HarborStore
  @StateObject private var updater: HarborUpdater

  init() {
    // Explicit alternate registry for isolated acceptance runs; default is the CLI default.
    let args = CommandLine.arguments
    let index = args.firstIndex(of: "--registry")
    let root = index.flatMap { $0 + 1 < args.count ? args[$0 + 1] : nil }
    let store = HarborStore(client: HarborClient(root: root))
    _store = State(initialValue: store)
    _updater = StateObject(wrappedValue: HarborUpdater(store: store))
  }

  var body: some Scene {
    WindowGroup("Harbor", id: "main", for: String.self) { _ in
      ContentView(store: store)
        .environment(\.locale, store.language.selection.locale)
        .frame(minWidth: 820, minHeight: 560)
        .onAppear { delegate.store = store; updater.start() }
    } defaultValue: {
      "main"
    }
    .defaultSize(width: 1000, height: 660)
    .commands { HarborCommands(store: store, updater: updater) }
    MenuBarExtra("Harbor", systemImage: "sailboat") {
      HarborMenuView(store: store, updater: updater)
        .environment(\.locale, store.language.selection.locale)
    }
    .menuBarExtraStyle(.menu)
  }
}

@MainActor
final class AppDelegate: NSObject, NSApplicationDelegate {
  weak var store: HarborStore?
  func applicationDidFinishLaunching(_ notification: Notification) {
    NSApp.setActivationPolicy(.regular)
    if let url = Bundle.main.url(forResource: "Harbor", withExtension: "icns"),
      let icon = NSImage(contentsOf: url)
    {
      NSApp.applicationIconImage = icon
    }
    NSApp.activate(ignoringOtherApps: true)
  }
  func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool { false }

  func applicationShouldTerminate(_ sender: NSApplication) -> NSApplication.TerminateReply {
    guard store?.busy == true else { return .terminateNow }
    let localization = store?.language ?? GUILocalization()
    let alert = NSAlert()
    alert.messageText = localization.text("操作仍在进行")
    alert.informativeText = localization.text("请等当前操作完成后再退出 Harbor。")
    alert.addButton(withTitle: localization.text("继续等待"))
    alert.runModal()
    return .terminateCancel
  }
}
