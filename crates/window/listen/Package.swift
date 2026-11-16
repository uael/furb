// swift-tools-version: 6.0
// The program of the speech of the window: Whisper through WhisperKit, which build.rs builds.
import PackageDescription

let package = Package(
  name: "listen",
  platforms: [.macOS(.v14)],
  dependencies: [.package(url: "https://github.com/argmaxinc/WhisperKit", exact: "1.1.1")],
  targets: [
    .executableTarget(
      name: "listen", dependencies: [.product(name: "WhisperKit", package: "WhisperKit")], path: "Sources")
  ],
  // WhisperKit is not Sendable, and one task at a time reads with it.
  swiftLanguageModes: [.v5]
)
