import XCTest

final class SofDevToolUITests: XCTestCase {
    override func setUpWithError() throws { continueAfterFailure = false }

    @MainActor
    private func makeApp() -> XCUIApplication {
        let app = XCUIApplication()
        app.launchArguments += ["-ApplePersistenceIgnoreState", "YES", "-ui-testing"]
        app.launchEnvironment["SOFDEVTOOL_DEFAULTS_SUITE"] =
            "dev.gerardo.SofDevTool.UITests.\(UUID().uuidString)"
        return app
    }

    @MainActor
    func testLauncherEscapeAndClickAwayDismissal() throws {
        let app = makeApp()
        app.launch()
        XCTAssertTrue(app.windows["SofDevTool"].waitForExistence(timeout: 5))

        let launcherButton = app.buttons.matching(identifier: "toolbar.launcher").firstMatch
        let launcherSearch = app.textFields["launcher.search"]

        launcherButton.click()
        XCTAssertTrue(launcherSearch.waitForExistence(timeout: 2))
        app.typeKey(.escape, modifierFlags: [])
        XCTAssertTrue(launcherSearch.waitForNonExistence(timeout: 2))

        launcherButton.click()
        XCTAssertTrue(launcherSearch.waitForExistence(timeout: 2))
        app.windows["SofDevTool"].coordinate(withNormalizedOffset: CGVector(dx: 0.08, dy: 0.4))
            .click()
        XCTAssertTrue(launcherSearch.waitForNonExistence(timeout: 2))
    }

    @MainActor
    func testCatalogSearchAndWorkspaceSwitching() throws {
        let app = makeApp()
        app.launch()
        XCTAssertTrue(app.windows["SofDevTool"].waitForExistence(timeout: 5))

        let search = app.textFields["catalog.search"]
        XCTAssertTrue(search.waitForExistence(timeout: 2))
        app.typeKey("f", modifierFlags: .command)
        search.typeText("base64")
        let base64 = app.staticTexts["catalog.utility.base64"]
        XCTAssertTrue(base64.waitForExistence(timeout: 2))
        base64.click()
        XCTAssertTrue(app.descendants(matching: .any)["base64.input"].waitForExistence(timeout: 2))

        app.buttons.matching(identifier: "toolbar.launcher").firstMatch.click()
        XCTAssertTrue(app.textFields["launcher.search"].waitForExistence(timeout: 2))
        let identifierGenerator = app.staticTexts["Identifier Generator"]
        XCTAssertTrue(identifierGenerator.waitForExistence(timeout: 2))
        identifierGenerator.click()
        XCTAssertTrue(app.buttons["identifier.generate"].waitForExistence(timeout: 2))
        XCTAssertTrue(app.textFields["launcher.search"].waitForNonExistence(timeout: 2))

        app.buttons.matching(identifier: "toolbar.settings").firstMatch.click()
        XCTAssertTrue(app.buttons["Record Shortcut"].waitForExistence(timeout: 2))
    }

    @MainActor
    func testColorTextConversionSynchronizesTheNativePicker() throws {
        let app = makeApp()
        app.launch()
        XCTAssertTrue(app.windows["SofDevTool"].waitForExistence(timeout: 5))

        let search = app.textFields["catalog.search"]
        XCTAssertTrue(search.waitForExistence(timeout: 2))
        search.click()
        search.typeText("color conversion")
        let colorConversion = app.staticTexts["catalog.utility.color-conversion"]
        XCTAssertTrue(colorConversion.waitForExistence(timeout: 2))
        colorConversion.click()

        let picker = app.descendants(matching: .any)["color-conversion.picker"]
        let input = app.textFields["color-conversion.input"]
        XCTAssertTrue(picker.waitForExistence(timeout: 2))
        XCTAssertTrue(input.waitForExistence(timeout: 2))
        let initialPickerValue = String(describing: picker.value)

        input.click()
        input.typeText("#ff000080")
        app.buttons["Convert"].click()

        XCTAssertTrue(app.staticTexts["#ff000080"].waitForExistence(timeout: 2))
        XCTAssertNotEqual(String(describing: picker.value), initialPickerValue)
    }

    @MainActor
    func testThemePropagatesPersistsAndPreservesUtilitySession() throws {
        let app = makeApp()
        app.launch()
        XCTAssertTrue(app.windows["SofDevTool"].waitForExistence(timeout: 5))

        openSettings(in: app)
        selectTheme("Graphite", in: app)
        app.typeKey("w", modifierFlags: .command)

        let search = app.textFields["catalog.search"]
        search.click()
        search.typeText("base64")
        let base64 = app.staticTexts["catalog.utility.base64"]
        XCTAssertTrue(base64.waitForExistence(timeout: 2))
        base64.click()

        let input = app.descendants(matching: .any)["base64.input"]
        let result = app.descendants(matching: .any)["base64.result"]
        let mode = app.descendants(matching: .any)["base64.mode"]
        let alphabet = app.descendants(matching: .any)["base64.alphabet"]
        let padding = app.descendants(matching: .any)["base64.padding"]
        XCTAssertTrue(input.waitForExistence(timeout: 2))
        XCTAssertTrue(result.waitForExistence(timeout: 2))
        input.click()
        input.typeText("session survives theme")
        XCTAssertTrue(
            result.waitForValue(containing: "c2Vzc2lvbiBzdXJ2aXZlcyB0aGVtZQ==", timeout: 3))
        let modeValue = String(describing: mode.value)
        let alphabetValue = String(describing: alphabet.value)
        let paddingValue = String(describing: padding.value)

        app.buttons.matching(identifier: "toolbar.history").firstMatch.click()
        XCTAssertTrue(app.descendants(matching: .any)["history.inspector"].waitForNonExistence(timeout: 2))

        openSettings(in: app)
        selectTheme("Catppuccin Frappé", in: app)
        XCTAssertEqual(
            app.descendants(matching: .any)["settings.theme"].value as? String,
            "Catppuccin Frappé")
        XCTAssertEqual(
            app.descendants(matching: .any)["settings.theme.value"].value as? String, "frappe")
        XCTAssertEqual(app.descendants(matching: .any)["workbench.theme"].value as? String, "frappe")
        app.typeKey("w", modifierFlags: .command)

        XCTAssertTrue(String(describing: input.value).contains("session survives theme"))
        XCTAssertTrue(String(describing: result.value).contains("c2Vzc2lvbiBzdXJ2aXZlcyB0aGVtZQ=="))
        XCTAssertEqual(String(describing: mode.value), modeValue)
        XCTAssertEqual(String(describing: alphabet.value), alphabetValue)
        XCTAssertEqual(String(describing: padding.value), paddingValue)
        XCTAssertFalse(app.descendants(matching: .any)["history.inspector"].exists)

        app.buttons.matching(identifier: "toolbar.launcher").firstMatch.click()
        let launcherTheme = app.descendants(matching: .any)["launcher.theme"]
        XCTAssertTrue(launcherTheme.waitForExistence(timeout: 2))
        XCTAssertEqual(launcherTheme.value as? String, "frappe")
        app.typeKey(.escape, modifierFlags: [])

        app.terminate()
        app.launch()
        XCTAssertTrue(app.windows["SofDevTool"].waitForExistence(timeout: 5))
        XCTAssertEqual(app.descendants(matching: .any)["workbench.theme"].value as? String, "frappe")
    }

    @MainActor
    private func openSettings(in app: XCUIApplication) {
        app.buttons.matching(identifier: "toolbar.settings").firstMatch.click()
        XCTAssertTrue(app.descendants(matching: .any)["settings.theme"].waitForExistence(timeout: 2))
    }

    @MainActor
    private func selectTheme(_ name: String, in app: XCUIApplication) {
        let picker = app.descendants(matching: .any)["settings.theme"]
        picker.click()
        let choice = app.menuItems[name]
        XCTAssertTrue(choice.waitForExistence(timeout: 2))
        choice.click()
    }
}

private extension XCUIElement {
    func waitForValue(containing expected: String, timeout: TimeInterval) -> Bool {
        let predicate = NSPredicate(format: "value CONTAINS %@", expected)
        return XCTWaiter.wait(
            for: [XCTNSPredicateExpectation(predicate: predicate, object: self)], timeout: timeout)
            == .completed
    }
}
