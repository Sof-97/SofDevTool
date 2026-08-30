import Carbon
import Foundation

struct ShortcutConfiguration: Equatable {
    let keyCode: UInt32
    let carbonModifiers: UInt32
    let displayName: String

    static let defaultLauncher = ShortcutConfiguration(
        keyCode: UInt32(kVK_Space),
        carbonModifiers: UInt32(controlKey | optionKey),
        displayName: "⌃⌥Space"
    )
}

@MainActor
final class GlobalShortcutController {
    nonisolated(unsafe) private static weak var active: GlobalShortcutController?
    private var hotKey: EventHotKeyRef?
    private var eventHandler: EventHandlerRef?
    private(set) var configuration = ShortcutConfiguration.defaultLauncher
    private let handler: @MainActor () -> Void
    private let defaults: UserDefaults

    init(defaults: UserDefaults = .standard, handler: @escaping @MainActor () -> Void) {
        self.defaults = defaults
        self.handler = handler
        Self.active = self
        var eventType = EventTypeSpec(
            eventClass: OSType(kEventClassKeyboard), eventKind: UInt32(kEventHotKeyPressed))
        InstallEventHandler(
            GetApplicationEventTarget(),
            { _, _, _ in
                Task { @MainActor in GlobalShortcutController.active?.handler() }
                return noErr
            }, 1, &eventType, nil, &eventHandler)
    }

    func registerDefault() -> String? { register(.defaultLauncher) }

    func registerSavedOrDefault() -> String? {
        if defaults.object(forKey: "launcher.shortcut.keyCode") != nil {
            let saved = ShortcutConfiguration(
                keyCode: UInt32(defaults.integer(forKey: "launcher.shortcut.keyCode")),
                carbonModifiers: UInt32(defaults.integer(forKey: "launcher.shortcut.modifiers")),
                displayName: defaults.string(forKey: "launcher.shortcut.displayName") ?? "Saved shortcut")
            if register(saved) == nil { return nil }
        }
        return registerDefault()
    }

    func register(_ newConfiguration: ShortcutConfiguration) -> String? {
        var candidate: EventHotKeyRef?
        let identifier = EventHotKeyID(signature: fourCharCode("SDTL"), id: 1)
        let status = RegisterEventHotKey(
            newConfiguration.keyCode, newConfiguration.carbonModifiers, identifier,
            GetApplicationEventTarget(), 0, &candidate)
        guard status == noErr, let candidate else {
            return "That shortcut could not be registered. The previous shortcut remains active."
        }
        if let hotKey { UnregisterEventHotKey(hotKey) }
        hotKey = candidate
        configuration = newConfiguration
        defaults.set(Int(newConfiguration.keyCode), forKey: "launcher.shortcut.keyCode")
        defaults.set(Int(newConfiguration.carbonModifiers), forKey: "launcher.shortcut.modifiers")
        defaults.set(newConfiguration.displayName, forKey: "launcher.shortcut.displayName")
        return nil
    }

    private func fourCharCode(_ value: String) -> OSType {
        value.utf8.reduce(0) { ($0 << 8) + OSType($1) }
    }
}
