#!/usr/bin/env python3
"""Generates the app icon and its symbolic variant.

The scalloped disc uses the same polar formula as `src/paint.rs`, so the icon and the note
sticker in the app are literally the same shape. Run from the repository root:

    python3 data/icons/make-icons.py
"""

import math

# --- the twelve-lobed scallop -------------------------------------------------------------------

def cookie(cx, cy, r, lobes=12, scallop=0.038, n=360):
    pts = []
    for i in range(n):
        t = i / n * 2 * math.pi
        rr = r * (1 - scallop + scallop * math.cos(lobes * t))
        pts.append((cx + rr * math.cos(t), cy + rr * math.sin(t)))
    return "M%.2f,%.2f " % pts[0] + " ".join("L%.2f,%.2f" % p for p in pts[1:]) + " Z"


# --- the tuning fork ----------------------------------------------------------------------------
# Laid out on a 16-unit grid: two long parallel tines, a half-annulus U-bend whose walls line up
# with them exactly, then a shorter handle ending in a small foot. Built from filled primitives,
# never strokes — GTK recolours symbolic icons by painting `fill`, and a stroke would survive
# that pass in its literal colour.

# A fork reads as a fork when the tines are clearly longer than the bend is wide. Earlier
# proportions had them about equal, which came out looking like a trident.
LIMB = 1.6          # thickness of tine, bend wall and handle alike
OFFSET = 1.85       # tine centre from the axis, which is also the bend's mean radius
BEND_Y = 8.4        # centre of the U-bend
TINE_TOP = 1.5
HANDLE_BOTTOM = 14.6


def fork(scale=1.0, dx=0.0, dy=0.0, fill="#222222"):
    def s(v):
        return v * scale

    half = LIMB / 2
    outer, inner = OFFSET + half, OFFSET - half
    bend_bottom = BEND_Y + outer
    tip_r = half * 0.45      # tine tips read as flat, not as capsules

    def rect(x, y, w, h, r):
        return '<rect x="%.2f" y="%.2f" width="%.2f" height="%.2f" rx="%.2f"/>' % (
            dx + s(x), dy + s(y), s(w), s(h), s(r))

    parts = [
        # tines
        rect(8 - OFFSET - half, TINE_TOP, LIMB, BEND_Y - TINE_TOP, tip_r),
        rect(8 + OFFSET - half, TINE_TOP, LIMB, BEND_Y - TINE_TOP, tip_r),
        # the U-bend: outer half-circle, back along the inner one
        '<path d="M%.2f,%.2f A%.2f,%.2f 0 0 0 %.2f,%.2f L%.2f,%.2f A%.2f,%.2f 0 0 1 %.2f,%.2f Z"/>'
        % (
            dx + s(8 - outer), dy + s(BEND_Y), s(outer), s(outer),
            dx + s(8 + outer), dy + s(BEND_Y),
            dx + s(8 + inner), dy + s(BEND_Y), s(inner), s(inner),
            dx + s(8 - inner), dy + s(BEND_Y),
        ),
        # handle: a plain rod with a rounded end, overlapping the bend so the join is seamless
        rect(8 - half, bend_bottom - 0.5, LIMB, HANDLE_BOTTOM - (bend_bottom - 0.5), half),
    ]
    return '<g fill="%s">%s</g>' % (fill, "".join(parts))


# --- files ---------------------------------------------------------------------------------------

FORK_HEIGHT = HANDLE_BOTTOM - TINE_TOP
FORK_MID = (HANDLE_BOTTOM + TINE_TOP) / 2
SCALE = 62.0 / FORK_HEIGHT           # sized to sit comfortably inside the disc

app = f'''<svg xmlns="http://www.w3.org/2000/svg" width="128" height="128" viewBox="0 0 128 128">
  <defs>
    <linearGradient id="base" x1="64" y1="8" x2="64" y2="120" gradientUnits="userSpaceOnUse">
      <stop offset="0" stop-color="#2f2b66"/>
      <stop offset="1" stop-color="#17153a"/>
    </linearGradient>
    <linearGradient id="disc" x1="64" y1="26" x2="64" y2="102" gradientUnits="userSpaceOnUse">
      <stop offset="0" stop-color="#f7d488"/>
      <stop offset="1" stop-color="#e8b74e"/>
    </linearGradient>
  </defs>
  <rect x="8" y="8" width="112" height="112" rx="18" fill="url(#base)"/>
  <path d="{cookie(64, 64, 39)}" fill="url(#disc)"/>
  {fork(scale=SCALE, dx=64 - 8 * SCALE, dy=64 - FORK_MID * SCALE, fill="#221f4d")}
</svg>
'''

symbolic = f'''<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 16 16">
  {fork()}
</svg>
'''

if __name__ == "__main__":
    import pathlib

    here = pathlib.Path(__file__).parent
    (here / "com.niconex.Vibes.svg").write_text(app)
    (here / "com.niconex.Vibes-symbolic.svg").write_text(symbolic)
    print("wrote com.niconex.Vibes.svg and com.niconex.Vibes-symbolic.svg")
