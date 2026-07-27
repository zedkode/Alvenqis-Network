#!/usr/bin/env python3
"""Convert logo solid dark plate / vignette to true PNG transparency (PIL only).

Aggressive near-black + dark-navy plate removal so the neon mark floats cleanly
on glass UI surfaces without a black square/circle behind it.
"""
from __future__ import annotations

from collections import deque
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
SRC = ROOT / "logo.png"
BAK = ROOT / "logo.original-opaque.png"
OUT = ROOT / "logo.png"
PUBLIC = ROOT / "public" / "logo.png"
ICON_SRC = ROOT / "src-tauri" / "icons" / "logo-source.png"


def luma(r: int, g: int, b: int) -> float:
    return 0.2126 * r + 0.7152 * g + 0.0722 * b


def is_bg(r: int, g: int, b: int) -> bool:
    """Near-black / dark navy plate (not neon content)."""
    mx = r if r >= g else g
    if b > mx:
        mx = b
    mn = r if r <= g else g
    if b < mn:
        mn = b
    chroma = mx - mn
    y = luma(r, g, b)

    # Pure plate: very dark, low chroma.
    if mx <= 16 and chroma <= 14:
        return True
    # Dark navy vignette (common in this brand plate).
    if mx <= 36 and chroma <= 22 and y <= 22:
        return True
    # Slightly lifted navy with blue cast still counts as plate.
    if mx <= 48 and chroma <= 28 and y <= 28 and r <= 28 and g <= 24:
        return True
    # Deep purple haze on plate (low luma, blue-dominant, not neon).
    if y <= 18 and mx <= 55 and b >= r and b >= g and chroma <= 40:
        return True
    return False


def is_soft_fringe(r: int, g: int, b: int) -> bool:
    """Dark halo that should fade, not stay as opaque black plate."""
    mx = max(r, g, b)
    mn = min(r, g, b)
    chroma = mx - mn
    y = luma(r, g, b)
    if y <= 10 and chroma <= 45:
        return True
    if y <= 22 and chroma <= 55 and mx <= 70:
        return True
    if y <= 32 and chroma <= 35 and mx <= 50:
        return True
    return False


def main() -> None:
    if BAK.exists():
        path = BAK
    elif SRC.exists():
        SRC.replace(BAK)
        path = BAK
    else:
        raise SystemExit(f"Missing logo at {SRC}")

    img = Image.open(path).convert("RGBA")
    w, h = img.size
    px = img.load()

    # 1) Flood-fill background from edges (connected dark plate).
    bg = [[False] * w for _ in range(h)]
    q: deque[tuple[int, int]] = deque()
    seeds: list[tuple[int, int]] = [
        (0, 0),
        (w - 1, 0),
        (0, h - 1),
        (w - 1, h - 1),
        (w // 2, 0),
        (0, h // 2),
        (w - 1, h // 2),
        (w // 2, h - 1),
    ]
    # Extra edge seeds every ~5% for solid plate coverage.
    for t in range(0, 21):
        x = min(w - 1, max(0, int(w * t / 20)))
        y = min(h - 1, max(0, int(h * t / 20)))
        seeds.extend([(x, 0), (x, h - 1), (0, y), (w - 1, y)])

    for x, y in seeds:
        r, g, b, _ = px[x, y]
        if is_bg(r, g, b) and not bg[y][x]:
            bg[y][x] = True
            q.append((x, y))

    while q:
        x, y = q.popleft()
        for nx, ny in ((x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)):
            if nx < 0 or ny < 0 or nx >= w or ny >= h or bg[ny][nx]:
                continue
            r, g, b, _ = px[nx, ny]
            if is_bg(r, g, b):
                bg[ny][nx] = True
                q.append((nx, ny))

    # 2) Morphological grow: pull dark fringe into the bg mask (2 passes).
    for _ in range(2):
        grow: list[tuple[int, int]] = []
        for y in range(h):
            for x in range(w):
                if bg[y][x]:
                    continue
                r, g, b, _ = px[x, y]
                if not is_soft_fringe(r, g, b) and not is_bg(r, g, b):
                    continue
                # Only grow if a 4-neighbor is already bg (keeps neon interior).
                for nx, ny in ((x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)):
                    if 0 <= nx < w and 0 <= ny < h and bg[ny][nx]:
                        grow.append((x, y))
                        break
        for x, y in grow:
            bg[y][x] = True

    transparent = 0
    opaque = 0
    min_x, min_y = w, h
    max_x, max_y = 0, 0

    for y in range(h):
        for x in range(w):
            r, g, b, _ = px[x, y]
            mx = max(r, g, b)
            mn = min(r, g, b)
            chroma = mx - mn
            yv = luma(r, g, b)

            if bg[y][x]:
                alpha = 0
            elif is_soft_fringe(r, g, b):
                # Soft residual halo: dark → more transparent.
                t = max(0.0, min(1.0, (yv - 6.0) / 28.0))
                alpha = max(0, min(255, int(t * 200)))
                if chroma < 20 and yv < 18:
                    alpha = min(alpha, int(yv * 6))
            else:
                # Very dark low-chroma leftovers still attached to plate edge.
                if mx < 20 and chroma < 22:
                    alpha = max(0, min(255, int(mx * 7)))
                elif mx < 40 and chroma < 28 and yv < 28:
                    t = (mx - 10) / 30.0
                    alpha = max(16, min(255, int(t * 230)))
                else:
                    alpha = 255

            px[x, y] = (r, g, b, alpha)
            if alpha < 16:
                transparent += 1
            elif alpha > 240:
                opaque += 1
            if alpha > 8:
                if x < min_x:
                    min_x = x
                if y < min_y:
                    min_y = y
                if x > max_x:
                    max_x = x
                if y > max_y:
                    max_y = y

    total = w * h
    bg_count = sum(1 for y in range(h) for x in range(w) if bg[y][x])
    print(f"bg_flood_px={bg_count} transparent_px={transparent} opaque_px={opaque} total={total}")
    if transparent < total * 0.25:
        raise SystemExit(
            f"Transparency conversion failed: too few transparent pixels ({transparent}/{total})"
        )

    if max_x >= min_x and max_y >= min_y:
        pad_x = max(8, int((max_x - min_x) * 0.03))
        pad_y = max(8, int((max_y - min_y) * 0.03))
        box = (
            max(0, min_x - pad_x),
            max(0, min_y - pad_y),
            min(w, max_x + pad_x + 1),
            min(h, max_y + pad_y + 1),
        )
        img = img.crop(box)

    # Tauri icon tooling requires a square source — pad with transparent pixels.
    cw, ch = img.size
    side = max(cw, ch)
    if cw != ch:
        canvas = Image.new("RGBA", (side, side), (0, 0, 0, 0))
        canvas.paste(img, ((side - cw) // 2, (side - ch) // 2), img)
        img = canvas

    PUBLIC.parent.mkdir(parents=True, exist_ok=True)
    img.save(OUT, "PNG", optimize=True)
    img.save(PUBLIC, "PNG", optimize=True)

    # UI mark (transparent)
    mark = img.copy()
    mark.thumbnail((256, 256), Image.Resampling.LANCZOS)
    mark_path = PUBLIC.parent / "logo-mark.png"
    mark.save(mark_path, "PNG", optimize=True)

    # Favicon / app icon candidate at 512
    icon512 = img.copy()
    icon512.thumbnail((512, 512), Image.Resampling.LANCZOS)
    # Keep square 512 canvas
    if icon512.size != (512, 512):
        canvas = Image.new("RGBA", (512, 512), (0, 0, 0, 0))
        ox = (512 - icon512.size[0]) // 2
        oy = (512 - icon512.size[1]) // 2
        canvas.paste(icon512, (ox, oy), icon512)
        icon512 = canvas
    icon_path = PUBLIC.parent / "icon.png"
    icon512.save(icon_path, "PNG", optimize=True)

    # Source for tauri icon bake + logo-source
    ICON_SRC.parent.mkdir(parents=True, exist_ok=True)
    icon_src = img.copy()
    # Prefer ~1024 square for icon tooling
    icon_src.thumbnail((1024, 1024), Image.Resampling.LANCZOS)
    if icon_src.size[0] != icon_src.size[1]:
        s = max(icon_src.size)
        canvas = Image.new("RGBA", (s, s), (0, 0, 0, 0))
        canvas.paste(
            icon_src,
            ((s - icon_src.size[0]) // 2, (s - icon_src.size[1]) // 2),
            icon_src,
        )
        icon_src = canvas
    icon_src.save(ICON_SRC, "PNG", optimize=True)
    # Also overwrite icons/icon.png so shell icons pick up transparency without full bake.
    icon512.save(ICON_SRC.parent / "icon.png", "PNG", optimize=True)

    print(f"saved {OUT} size={img.size} mode={img.mode} file_bytes={OUT.stat().st_size}")
    print(f"saved {mark_path} {mark.size}")
    print(f"saved {icon_path} {icon512.size}")
    print(f"saved {ICON_SRC} {icon_src.size}")


if __name__ == "__main__":
    main()
