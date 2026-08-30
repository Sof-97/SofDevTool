#!/usr/bin/env node

import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { readFileSync, readdirSync, statSync } from "node:fs";
import { join, resolve } from "node:path";

const root = resolve(import.meta.dirname, "..");
const packageRoot = join(root, "ThirdParty/PierreDiffsSwift");
const swiftRoot = join(packageRoot, "Sources/PierreDiffsSwift");
const coordinator = readFileSync(
  join(swiftRoot, "WebView/DiffWebViewCoordinator.swift"),
  "utf8"
);
const view = readFileSync(join(swiftRoot, "WebView/PierreDiffView.swift"), "utf8");
const project = readFileSync(join(root, "SofDevTool.xcodeproj/project.pbxproj"), "utf8");
const lockPath = join(packageRoot, "scripts/package-lock.json");
const lock = JSON.parse(readFileSync(lockPath, "utf8"));
const notices = readFileSync(join(root, "SofDevTool/Resources/ThirdPartyNotices.md"), "utf8");

assert.match(coordinator, /new TextDecoder\(\)\.decode\(bytes\)/);
assert.match(coordinator, /case \.bridgeReady:\s+isReady = true\s+executePendingOperations\(\)/);
assert.match(coordinator, /case \.ready:\s+onReady\?\(\)/);
assert.doesNotMatch(view, /developerExtrasEnabled/);
assert.match(project, /XCLocalSwiftPackageReference/);
assert.doesNotMatch(project, /XCRemoteSwiftPackageReference/);

function swiftFiles(directory) {
  return readdirSync(directory).flatMap((name) => {
    const path = join(directory, name);
    return statSync(path).isDirectory() ? swiftFiles(path) : path.endsWith(".swift") ? [path] : [];
  });
}

for (const path of swiftFiles(swiftRoot)) {
  const source = readFileSync(path, "utf8");
  assert.doesNotMatch(source, /URLSession|load\s*\(\s*URLRequest|https?:\/\//);
}

const sample = {
  old: 'let café = "Straße"\nlet emoji = "👩🏽‍💻"',
  new: 'let CAFÉ = "STRASSE"\nlet emoji = "👩🏽‍🚀"',
};
const encoded = Buffer.from(JSON.stringify(sample), "utf8").toString("base64");
const binary = atob(encoded);
const bytes = Uint8Array.from(binary, (character) => character.charCodeAt(0));
const decoded = JSON.parse(new TextDecoder().decode(bytes));
assert.deepEqual(decoded, sample);
assert.doesNotMatch(JSON.stringify(decoded), /Ã|ðŸ|â€|�/);

const productionPackages = Object.entries(lock.packages).filter(
  ([path, metadata]) => path.startsWith("node_modules/") && !metadata.dev
);
assert.equal(productionPackages.length, 54);
for (const [path, metadata] of productionPackages) {
  const name = path.slice("node_modules/".length);
  assert.ok(notices.includes(`| \`${name}\` | \`${metadata.version}\` |`));
}

function sha256(path) {
  return createHash("sha256").update(readFileSync(path)).digest("hex");
}

for (const path of [
  lockPath,
  join(packageRoot, "Sources/PierreDiffsSwift/Resources/pierre-diffs-bundle.js"),
  join(packageRoot, "Sources/PierreDiffsSwift/Resources/pierre-diffs-edit-bundle.js"),
]) {
  assert.ok(notices.includes(sha256(path)));
}

console.log(
  `Verified Text Diff UTF-8 bridge, offline-only runtime, and notices for ${productionPackages.length} bundled packages.`
);
