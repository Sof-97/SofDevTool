#!/usr/bin/env python3
"""Build one identified macOS app bundle from the root Cargo workspace."""

import argparse
import json
import os
from pathlib import Path
import re
import shutil
import struct
import subprocess
import sys
import tempfile
from xml.sax.saxutils import escape


ROOT = Path(__file__).resolve().parents[2]
ASSETS = ROOT / "crates/app/src/text_diff/assets"
MACOS = ROOT / "platform/macos"
PROFILES = {
    "debug": ("SofDevTool Debug", "com.gerardocalia.sofdevtool.debug", "debug.iconset"),
    "release": ("SofDevTool", "com.gerardocalia.sofdevtool", "release.iconset"),
}

ICON_TYPES = (
    ("icp4", "icon_16x16.png", 16),
    ("icp5", "icon_32x32.png", 32),
    ("icp6", "icon_32x32@2x.png", 64),
    ("ic07", "icon_128x128.png", 128),
    ("ic08", "icon_256x256.png", 256),
    ("ic09", "icon_512x512.png", 512),
    ("ic10", "icon_512x512@2x.png", 1024),
    ("ic11", "icon_16x16@2x.png", 32),
    ("ic12", "icon_32x32@2x.png", 64),
    ("ic13", "icon_128x128@2x.png", 256),
    ("ic14", "icon_256x256@2x.png", 512),
)


def write_icns(iconset, destination):
    """Wrap the unchanged approved PNGs in standard modern ICNS PNG chunks."""
    chunks = []
    for kind, filename, size in ICON_TYPES:
        image = (iconset / filename).read_bytes()
        if image[:16] != b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR":
            raise ValueError(f"invalid PNG icon resource: {filename}")
        width, height = struct.unpack(">II", image[16:24])
        if (width, height) != (size, size):
            raise ValueError(f"wrong icon dimensions in {filename}: {width}x{height}")
        chunks.append(kind.encode("ascii") + struct.pack(">I", len(image) + 8) + image)
    payload = b"".join(chunks)
    destination.write_bytes(b"icns" + struct.pack(">I", len(payload) + 8) + payload)


def package_version():
    metadata = subprocess.run(
        ["cargo", "metadata", "--no-deps", "--format-version=1"],
        cwd=ROOT, check=True, stdout=subprocess.PIPE, text=True,
    )
    packages = json.loads(metadata.stdout)["packages"]
    return next(package["version"] for package in packages if package["name"] == "sofdevtool-app")


def source_metadata():
    output = subprocess.run(
        [sys.executable, str(ROOT / "scripts/rust/source-metadata.py")],
        cwd=ROOT, check=True, stdout=subprocess.PIPE, text=True,
    ).stdout
    values = dict(line.split("=", 1) for line in output.splitlines())
    if values["state"] not in {"clean", "dirty", "unknown"}:
        raise ValueError("invalid source state")
    if not re.fullmatch(r"[0-9a-f]{40}", values["revision"]) or values["state"] == "unknown":
        raise RuntimeError("cannot package without an exact Git revision and known source state")
    return values["revision"], values["state"]


def build_binary(profile, revision, state, build_id):
    command = ["cargo", "build", "-p", "sofdevtool-app", "--bin", "sofdevtool", "--message-format=json"]
    if profile == "release":
        command.insert(2, "--release")
    environment = os.environ.copy()
    environment.update({
        "SOFDEVTOOL_REVISION": revision,
        "SOFDEVTOOL_SOURCE_STATE": state,
        "SOFDEVTOOL_BUILD_ID": build_id,
    })
    result = subprocess.run(command, cwd=ROOT, env=environment, check=True, stdout=subprocess.PIPE, text=True)
    executables = []
    for line in result.stdout.splitlines():
        try:
            message = json.loads(line)
        except ValueError:
            continue
        target = message.get("target", {})
        if message.get("reason") == "compiler-artifact" and target.get("name") == "sofdevtool" and "bin" in target.get("kind", []):
            if message.get("executable"):
                executables.append(Path(message["executable"]))
    if not executables or not executables[-1].is_file():
        raise RuntimeError("Cargo did not report the sofdevtool executable")
    return executables[-1]


def plist(profile, version, build_id, revision, state):
    name, identifier, _ = PROFILES[profile]
    values = {
        "CFBundleDevelopmentRegion": "en",
        "CFBundleExecutable": "sofdevtool",
        "CFBundleIdentifier": identifier,
        "CFBundleInfoDictionaryVersion": "6.0",
        "CFBundleName": name,
        "CFBundleDisplayName": name,
        "CFBundleIconFile": "AppIcon",
        "CFBundlePackageType": "APPL",
        "CFBundleShortVersionString": version,
        "CFBundleVersion": build_id,
        "LSMinimumSystemVersion": "14.0",
        "SofDevToolBuildID": build_id,
        "SofDevToolChannel": profile,
        "SofDevToolRevision": revision,
        "SofDevToolSourceState": state,
    }
    body = "\n".join(f"  <key>{escape(key)}</key><string>{escape(value)}</string>" for key, value in values.items())
    return (
        '<?xml version="1.0" encoding="UTF-8"?>\n'
        '<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">\n'
        f'<plist version="1.0"><dict>\n{body}\n  <key>NSHighResolutionCapable</key><true/>\n</dict></plist>\n'
    )


def assemble(profile, binary, output_root, version, build_id, revision, state):
    name, _identifier, iconset = PROFILES[profile]
    output_root.mkdir(parents=True, exist_ok=True)
    destination = output_root / f"{name}.app"
    if destination.is_symlink():
        raise RuntimeError(f"refusing to replace a symlink bundle: {destination}")
    temporary = Path(tempfile.mkdtemp(prefix=".sofdevtool-package-", dir=output_root))
    backup = None
    try:
        contents = temporary / "Contents"
        (contents / "MacOS").mkdir(parents=True)
        (contents / "Resources").mkdir()
        shutil.copy2(binary, contents / "MacOS/sofdevtool")
        shutil.copy2(ASSETS / "THIRD_PARTY_NOTICES.md", contents / "Resources/THIRD_PARTY_NOTICES.md")
        shutil.copy2(MACOS / "APP_NOTICES.md", contents / "Resources/APP_NOTICES.md")
        shutil.copy2(ROOT / "crates/app/src/text_diff/assets-source/RETAINED_SOURCE_LICENSE.md", contents / "Resources/RETAINED_SOURCE_LICENSE.md")
        write_icns(MACOS / "icons" / iconset, contents / "Resources/AppIcon.icns")
        (contents / "Info.plist").write_text(plist(profile, version, build_id, revision, state), encoding="utf-8")
        subprocess.run(["/usr/libexec/PlistBuddy", "-c", "Print :CFBundleIdentifier", str(contents / "Info.plist")], check=True, stdout=subprocess.DEVNULL)
        if destination.exists():
            backup = Path(tempfile.mkdtemp(prefix=".sofdevtool-old-", dir=output_root))
            backup.rmdir()
            destination.rename(backup)
        try:
            temporary.rename(destination)
        except OSError:
            if backup is not None:
                backup.rename(destination)
                backup = None
            raise
        if backup is not None:
            shutil.rmtree(backup)
        return destination
    finally:
        if temporary.exists():
            shutil.rmtree(temporary)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--profile", choices=PROFILES, required=True)
    parser.add_argument("--output-root", type=Path, default=ROOT / "artifacts")
    args = parser.parse_args()
    build_id = os.environ.get("SOFDEVTOOL_BUILD_ID", "1")
    if not re.fullmatch(r"[1-9][0-9]*", build_id):
        parser.error("SOFDEVTOOL_BUILD_ID must be a positive decimal integer")
    version = package_version()
    revision, state = source_metadata()
    binary = build_binary(args.profile, revision, state, build_id)
    if source_metadata() != (revision, state):
        raise RuntimeError("source revision or cleanliness changed during packaging")
    destination = assemble(args.profile, binary, args.output_root.resolve(), version, build_id, revision, state)
    print(destination)
    print(f"{version} build {build_id}, {args.profile}, revision {revision}, source {state}")


if __name__ == "__main__":
    main()
