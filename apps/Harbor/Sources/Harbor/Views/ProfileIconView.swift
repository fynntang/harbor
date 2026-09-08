import SwiftUI

struct ProfileIconView: View {
  let appPath: String
  let revision: Int
  @State private var icon: NSImage?

  var body: some View {
    Group {
      if let icon {
        Image(nsImage: icon).resizable().scaledToFit()
      } else {
        Image(systemName: "macwindow").resizable().scaledToFit().foregroundStyle(.tint)
      }
    }
    .task(id: "\(appPath):\(revision)") {
      icon =
        NSImage(contentsOfFile: appPath + "/Contents/Resources/icon-chatgpt.png")
        ?? NSWorkspace.shared.icon(forFile: appPath)
    }
  }
}
