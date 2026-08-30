import assert from "node:assert/strict";
import fs from "node:fs";

const coordinatorPath = new URL(
  "../Vendor/PierreDiffsSwift/Sources/PierreDiffsSwift/WebView/DiffWebViewCoordinator.swift",
  import.meta.url,
);
const coordinator = fs.readFileSync(coordinatorPath, "utf8");
const usesUTF8Decoder = coordinator.includes("new TextDecoder().decode(bytes)");

const expected = {
  old: 'let café = "Straße"\nlet emoji = "👩🏽‍💻"',
  new: 'let CAFÉ = "STRASSE"\nlet emoji = "👩🏽‍🚀"',
};
const base64 = Buffer.from(JSON.stringify(expected), "utf8").toString("base64");
const binary = atob(base64);
const decoded = usesUTF8Decoder
  ? new TextDecoder().decode(Uint8Array.from(binary, (character) => character.charCodeAt(0)))
  : binary;
const actual = JSON.parse(decoded);

assert.deepEqual(
  actual,
  expected,
  "PierreDiffsSwift must decode atob bytes as UTF-8 before JSON.parse",
);
console.log("PASS: PierreDiffsSwift preserves accented text and emoji across its JavaScript bridge");
