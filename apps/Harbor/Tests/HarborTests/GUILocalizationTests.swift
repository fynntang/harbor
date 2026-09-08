import Foundation
import XCTest

@testable import Harbor

final class GUILocalizationTests: XCTestCase {
  func testSystemLanguageResolutionAndInvalidPreferenceFallback() {
    XCTAssertEqual(
      GUILanguage.initial(saved: nil, preferred: ["zh-Hans-CN", "en-US"]), .simplifiedChinese)
    XCTAssertEqual(GUILanguage.initial(saved: nil, preferred: ["en-GB", "zh-Hans"]), .english)
    XCTAssertEqual(
      GUILanguage.initial(saved: "invalid", preferred: ["fr", "zh-CN"]), .simplifiedChinese)
    XCTAssertEqual(GUILanguage.initial(saved: nil, preferred: ["fr"]), .english)
    XCTAssertEqual(GUILanguage.initial(saved: "en", preferred: ["zh-Hans"]), .english)
  }

  @MainActor
  func testSwitchPersistsAndUpdatesExistingMessagesWithoutResettingStore() throws {
    let suite = "harbor-language-test-\(UUID().uuidString)"
    let defaults = try XCTUnwrap(UserDefaults(suiteName: suite))
    defer { defaults.removePersistentDomain(forName: suite) }
    let language = GUILocalization(defaults: defaults, preferred: ["zh-Hans"])
    let store = HarborStore(client: HarborClient(), language: language)
    store.selection = "work"
    store.creating = true
    store.busy = true
    store.activity = HarborStore.stageTitle("copy")
    store.message = "启动成功，请在副本窗口继续（进程 \(42)）。"
    store.error = GUIMessage(error: CLIError(message: "操作未完成（退出码 \(2)）。"))
    XCTAssertEqual(store.text(store.activity), "正在复制应用…")
    language.selection = .english
    XCTAssertEqual(store.text(store.activity), "Copying the app…")
    XCTAssertEqual(
      store.text(store.message), "Started successfully. Continue in the copy's window (process 42)."
    )
    XCTAssertEqual(
      store.text(try XCTUnwrap(store.error)), "The operation did not complete (exit code 2).")
    XCTAssertEqual(store.selection, "work")
    XCTAssertTrue(store.creating)
    XCTAssertTrue(store.busy)
    XCTAssertEqual(GUILocalization(defaults: defaults, preferred: ["zh-Hans"]).selection, .english)
    language.selection = .simplifiedChinese
    XCTAssertEqual(store.text(store.message), "启动成功，请在副本窗口继续（进程 42）。")
    XCTAssertEqual(
      GUILocalization(defaults: defaults, preferred: ["en"]).selection, .simplifiedChinese)
  }

  func testUserValuesAndBackendDiagnosticsRemainVerbatim() {
    let path = "/tmp/中文/%@/100% work"
    let message: GUIMessage = "路径不存在：\(path)"
    XCTAssertEqual(message.rendered(language: .english), "Path does not exist: " + path)
    let raw = GUIMessage(verbatim: "操作未完成")
    XCTAssertEqual(raw.rendered(language: .english), "操作未完成")
    XCTAssertEqual(
      GUIMessage(key: "unrecognized diagnostic").rendered(language: .english),
      "unrecognized diagnostic")
  }

  @MainActor
  func testCatalogPreservesArgumentsAndTranslatesEveryProcessAndProgressState() {
    for (key, translation) in GUILocalization.english {
      XCTAssertFalse(translation.isEmpty, key)
      XCTAssertEqual(
        key.components(separatedBy: "%@").count, translation.components(separatedBy: "%@").count,
        key)
    }
    for stage in [
      "verify_source", "copy", "verify_copy", "sign", "verify_signature", "complete", "unknown",
    ] {
      XCTAssertNotNil(GUILocalization.english[HarborStore.stageTitle(stage).key])
    }
    for state in ["running", "stopped", "helpers_running", "conflict", "unknown"] {
      let title = ProcessStatus(state: state, pids: [], error: nil).title
      XCTAssertNotNil(GUILocalization.english[title])
    }
  }
}
