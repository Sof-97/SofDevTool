#!/usr/bin/env python3
"""Generate a simple deterministic app icon PNG without external libraries.

Draws a dark rounded square with a blue diagonal band and a light inset square.
Used by scripts/bundle-release; not part of the application runtime.
"""
import struct
import sys
import zlib

SIZE = 1024


def blend(base, over, alpha):
    return tuple(round(b * (1 - alpha) + o * alpha) for b, o in zip(base, over))


def pixel(x, y):
    # Background: graphite.
    color = (0x1B, 0x1C, 0x1F, 255)
    margin = 96
    radius = 180
    # Rounded-square mask.
    inside = True
    cx = min(max(x, margin + radius), SIZE - margin - radius)
    cy = min(max(y, margin + radius), SIZE - margin - radius)
    dx = x - cx
    dy = y - cy
    if (x < margin or x >= SIZE - margin or y < margin or y >= SIZE - margin):
        inside = False
    elif dx * dx + dy * dy > radius * radius and (x < margin + radius or x >= SIZE - margin - radius) and (y < margin + radius or y >= SIZE - margin - radius):
        inside = False
    if not inside:
        return (0, 0, 0, 0)
    # Diagonal blue band.
    if abs((x - y)) < 150:
        color = blend(color, (0x5B, 0x8D, 0xEF), 0.95)
    # Light inset square (the "toolbox" body).
    if 330 <= x <= 694 and 330 <= y <= 694:
        color = blend(color, (0xE6, 0xE7, 0xEA), 0.92)
        if 400 <= x <= 624 and 400 <= y <= 470:
            color = blend(color, (0x24, 0x26, 0x2B), 1.0)
    return color


def chunk(tag, data):
    return (
        struct.pack(">I", len(data))
        + tag
        + data
        + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)
    )


def main(path):
    raw = bytearray()
    for y in range(SIZE):
        raw.append(0)  # filter type 0
        for x in range(SIZE):
            raw.extend(pixel(x, y))
    header = struct.pack(">IIBBBBB", SIZE, SIZE, 8, 6, 0, 0, 0)
    png = (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", header)
        + chunk(b"IDAT", zlib.compress(bytes(raw), 9))
        + chunk(b"IEND", b"")
    )
    with open(path, "wb") as handle:
        handle.write(png)


if __name__ == "__main__":
    main(sys.argv[1] if len(sys.argv) > 1 else "icon.png")
