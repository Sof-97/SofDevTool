#!/usr/bin/env python3
"""Install only the verified Release app into a chosen Applications directory."""

import argparse
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile


ROOT = Path(__file__).resolve().parents[2]
RELEASE_NAME = "SofDevTool.app"
RELEASE_ID = "com.gerardocalia.sofdevtool"


def plist_value(bundle, key):
    result = subprocess.run(
        ["/usr/libexec/PlistBuddy", "-c", f"Print :{key}", str(bundle / "Contents/Info.plist")],
        stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True,
    )
    if result.returncode:
        raise ValueError(f"bundle has no readable {key}: {bundle}")
    return result.stdout.strip()


def verify_release(bundle, check_name=True):
    if not bundle.is_dir() or bundle.is_symlink() or (check_name and bundle.name != RELEASE_NAME):
        raise ValueError(f"not a regular {RELEASE_NAME} bundle: {bundle}")
    if plist_value(bundle, "CFBundleIdentifier") != RELEASE_ID:
        raise ValueError("bundle identifier is not the intended Release product")
    if plist_value(bundle, "SofDevToolChannel") != "release":
        raise ValueError("bundle channel is not Release")
    for relative in (
        "Contents/MacOS/sofdevtool",
        "Contents/Resources/AppIcon.icns",
        "Contents/Resources/APP_NOTICES.md",
        "Contents/Resources/THIRD_PARTY_NOTICES.md",
    ):
        if not (bundle / relative).is_file():
            raise ValueError(f"Release bundle lacks {relative}")


def install_bundle(bundle, destination_dir):
    verify_release(bundle)
    if destination_dir.is_symlink():
        raise ValueError(f"refusing a symlink installation directory: {destination_dir}")
    destination_dir.mkdir(parents=True, exist_ok=True)
    destination = destination_dir / RELEASE_NAME
    if destination.is_symlink():
        raise ValueError(f"refusing to replace a symlink bundle: {destination}")
    if destination.exists():
        verify_release(destination)
    temporary = Path(tempfile.mkdtemp(prefix=".sofdevtool-install-", dir=destination_dir))
    backup = None
    try:
        shutil.rmtree(temporary)
        shutil.copytree(bundle, temporary)
        verify_release(temporary, check_name=False)
        if destination.exists():
            backup = Path(tempfile.mkdtemp(prefix=".sofdevtool-old-", dir=destination_dir))
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
    parser.add_argument("--destination", type=Path, default=Path.home() / "Applications")
    parser.add_argument("--from-bundle", type=Path, help="use an already packaged Release bundle")
    args = parser.parse_args()
    bundle = args.from_bundle
    if bundle is None:
        subprocess.run([sys.executable, str(ROOT / "scripts/rust/package.py"), "--profile", "release"], cwd=ROOT, check=True)
        bundle = ROOT / "artifacts" / RELEASE_NAME
    destination = install_bundle(bundle.resolve(), args.destination.expanduser())
    print(destination)


if __name__ == "__main__":
    main()
