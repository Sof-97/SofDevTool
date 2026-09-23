#!/usr/bin/env python3
"""Compare actual worktree bytes with HEAD; never infer cleanliness from the index.

GitButler may construct a synthetic index while the working files remain intact.
This helper uses HEAD's blob table and the filesystem, then filters untracked
paths against that table. It reports unknown if Git cannot establish either.
"""

import hashlib
import os
from pathlib import Path
import stat
import subprocess


ROOT = Path(__file__).resolve().parents[2]


def git(*args):
    return subprocess.run(
        ["git", *args], cwd=ROOT, check=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE
    ).stdout


def blob_id(path):
    data = os.readlink(path).encode() if path.is_symlink() else path.read_bytes()
    return hashlib.sha1(b"blob " + str(len(data)).encode() + b"\0" + data).hexdigest()


def mode_matches(path, mode):
    if mode == "120000":
        return path.is_symlink()
    if path.is_symlink() or not path.is_file():
        return False
    if mode not in {"100644", "100755"}:
        return False
    executable = bool(path.stat().st_mode & stat.S_IXUSR)
    return executable == (mode == "100755")


def source_metadata():
    try:
        revision = git("rev-parse", "HEAD").decode().strip()
        tree = git("ls-tree", "-r", "-z", "HEAD").split(b"\0")
        tracked = {}
        for record in filter(None, tree):
            header, raw_path = record.split(b"\t", 1)
            mode, kind, object_id = header.decode().split()
            if kind == "blob":
                tracked[os.fsdecode(raw_path)] = (mode, object_id)
        for name, (mode, expected) in tracked.items():
            path = ROOT / name
            if not path.exists() and not path.is_symlink():
                return revision, "dirty"
            if not mode_matches(path, mode):
                return revision, "dirty"
            if blob_id(path) != expected:
                return revision, "dirty"
        others = git("ls-files", "--others", "--exclude-standard", "-z").split(b"\0")
        if any(os.fsdecode(name) not in tracked for name in others if name):
            return revision, "dirty"
        return revision, "clean"
    except (OSError, ValueError, subprocess.CalledProcessError):
        return "unknown", "unknown"


if __name__ == "__main__":
    revision, state = source_metadata()
    print(f"revision={revision}")
    print(f"state={state}")
