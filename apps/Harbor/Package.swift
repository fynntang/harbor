// swift-tools-version: 6.0
import PackageDescription

let package = Package(
  name: "Harbor",
  platforms: [.macOS(.v14)],
  products: [
    .executable(name: "Harbor", targets: ["Harbor"]),
    .executable(name: "harbor-native", targets: ["HarborNative"]),
  ],
  targets: [
    .executableTarget(name: "Harbor"),
    .executableTarget(name: "HarborNative"),
    .testTarget(name: "HarborTests", dependencies: ["Harbor", "HarborNative"]),
  ]
)
