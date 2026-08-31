#!/usr/bin/env node

import { createHash } from "node:crypto";
import { existsSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { basename, join, resolve } from "node:path";

const repositoryRoot = resolve(import.meta.dirname, "..");
const packageRoot = join(repositoryRoot, "ThirdParty/PierreDiffsSwift");
const yamsLicensePath = join(repositoryRoot, "ThirdParty/Yams-LICENSE");
const scriptsRoot = join(packageRoot, "scripts");
const nodeModulesRoot = resolve(process.argv[2] ?? join(scriptsRoot, "node_modules"));
const lockPath = join(scriptsRoot, "package-lock.json");

if (!existsSync(nodeModulesRoot)) {
  throw new Error(`node_modules not found at ${nodeModulesRoot}`);
}

const lock = JSON.parse(readFileSync(lockPath, "utf8"));
const packages = Object.entries(lock.packages)
  .filter(([path, metadata]) => path.startsWith("node_modules/") && !metadata.dev)
  .map(([path, metadata]) => ({
    name: path.slice("node_modules/".length),
    version: metadata.version,
    license: metadata.license,
  }))
  .sort((left, right) => left.name.localeCompare(right.name));

function sha256(path) {
  return createHash("sha256").update(readFileSync(path)).digest("hex");
}

function licenseText(packageName) {
  const directory = join(nodeModulesRoot, packageName);
  const licenseFile = readdirSync(directory).find((name) => /^licen[sc]e($|\.)/i.test(name));
  if (licenseFile) return readFileSync(join(directory, licenseFile), "utf8").trim();

  if (packageName === "lru_map") {
    const readme = readFileSync(join(directory, "README.md"), "utf8");
    const marker = readme.indexOf("# MIT license");
    if (marker >= 0) return readme.slice(marker + "# MIT license".length).trim();
  }

  throw new Error(`No license text found for ${packageName}`);
}

const noticesByText = new Map();
for (const packageMetadata of packages) {
  const text = licenseText(packageMetadata.name);
  const entries = noticesByText.get(text) ?? [];
  entries.push(packageMetadata);
  noticesByText.set(text, entries);
}

const output = [];
output.push("# Third-party notices", "");
output.push(
  "SofDevTool includes an immutable maintained snapshot of PierreDiffsSwift 1.2.4 at upstream revision `c2249d7890de957a96480711152d90a06fa1222b`.",
  "The snapshot bundles the JavaScript runtime described by its exact npm lockfile. This inventory intentionally includes every production dependency in that lockfile.",
  ""
);
output.push(`- Lockfile SHA-256: \`${sha256(lockPath)}\``);
output.push(
  `- Main JavaScript bundle SHA-256: \`${sha256(join(packageRoot, "Sources/PierreDiffsSwift/Resources/pierre-diffs-bundle.js"))}\``
);
output.push(
  `- Optional edit bundle SHA-256: \`${sha256(join(packageRoot, "Sources/PierreDiffsSwift/Resources/pierre-diffs-edit-bundle.js"))}\``
);
output.push(
  "- Regenerate after an audited dependency update with `node scripts/generate-third-party-notices.mjs /path/to/exact/node_modules`.",
  ""
);

output.push("## Swift wrapper", "", "### PierreDiffsSwift 1.2.4 - MIT", "");
output.push(readFileSync(join(packageRoot, "LICENSE"), "utf8").trim(), "");

output.push("## YAML parser", "", "### Yams 6.2.2 and bundled libYAML - MIT", "");
output.push(
  "Yams is resolved exactly at version `6.2.2`, revision `a27b21e0c81c5bf42049b897a62aaf387e80f279`.",
  "The package includes its CYaml/libYAML implementation and has no transitive Swift package dependencies.",
  "",
  readFileSync(yamsLicensePath, "utf8").trim(),
  ""
);

output.push("## Bundled JavaScript inventory", "", "| Package | Version | Declared license |", "| --- | --- | --- |");
for (const packageMetadata of packages) {
  output.push(
    `| \`${packageMetadata.name}\` | \`${packageMetadata.version}\` | ${packageMetadata.license} |`
  );
}
output.push("");

output.push("## Bundled JavaScript license texts", "");
for (const [text, entries] of noticesByText) {
  const names = entries.map(({ name, version }) => `${name} ${version}`).join(", ");
  output.push(`### ${names}`, "", "```text", text, "```", "");
}

const outputPath = join(repositoryRoot, "SofDevTool/Resources/ThirdPartyNotices.md");
writeFileSync(outputPath, `${output.join("\n")}\n`);
console.log(`Wrote ${basename(outputPath)} for ${packages.length} production packages.`);
