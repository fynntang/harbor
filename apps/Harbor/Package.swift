// swift-tools-version: 6.0
import PackageDescription

let package = Package(
  name: "Harbor",
  platforms: [.macOS(.v14)],
  products: [
    .executable(name: "Harbor", targets: ["Harbor"]),
    .executable(name: "harbor-native", targets: ["HarborNative"]),
  ],
  dependencies: [.package(url: "https://github.com/sparkle-project/Sparkle", exact: "2.9.6")],
  targets: [
    .executableTarget(
      name: "Harbor",
      dependencies: [.product(name: "Sparkle", package: "Sparkle")],
      linkerSettings: [.unsafeFlags(["-Xlinker", "-rpath", "-Xlinker", "@executable_path/../Frameworks"])]
    ),
    .executableTarget(name: "HarborNative"),
    .testTarget(name: "HarborTests", dependencies: ["Harbor", "HarborNative"]),
  ]
)
