import AppKit
import SwiftUI

@main
struct SofDevToolApp: App {
    @NSApplicationDelegateAdaptor(AppDelegate.self) private var appDelegate
    @StateObject private var model = AppModel()

    var body: some Scene {
        WindowGroup("SofDevTool", id: "workbench") {
            WorkbenchView()
                .environmentObject(model)
                .environment(\.appTheme, model.theme)
                .preferredColorScheme(.dark)
                .frame(minWidth: 940, minHeight: 620)
                .onAppear { appDelegate.configure(model: model) }
        }
        .defaultSize(width: 1180, height: 760)
        .windowResizability(.contentMinSize)
        .commands {
            CommandGroup(after: .textEditing) {
                Button("Find Utilities") {
                    NotificationCenter.default.post(name: .focusUtilitySearch, object: nil)
                }
                .keyboardShortcut("f", modifiers: .command)
            }
        }

        Settings {
            SettingsView()
                .environmentObject(model)
                .environment(\.appTheme, model.theme)
                .preferredColorScheme(.dark)
        }
    }
}

@MainActor
final class AppDelegate: NSObject, NSApplicationDelegate {
    private var launcher: LauncherWindowController?
    private var shortcut: GlobalShortcutController?

    func applicationDidFinishLaunching(_ notification: Notification) {
        NSApp.appearance = NSAppearance(named: .darkAqua)
        NSWindow.allowsAutomaticWindowTabbing = false
    }

    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool { false }

    func applicationShouldHandleReopen(_ sender: NSApplication, hasVisibleWindows flag: Bool) -> Bool {
        if !flag { showMainWindow() }
        return true
    }

    func configure(model: AppModel) {
        guard launcher == nil else { return }
        let launcher = LauncherWindowController(model: model)
        self.launcher = launcher
        let shortcut = GlobalShortcutController(defaults: model.preferenceDefaults) { launcher.toggle() }
        self.shortcut = shortcut
        model.shortcutController = shortcut
        model.showLauncher = { launcher.toggle() }
        _ = shortcut.registerSavedOrDefault()
    }

    private func showMainWindow() {
        NSApp.activate(ignoringOtherApps: true)
        NSApp.windows.first(where: { !($0 is NSPanel) })?.makeKeyAndOrderFront(nil)
    }
}
