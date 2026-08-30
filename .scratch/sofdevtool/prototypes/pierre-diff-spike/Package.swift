// swift-tools-version: 6.0

import PackageDescription

let package = Package(
    name: "PierreDiffSpike",
    platforms: [.macOS(.v14)],
    dependencies: [
        .package(path: "Vendor/PierreDiffsSwift")
    ],
    targets: [
        .target(
            name: "PierreDiffSpikeSupport",
            dependencies: ["PierreDiffsSwift"]
        ),
        .executableTarget(
            name: "PierreDiffSpikeApp",
            dependencies: ["PierreDiffsSwift", "PierreDiffSpikeSupport"]
        ),
        .testTarget(
            name: "PierreDiffSpikeTests",
            dependencies: ["PierreDiffSpikeSupport"]
        )
    ]
)
