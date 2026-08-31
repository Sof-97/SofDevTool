import AppKit
import Carbon
import SwiftUI

struct SettingsView: View {
    @EnvironmentObject private var model: AppModel
    @State private var shortcutError: String?
    @State private var shortcutName = ShortcutConfiguration.defaultLauncher.displayName
    @State private var shortcutMonitor: Any?
    @State private var isRecordingShortcut = false
    @State private var clearAllConfirmation = false

    private var palette: SemanticThemePalette { model.theme.palette }

    var body: some View {
        TabView {
            Form {
                Section("Appearance") {
                    Picker("Theme", selection: $model.theme) {
                        ForEach(AppTheme.allCases) { theme in
                            Text(theme.displayName).tag(theme)
                        }
                    }
                    .accessibilityIdentifier("settings.theme")
                    .accessibilityValue(model.theme.rawValue)
                    Text("Both themes are dark and apply immediately across SofDevTool.")
                        .font(.caption)
                        .foregroundStyle(palette.secondaryText.color)
                }
                Section("Utility Launcher") {
                    LabeledContent("Shortcut") {
                        Text(shortcutName).monospaced()
                        Button(isRecordingShortcut ? "Press shortcut…" : "Record Shortcut") {
                            beginRecordingShortcut()
                        }
                    }
                    if let shortcutError { Text(shortcutError).foregroundStyle(palette.error.color) }
                    Text("The shortcut is registered only while SofDevTool is running.")
                        .foregroundStyle(palette.secondaryText.color)
                }
            }
            .padding().tabItem { Label("General", systemImage: "gearshape") }

            Form {
                Toggle(
                    "Record Utility History",
                    isOn: Binding(
                        get: { model.history.isRecordingEnabled },
                        set: { model.history.isRecordingEnabled = $0 }
                    )
                )
                Text("Turning recording off preserves existing entries.")
                    .foregroundStyle(palette.secondaryText.color)
                ForEach(model.registry.definitions) { definition in
                    Toggle(
                        definition.name,
                        isOn: Binding(
                            get: {
                                model.history.isEnabled(
                                    for: definition.id, default: definition.historyEnabledByDefault)
                            },
                            set: {
                                model.history.setEnabled($0, for: definition.id)
                                model.objectWillChange.send()
                            }
                        ))
                }
                HoldToConfirmButton(title: "Hold to Clear All History", duration: 2) {
                    try? model.history.clearAll()
                }
                .accessibilityAction { clearAllConfirmation = true }
            }
            .padding().tabItem { Label("History", systemImage: "clock") }
        }
        .frame(width: 560, height: 410)
        .background(palette.windowBackground.color)
        .foregroundStyle(palette.primaryText.color)
        .tint(palette.appAccent.color)
        .overlay(alignment: .topLeading) {
            ThemeAccessibilityValue(
                surface: "Settings", theme: model.theme, accessibilityID: "settings.theme.value")
        }
        .onAppear { shortcutName = model.shortcutController?.configuration.displayName ?? shortcutName }
        .onDisappear { stopRecordingShortcut() }
        .alert("Clear all Utility History?", isPresented: $clearAllConfirmation) {
            Button("Clear All", role: .destructive) { try? model.history.clearAll() }
            Button("Cancel", role: .cancel) {}
        } message: {
            Text("This permanently deletes every Utility History file and cannot be undone.")
        }
    }

    private func beginRecordingShortcut() {
        stopRecordingShortcut()
        isRecordingShortcut = true
        shortcutError = nil
        shortcutMonitor = NSEvent.addLocalMonitorForEvents(matching: .keyDown) { event in
            let flags = event.modifierFlags.intersection(.deviceIndependentFlagsMask)
            let hasRequiredModifier =
                flags.contains(.command) || flags.contains(.control) || flags.contains(.option)
            guard hasRequiredModifier else {
                shortcutError = "Use Command, Control, or Option with another key."
                return nil
            }
            var carbonModifiers: UInt32 = 0
            var display = ""
            if flags.contains(.control) {
                carbonModifiers |= UInt32(controlKey)
                display += "⌃"
            }
            if flags.contains(.option) {
                carbonModifiers |= UInt32(optionKey)
                display += "⌥"
            }
            if flags.contains(.shift) {
                carbonModifiers |= UInt32(shiftKey)
                display += "⇧"
            }
            if flags.contains(.command) {
                carbonModifiers |= UInt32(cmdKey)
                display += "⌘"
            }
            let key =
                event.keyCode == UInt16(kVK_Space)
                ? "Space" : (event.charactersIgnoringModifiers?.uppercased() ?? "Key \(event.keyCode)")
            let configuration = ShortcutConfiguration(
                keyCode: UInt32(event.keyCode), carbonModifiers: carbonModifiers, displayName: display + key)
            shortcutError = model.shortcutController?.register(configuration)
            if shortcutError == nil { shortcutName = configuration.displayName }
            stopRecordingShortcut()
            return nil
        }
    }

    private func stopRecordingShortcut() {
        if let shortcutMonitor { NSEvent.removeMonitor(shortcutMonitor) }
        shortcutMonitor = nil
        isRecordingShortcut = false
    }
}
