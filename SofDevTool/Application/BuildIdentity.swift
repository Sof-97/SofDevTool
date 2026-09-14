import Foundation

enum BuildChannel: String, Sendable {
    case debug = "Debug"
    case release = "Release"
}

struct BuildIdentity: Equatable, Sendable {
    let channel: BuildChannel
    let version: String
    let build: String

    static var current: BuildIdentity {
        #if DEBUG
            let channel = BuildChannel.debug
        #else
            let channel = BuildChannel.release
        #endif

        return BuildIdentity(
            channel: channel,
            infoDictionary: Bundle.main.infoDictionary ?? [:]
        )
    }

    init(channel: BuildChannel, infoDictionary: [String: Any]) {
        self.channel = channel
        version = infoDictionary["CFBundleShortVersionString"] as? String ?? "Unknown"
        build = infoDictionary["CFBundleVersion"] as? String ?? "Unknown"
    }

    var isDevelopment: Bool { channel == .debug }

    var versionDescription: String { "\(version) (\(build))" }
}
