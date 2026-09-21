# Vibes 🎸 — GTK4 / libadwaita

A chromatic instrument tuner. Rust port of [Vibes for Android](../Vibes), keeping the editorial
look — full-bleed wavy bands, a scalloped note sticker, physics-driven motion — but rebuilt on
**Adwaita's** own colours and widgets instead of Material 3 Expressive.

Built with **relm4** on gtk4-rs + libadwaita.

## What the port keeps

- **Chromatic detection** — the same YIN detector (DC removal, low silence gate, median
  smoothing, octave-error correction, grace hold), ported line for line with its tests.
- **Glanceable feedback** — the note in a big serif inside a 12-lobed scalloped blob, a status
  pill (IN TUNE / SHARP ↓ / FLAT ↑), marching chevrons, and a field of wavy bands that drifts
  **up when flat / down when sharp**, faster the further off you are and still once you lock in.
- **The sticker rings at the note** — the blob's lobes pump at the detected frequency itself,
  dropped by whole octaves into a range the eye can follow (A4 → 6.9 Hz). Hard while the note is
  off, a shimmer once it locks.
- **Adaptive** — one layout from 360×294 (GNOME Mobile's floor) to a maximised desktop window:
  the blob, the chevrons and the note's typography all size themselves from the space they get.
- **Settings** — reference pitch (415–466 Hz), note names (`A B C` / `Do Re Mi`), and sustain
  (0.5–2.5 s, with a magnetic detent on the 1.2 s default).

## What the port changes

| Android | Here |
|---|---|
| Material You wallpaper palette | The libadwaita **accent colour**, followed live along with light/dark |
| `MaterialShapes.Cookie12Sided` | The same silhouette drawn in cairo as a disc unioned with twelve lobes |
| `MotionScheme` spring specs | Frame-rate independent exponential chase on every animated value |
| Compose predictive back | `AdwWindow` settings window with the system decorations |
| Runtime mic permission | The desktop has none; a failure shows an `AdwStatusPage` instead |
| Haptics on lock and slider steps | Dropped — desktops have no vibrator |

## Build & run

Needs GTK 4 and libadwaita 1.7+ development packages.

```bash
cargo run --release
```

```bash
cargo test                      # the pitch DSP and note math
cargo run --example render      # offscreen PNGs of the painted layer, into /tmp
```

## Layout

| File | |
|---|---|
| `src/pitch.rs` | YIN detector + note math, no GTK, fully tested |
| `src/audio.rs` | cpal capture → detection worker → Hz |
| `src/paint.rs` | Palette, animation state, and every cairo drawing routine |
| `src/main.rs` | The tuner window |
| `src/settings.rs` | The settings window |
| `src/style.css` | Chip, readout and note typography, on libadwaita named colours |

## License

GPL-3.0-or-later. See [`LICENSE`](LICENSE).
