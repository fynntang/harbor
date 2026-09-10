import XCTest
import Sparkle
@testable import Harbor

final class HarborUpdaterTests: XCTestCase {
  func testCalendarBuildOrderUsesSparkleComparator() {
    let versions = ["0.0.1", "26.9.10000", "26.9.10001", "26.9.10059", "26.9.10100", "26.9.92359", "26.9.100000", "26.9.101156", "26.10.10000", "27.1.10000"]
    let comparator = SUStandardVersionComparator()
    for (older, newer) in zip(versions, versions.dropFirst()) {
      XCTAssertEqual(comparator.compareVersion(older, toVersion: newer), .orderedAscending)
      XCTAssertEqual(comparator.compareVersion(newer, toVersion: older), .orderedDescending)
    }
    XCTAssertEqual(comparator.compareVersion("26.9.101156", toVersion: "26.9.101156"), .orderedSame)
  }

  @MainActor
  func testRelaunchWaitsForCurrentOperation() async {
    let store = HarborStore(client: HarborClient())
    let updater = HarborUpdater(store: store, enabled: false)
    XCTAssertFalse(updater.postponeRelaunch { XCTFail("Idle callback belongs to Sparkle") })
    store.busy = true
    var resumed = false
    XCTAssertTrue(updater.postponeRelaunch { resumed = true })
    await Task.yield()
    XCTAssertFalse(resumed)
    store.busy = false
    for _ in 0..<20 where !resumed {
      try? await Task.sleep(for: .milliseconds(50))
    }
    XCTAssertTrue(resumed)
  }

  @MainActor
  func testDevelopmentUpdaterIsDisabledAndBusyOperationsBlockChecks() throws {
    let store = HarborStore(client: HarborClient())
    let updater = HarborUpdater(store: store, enabled: false)
    updater.start()
    updater.start()
    XCTAssertFalse(updater.enabled)
    XCTAssertFalse(updater.canCheck)
    XCTAssertNoThrow(try updater.requireIdle())
    store.busy = true
    XCTAssertThrowsError(try updater.requireIdle())
    updater.check()
    XCTAssertTrue(store.busy)
    store.busy = false
    XCTAssertNoThrow(try updater.requireIdle())
  }
}
