import Combine
import Sparkle

/// One updater for Harbor itself; it never updates managed client copies.
@MainActor
final class HarborUpdater: NSObject, ObservableObject, SPUUpdaterDelegate {
  @Published private(set) var canCheck = false
  @Published private(set) var automaticChecks = false
  private var observations: Set<AnyCancellable> = []
  private weak var store: HarborStore?
  private var started = false
  private var controller: SPUStandardUpdaterController?

  init(store: HarborStore, enabled: Bool = Bundle.main.object(forInfoDictionaryKey: "HarborUpdatesEnabled") as? Bool == true) {
    self.store = store
    super.init()
    guard enabled else { return }
    let controller = SPUStandardUpdaterController(
      startingUpdater: false, updaterDelegate: self, userDriverDelegate: nil)
    self.controller = controller
    controller.updater.publisher(for: \.canCheckForUpdates)
      .sink { [weak self] in self?.canCheck = $0 }.store(in: &observations)
    controller.updater.publisher(for: \.automaticallyChecksForUpdates)
      .sink { [weak self] in self?.automaticChecks = $0 }.store(in: &observations)
  }

  var enabled: Bool { controller != nil }
  func start() {
    guard !started else { return }
    started = true
    controller?.startUpdater()
  }
  func check() {
    guard store?.busy != true, canCheck else { return }
    controller?.checkForUpdates(nil)
  }
  func setAutomaticChecks(_ value: Bool) {
    controller?.updater.automaticallyChecksForUpdates = value
  }
  func updater(_ updater: SPUUpdater, mayPerform updateCheck: SPUUpdateCheck) throws {
    try requireIdle()
  }
  func requireIdle() throws {
    if store?.busy == true {
      throw NSError(domain: "local.harbor.desktop", code: 1, userInfo: [
        NSLocalizedDescriptionKey: store?.text("请等当前操作完成后再更新 Harbor。") ?? "Harbor is busy."
      ])
    }
  }

  func updater(_ updater: SPUUpdater, shouldPostponeRelaunchForUpdate item: SUAppcastItem,
               untilInvokingBlock installHandler: @escaping () -> Void) -> Bool {
    postponeRelaunch(installHandler)
  }

  func postponeRelaunch(_ installHandler: @escaping () -> Void) -> Bool {
    guard store?.busy == true else { return false }
    Task { @MainActor [weak self] in
      while self?.store?.busy == true {
        try? await Task.sleep(for: .milliseconds(250))
      }
      installHandler()
    }
    return true
  }
}
