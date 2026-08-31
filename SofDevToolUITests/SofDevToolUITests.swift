import XCTest

final class SofDevToolUITests: XCTestCase {
    override func setUpWithError() throws { continueAfterFailure = false }

    @MainActor
    private func makeApp() -> XCUIApplication {
        let app = XCUIApplication()
        app.launchArguments += ["-ApplePersistenceIgnoreState", "YES"]
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
        search.click()
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
}
