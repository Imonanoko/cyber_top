#!/usr/bin/env python3
"""
Generate map-item sprite icons as PNG files.
Uses only Python built-in modules (struct, zlib, math, os) — no pip required.
Run once: python3 gen_assets.py
Output: assets/obstacles/{obstacle,gravity_device,speed_boost,damage_boost}.png
"""
import os, struct, zlib, math

SIZE = 64  # 64×64 RGBA

# ── PNG writer ────────────────────────────────────────────────────────

def make_png(pixels_rgba):
    def chunk(ctype, data):
        c = ctype + data
        return struct.pack('>I', len(data)) + c + struct.pack('>I', zlib.crc32(c) & 0xFFFFFFFF)

    raw = bytearray()
    for y in range(SIZE):
        raw.append(0)  # filter = None per row
        raw += pixels_rgba[y * SIZE * 4: (y + 1) * SIZE * 4]

    return (
        b'\x89PNG\r\n\x1a\n'
        + chunk(b'IHDR', struct.pack('>IIBBBBB', SIZE, SIZE, 8, 6, 0, 0, 0))
        + chunk(b'IDAT', zlib.compress(bytes(raw), 9))
        + chunk(b'IEND', b'')
    )

def render(fn):
    buf = bytearray(SIZE * SIZE * 4)
    for y in range(SIZE):
        for x in range(SIZE):
            r, g, b, a = fn(x, y)
            i = (y * SIZE + x) * 4
            buf[i], buf[i+1], buf[i+2], buf[i+3] = (
                max(0, min(255, r)), max(0, min(255, g)),
                max(0, min(255, b)), max(0, min(255, a)),
            )
    return buf

def seg_dist(px, py, ax, ay, bx, by):
    dx, dy = bx - ax, by - ay
    l2 = dx*dx + dy*dy
    if l2 == 0: return math.hypot(px - ax, py - ay)
    t = max(0.0, min(1.0, ((px - ax)*dx + (py - ay)*dy) / l2))
    return math.hypot(px - (ax + t*dx), py - (ay + t*dy))

# ── Icons ─────────────────────────────────────────────────────────────

def obstacle(x, y):
    nx, ny = x / SIZE, y / SIZE
    b = 0.06
    if nx < b or nx > 1-b or ny < b or ny > 1-b:
        return (90, 90, 100, 255)
    # X pattern: two diagonals
    t = 0.11
    d1 = abs(ny - nx) / math.sqrt(2)
    d2 = abs(ny - (1 - nx)) / math.sqrt(2)
    if d1 < t or d2 < t:
        return (210, 210, 220, 255)
    return (55, 55, 65, 255)


def gravity_device(x, y):
    cx = cy = (SIZE - 1) / 2
    d = math.hypot(x - cx, y - cy) / (SIZE / 2)   # 0..1
    if d > 0.97:
        return (0, 0, 0, 0)
    # Concentric rings
    ring = (math.sin(d * math.pi * 6) + 1) / 2
    r = int(90 + ring * 50)
    g = int(20 + ring * 15)
    b = int(170 + ring * 60)
    a = int(220 * (1 - d * 0.35))
    # Bright core
    if d < 0.18:
        bright = (0.18 - d) / 0.18
        r += int(bright * 80); g += int(bright * 30); b += int(bright * 50)
    # Outer glow ring
    if 0.82 < d < 0.97:
        glow = 1 - abs(d - 0.90) / 0.08
        r += int(glow * 40); g += int(glow * 20); b += int(glow * 60)
    return (r, g, b, a)


def speed_boost(x, y):
    nx, ny = x / SIZE, y / SIZE
    b = 0.05
    if nx < b or nx > 1-b or ny < b or ny > 1-b:
        return (30, 130, 40, 255)
    # Lightning bolt segments (nx, ny coords 0..1)
    # Main shaft: upper-right to middle-left to lower-right
    segs = [
        (0.62, 0.08, 0.35, 0.48),
        (0.35, 0.48, 0.65, 0.48),
        (0.65, 0.48, 0.38, 0.92),
    ]
    thick = 0.10
    if any(seg_dist(nx, ny, *s) < thick for s in segs):
        return (200, 255, 80, 255)
    # Glow around bolt
    thick_glow = 0.17
    if any(seg_dist(nx, ny, *s) < thick_glow for s in segs):
        return (80, 180, 50, 200)
    return (20, 70, 25, 255)


def damage_boost(x, y):
    nx, ny = x / SIZE, y / SIZE
    b = 0.05
    if nx < b or nx > 1-b or ny < b or ny > 1-b:
        return (150, 30, 30, 255)
    # Sword: vertical blade + guard + handle
    blade  = seg_dist(nx, ny, 0.50, 0.07, 0.50, 0.70) < 0.07
    guard  = seg_dist(nx, ny, 0.22, 0.64, 0.78, 0.64) < 0.06
    handle = seg_dist(nx, ny, 0.50, 0.72, 0.50, 0.93) < 0.07
    if blade or guard or handle:
        # Slight gradient: brighter in center
        center_dist = abs(nx - 0.5)
        bright = int(230 - center_dist * 80)
        return (bright, bright, bright + 10, 255)
    # Tip glow
    if ny < 0.12 and abs(nx - 0.5) < 0.10:
        tip = (0.12 - ny) / 0.12
        return (255, int(80 + tip * 100), int(tip * 80), 255)
    return (75, 15, 15, 255)


# ── Write files ───────────────────────────────────────────────────────

icons = {
    'obstacle': obstacle,
    'gravity_device': gravity_device,
    'speed_boost': speed_boost,
    'damage_boost': damage_boost,
}

out_dir = os.path.join(os.path.dirname(os.path.abspath(__file__)), 'assets', 'obstacles')
os.makedirs(out_dir, exist_ok=True)

for name, fn in icons.items():
    pixels = render(fn)
    data = make_png(pixels)
    path = os.path.join(out_dir, f'{name}.png')
    with open(path, 'wb') as f:
        f.write(data)
    print(f'  {path}  ({len(data)} bytes)')

print('Done.')
