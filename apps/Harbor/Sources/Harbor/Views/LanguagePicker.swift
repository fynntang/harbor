import SwiftUI

struct LanguagePicker: View {
  @Bindable var language: GUILocalization

  var body: some View {
    Picker(language.text("语言"), selection: $language.selection) {
      ForEach(GUILanguage.allCases) { value in
        Text(value.title).tag(value)
      }
    }
    .accessibilityIdentifier("gui-language")
  }
}
