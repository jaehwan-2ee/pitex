// swift-tools-version: 6.0

import PackageDescription

let package = Package(
    name: "TexCore",
    platforms: [
        .macOS(.v15)
    ],
    products: [
        .library(name: "TexDomain", targets: ["TexDomain"]),
        .library(name: "ProjectCore", targets: ["ProjectCore"]),
        .library(name: "DocumentSessionCore", targets: ["DocumentSessionCore"]),
        .library(name: "LanguageCore", targets: ["LanguageCore"]),
        .library(name: "BuildCore", targets: ["BuildCore"]),
        .library(name: "SyncTeXCore", targets: ["SyncTeXCore"]),
        .library(name: "AICore", targets: ["AICore"]),
        .library(name: "GitCore", targets: ["GitCore"]),
        .library(name: "RemoteCore", targets: ["RemoteCore"]),
        .library(name: "ParityKit", targets: ["ParityKit"])
    ],
    targets: [
        .target(name: "TexDomain"),
        .target(name: "ProjectCore", dependencies: ["TexDomain"]),
        .target(
            name: "DocumentSessionCore",
            dependencies: ["TexDomain", "ProjectCore"]
        ),
        .target(name: "LanguageCore", dependencies: ["TexDomain"]),
        .target(
            name: "BuildCore",
            dependencies: ["TexDomain", "ProjectCore"]
        ),
        .target(
            name: "SyncTeXCore",
            dependencies: ["TexDomain", "ProjectCore"]
        ),
        .target(
            name: "AICore",
            dependencies: ["TexDomain", "DocumentSessionCore"]
        ),
        .target(name: "GitCore"),
        .target(name: "RemoteCore", dependencies: ["BuildCore"]),
        .target(name: "ParityKit", dependencies: ["TexDomain"]),
        .testTarget(
            name: "TexCoreTests",
            dependencies: [
                "TexDomain",
                "ProjectCore",
                "DocumentSessionCore",
                "LanguageCore",
                "BuildCore",
                "SyncTeXCore",
                "AICore",
                "GitCore",
                "ParityKit",
                "RemoteCore"
            ]
        )
    ],
    swiftLanguageModes: [.v6]
)
