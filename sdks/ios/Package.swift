// swift-tools-version: 5.9

import PackageDescription

let package = Package(
    name: "AetherSDK",
    platforms: [
        .iOS(.v15),
        .macOS(.v13),
    ],
    products: [
        .library(name: "AetherSDK", targets: ["AetherSDK"]),
    ],
    targets: [
        .target(
            name: "AetherSDK",
            dependencies: ["AetherFFI"],
            path: "Sources/AetherSDK"
        ),
        .binaryTarget(
            name: "AetherFFI",
            path: "AetherSDK.xcframework"
        ),
    ]
)
