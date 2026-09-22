//! Everything hand-drawn: the palette derived from the libadwaita accent, the flowing wave
//! field, the scalloped note blob, and the marching chevrons.

use std::f64::consts::PI;

use relm4::gtk::cairo::{Context, LineCap, LineJoin};
#[allow(deprecated)]
use relm4::gtk::prelude::{StyleContextExt, WidgetExt};
use relm4::{adw, gtk};

// ---------------------------------------------------------------------------------------------
// Colour
// ---------------------------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq)]
pub struct Rgb(pub f64, pub f64, pub f64);

impl Rgb {
    fn hex(v: u32) -> Self {
        Rgb(
            ((v >> 16) & 0xff) as f64 / 255.0,
            ((v >> 8) & 0xff) as f64 / 255.0,
            (v & 0xff) as f64 / 255.0,
        )
    }

    pub fn mix(self, other: Rgb, t: f64) -> Rgb {
        Rgb(
            self.0 + (other.0 - self.0) * t,
            self.1 + (other.1 - self.1) * t,
            self.2 + (other.2 - self.2) * t,
        )
    }

    /// True when white text on this colour clears WCAG's 3:1 for large text — the note is
    /// display-sized. 1.05 / (L + 0.05) >= 3 puts the crossover at L = 0.30; the old 0.45 put
    /// white on Adwaita's amber at about 2:1.
    pub fn wants_light_text(self) -> bool {
        fn lin(c: f64) -> f64 {
            if c <= 0.03928 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        }
        0.2126 * lin(self.0) + 0.7152 * lin(self.1) + 0.0722 * lin(self.2) <= 0.30
    }

    fn to_hsl(self) -> (f64, f64, f64) {
        let (r, g, b) = (self.0, self.1, self.2);
        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let l = (max + min) / 2.0;
        if (max - min).abs() < 1e-9 {
            return (0.0, 0.0, l);
        }
        let d = max - min;
        let s = if l > 0.5 { d / (2.0 - max - min) } else { d / (max + min) };
        let h = if max == r {
            ((g - b) / d + if g < b { 6.0 } else { 0.0 }) / 6.0
        } else if max == g {
            ((b - r) / d + 2.0) / 6.0
        } else {
            ((r - g) / d + 4.0) / 6.0
        };
        (h * 360.0, s, l)
    }

    fn from_hsl(h: f64, s: f64, l: f64) -> Rgb {
        let h = h.rem_euclid(360.0) / 360.0;
        if s <= 0.0 {
            return Rgb(l, l, l);
        }
        let q = if l < 0.5 { l * (1.0 + s) } else { l + s - l * s };
        let p = 2.0 * l - q;
        let f = |mut t: f64| {
            t = t.rem_euclid(1.0);
            if t < 1.0 / 6.0 {
                p + (q - p) * 6.0 * t
            } else if t < 0.5 {
                q
            } else if t < 2.0 / 3.0 {
                p + (q - p) * (2.0 / 3.0 - t) * 6.0
            } else {
                p
            }
        };
        Rgb(f(h + 1.0 / 3.0), f(h), f(h - 1.0 / 3.0))
    }
}

/// The whole look is derived from one input — the user's libadwaita accent colour — the way the
/// Android original derives itself from the wallpaper. Light and dark are separate recipes.
#[derive(Clone, Copy, PartialEq)]
pub struct Palette {
    pub bg: Rgb,
    pub fg: Rgb,
    /// Three related-but-distinct tonalities for the wave field.
    pub bands: [Rgb; 3],
    pub accent: Rgb,
    pub error: Rgb,
    pub warning: Rgb,
    pub muted: Rgb,
}

/// The Adwaita palette's hue families, in wheel order. libadwaita defines `blue_1` … `blue_5`
/// and friends as named colours, so these are the system's own shades, not ours.
const WHEEL: [&str; 6] = ["blue", "purple", "red", "orange", "yellow", "green"];

/// Where the user's accent sits on that wheel. Teal, pink and slate have no palette family of
/// their own, so they borrow their nearest neighbour's.
fn wheel_index(accent: adw::AccentColor) -> usize {
    match accent {
        adw::AccentColor::Purple | adw::AccentColor::Pink => 1,
        adw::AccentColor::Red => 2,
        adw::AccentColor::Orange => 3,
        adw::AccentColor::Yellow => 4,
        adw::AccentColor::Green | adw::AccentColor::Teal => 5,
        _ => 0, // Blue, Slate, and anything a future libadwaita adds
    }
}

/// Resolves a theme colour by name through a throwaway widget. `lookup_color` is deprecated but
/// it is still the one call that answers "what is `@blue_2` on this display, right now" — and
/// every caller below has a fallback for when it answers nothing.
#[allow(deprecated)]
fn theme_color(probe: &gtk::Label, name: &str) -> Option<Rgb> {
    probe
        .style_context()
        .lookup_color(name)
        .map(|c| Rgb(c.red() as f64, c.green() as f64, c.blue() as f64))
}

impl Palette {
    /// Builds the look out of the colours the system hands us: the accent, the window and
    /// semantic colours, and three hue families from the Adwaita palette for the wave bands.
    /// Anything the running theme does not define falls back to deriving it from the accent.
    pub fn current() -> Self {
        let sm = adw::StyleManager::default();
        let dark = sm.is_dark();
        let probe = gtk::Label::new(None);

        let rgba = sm.accent_color().to_rgba();
        let accent = theme_color(&probe, "accent_bg_color").unwrap_or(Rgb(
            rgba.red() as f64,
            rgba.green() as f64,
            rgba.blue() as f64,
        ));

        let mut palette = Palette::from_accent(accent, dark);
        if let Some(c) = theme_color(&probe, "window_bg_color") {
            palette.bg = c.mix(accent, if dark { 0.05 } else { 0.045 });
        }
        if let Some(c) = theme_color(&probe, "window_fg_color") {
            palette.fg = c;
        }
        palette.muted = palette.bg.mix(palette.fg, 0.18);
        if let Some(c) = theme_color(&probe, "error_bg_color") {
            palette.error = c;
        }
        if let Some(c) = theme_color(&probe, "warning_bg_color") {
            palette.warning = c;
        }

        // Three neighbouring families starting at the accent's own — blue, purple, red for the
        // default accent, which is exactly the lavender / periwinkle / pink of the original.
        // The palette's shades are vivid on their own, so each is settled toward the window
        // ground until it reads as a background band rather than a button.
        let start = wheel_index(sm.accent_color());
        let shade = if dark { 5 } else { 1 };
        let toward_ground = if dark { 0.55 } else { 0.32 };
        let from_palette: [Option<Rgb>; 3] = std::array::from_fn(|i| {
            theme_color(&probe, &format!("{}_{}", WHEEL[(start + i) % WHEEL.len()], shade))
                .map(|c| c.mix(palette.bg, toward_ground))
        });
        if let [Some(a), Some(b), Some(c)] = from_palette {
            palette.bands = [a, b, c];
        }
        palette
    }

    /// The fallback recipe: everything derived from one accent colour by hue and lightness.
    pub fn from_accent(accent: Rgb, dark: bool) -> Self {
        // libadwaita's own window/semantic colours, so the painted layer and the CSS layer agree.
        let fg = if dark { Rgb::hex(0xffffff) } else { Rgb::hex(0x000000) };
        let error = if dark { Rgb::hex(0xc01c28) } else { Rgb::hex(0xe01b24) };
        let warning = if dark { Rgb::hex(0xcd9309) } else { Rgb::hex(0xe5a50a) };
        // Adwaita's window background, barely tinted toward the accent so the gaps between the
        // bands belong to the same world as the bands themselves.
        let ground = if dark { Rgb::hex(0x1d1d20) } else { Rgb::hex(0xfafafb) };
        let bg = ground.mix(accent, if dark { 0.05 } else { 0.045 });

        // Three tonalities walking one way around the wheel from the accent: related enough to
        // read as one family, distinct enough to tell the bands apart. Pastel over the near-white
        // ground, deep and smoky over the near-black one.
        let (h, s, _) = accent.to_hsl();
        // Wide enough apart to be three colours rather than three shades of one — the way M3's
        // primary / secondary / tertiary containers read in the original.
        let hues = [0.0, 42.0, 86.0];
        let lights = if dark { [0.265, 0.315, 0.235] } else { [0.885, 0.845, 0.915] };
        let sats = if dark { [0.30, 0.24, 0.34] } else { [0.52, 0.62, 0.44] };
        let bands = std::array::from_fn(|i| {
            Rgb::from_hsl(
                h + hues[i],
                (s * sats[i] * 1.6)
                    .clamp(if dark { 0.12 } else { 0.28 }, if dark { 0.30 } else { 0.68 }),
                lights[i],
            )
        });

        Palette { bg, fg, bands, accent, error, warning, muted: bg.mix(fg, 0.18) }
    }
}

fn set(cr: &Context, c: Rgb) {
    cr.set_source_rgb(c.0, c.1, c.2);
}

fn set_a(cr: &Context, c: Rgb, a: f64) {
    cr.set_source_rgba(c.0, c.1, c.2, a);
}

// ---------------------------------------------------------------------------------------------
// Animation state
// ---------------------------------------------------------------------------------------------

/// Everything the three drawing areas need for one frame. Lives in an `Rc<RefCell<_>>` shared
/// between the frame clock tick and the draw functions, so 60 fps never touches the relm4 loop.
pub struct Anim {
    pub palette: Palette,
    pub time: f64,

    // Targets are written by the update loop; the values chase them every frame.
    pub speed_target: f32,
    pub speed: f32,
    pub energy_target: f32,
    pub energy: f32,
    pub scroll: f32,
    pub in_tune: bool,
    pub wash: f32,

    pub blob_target: Rgb,
    pub blob: Rgb,
    pub wobble_amp_target: f32,
    pub wobble_amp: f32,
    pub wobble_period: f32,
    pub wobble_phase: f32,

    /// The note's own frequency, octave-shifted down into a range the eye can follow.
    pub vib_hz: f32,
    pub vib_phase: f32,
    pub vib_amp_target: f32,
    pub vib_amp: f32,

    pub up_target: f32,
    pub up: f32,
    pub down_target: f32,
    pub down: f32,

    /// The system asked for no animations: decoration freezes, the directional drift stays.
    pub reduced: bool,
}

/// The blob stops growing here, so it scales down to a phone without becoming a dinner
/// plate on a maximised desktop window. main.rs sizes the note's type from the same value.
pub const MAX_BLOB_PX: f64 = 330.0;

/// Max background scroll speed, in colour-periods per second.
const MAX_WAVE_PPS: f32 = 0.22;

impl Anim {
    pub fn new(palette: Palette) -> Self {
        Anim {
            palette,
            time: 0.0,
            speed_target: 0.0,
            speed: 0.0,
            energy_target: 0.0,
            energy: 0.0,
            scroll: 0.0,
            in_tune: false,
            wash: 0.0,
            blob_target: palette.muted,
            blob: palette.muted,
            wobble_amp_target: 3.0,
            wobble_amp: 3.0,
            wobble_period: 2.8,
            wobble_phase: 0.0,
            vib_hz: 6.0,
            vib_phase: 0.0,
            vib_amp_target: 0.0,
            vib_amp: 0.0,
            up_target: 0.0,
            up: 0.0,
            down_target: 0.0,
            down: 0.0,
            reduced: false,
        }
    }

    /// Feed the current reading. Kept separate from `step` so the UI only touches targets.
    pub fn set_pitch(&mut self, freq: f32, cents: f32, in_tune: bool) {
        let has_pitch = freq > 0.0;
        let closeness = if has_pitch { 1.0 - (cents.abs() / 50.0).min(1.0) } else { 0.0 };
        self.energy_target = closeness;
        // Flat (cents < 0) drifts the bands UP, sharp (cents > 0) DOWN, faster the further from
        // centre, still when perfectly in tune.
        self.speed_target =
            if has_pitch { MAX_WAVE_PPS * (cents / 50.0).clamp(-1.0, 1.0) } else { 0.0 };
        self.in_tune = in_tune;
        self.blob_target = if !has_pitch {
            self.palette.muted
        } else if in_tune {
            self.palette.accent
        } else if cents > 0.0 {
            self.palette.warning
        } else {
            self.palette.error
        };
        // A slow sway only. The ring below is what shows a note is off; a fast twist on top of
        // it (it used to be ±7° every 0.78 s) fought the ring and read as jitter.
        let (amp, period) = if !has_pitch {
            (3.0, 2.8)
        } else if in_tune {
            (1.0, 3.2)
        } else {
            (2.0, 2.4)
        };
        self.wobble_amp_target = amp;
        self.wobble_period = period;
        // The blob rings at the note itself, dropped by whole octaves into 2–4 Hz: A4 = 440 Hz
        // → 3.4 Hz; the low E of a guitar → 2.6 Hz. Same note, same ring, every time. One octave
        // higher, 4–8 Hz, is the band the eye reads as a tremor rather than a swing.
        if has_pitch {
            let mut hz = freq;
            while hz >= 4.0 {
                hz *= 0.5;
            }
            while hz < 2.0 {
                hz *= 2.0;
            }
            self.vib_hz = hz;
        }
        // It rings while the note is off and falls still once locked — only the breath is left.
        self.vib_amp_target =
            if !has_pitch || in_tune { 0.0 } else { 0.020 + 0.030 * (cents.abs() / 50.0).min(1.0) };
        self.up_target = if has_pitch && !in_tune && cents < 0.0 { 1.0 } else { 0.0 };
        self.down_target = if has_pitch && !in_tune && cents > 0.0 { 1.0 } else { 0.0 };
    }

    /// Advance one frame. Exponential chase rather than fixed-duration tweens: changing the
    /// target mid-flight never snaps or restarts, which is what makes the motion feel physical.
    pub fn step(&mut self, dt: f64) {
        let dt32 = dt as f32;
        // Reduced motion stops the clock every ornament runs on (ripple, breath, pulse, the
        // chevrons' march) and silences the ring and sway. The drift up or down is the tuning
        // instruction itself, so it keeps moving.
        if self.reduced {
            self.vib_amp_target = 0.0;
            self.wobble_amp_target = 0.0;
        } else {
            self.time += dt;
        }
        self.speed = approach(self.speed, self.speed_target, 0.35, dt32);
        self.energy = approach(self.energy, self.energy_target, 0.30, dt32);
        self.wobble_amp = approach(self.wobble_amp, self.wobble_amp_target, 0.25, dt32);
        self.up = approach(self.up, self.up_target, 0.12, dt32);
        self.down = approach(self.down, self.down_target, 0.12, dt32);
        self.wash = approach(self.wash, if self.in_tune { 1.0 } else { 0.0 }, 0.30, dt32);
        self.blob = Rgb(
            approach(self.blob.0 as f32, self.blob_target.0 as f32, 0.18, dt32) as f64,
            approach(self.blob.1 as f32, self.blob_target.1 as f32, 0.18, dt32) as f64,
            approach(self.blob.2 as f32, self.blob_target.2 as f32, 0.18, dt32) as f64,
        );
        // Frame-driven scroll: position integrates the (variable, signed) speed, so changing
        // tempo or direction never teleports. Wraps at one colour period → no seam.
        self.scroll = (self.scroll + self.speed * dt32).rem_euclid(1.0);
        self.wobble_phase = (self.wobble_phase + dt32 / self.wobble_period).rem_euclid(1.0);
        // Slower than the 93 ms detection window, so per-window cents jitter never shows as flutter.
        self.vib_amp = approach(self.vib_amp, self.vib_amp_target, 0.35, dt32);
        self.vib_phase = (self.vib_phase + self.vib_hz * dt32).rem_euclid(1.0);
    }
}

/// Frame-rate independent exponential approach. `tau` is the time constant in seconds.
fn approach(cur: f32, target: f32, tau: f32, dt: f32) -> f32 {
    cur + (target - cur) * (1.0 - (-dt / tau).exp())
}

// ---------------------------------------------------------------------------------------------
// The wave field
// ---------------------------------------------------------------------------------------------

/// Full-bleed field of flat wavy bands in different palette tonalities, stacked and scrolling UP
/// when flat / DOWN when sharp. The ripple swells with resonance as the pitch nears in-tune,
/// where the bands settle and a soft accent wash confirms the lock.
pub fn draw_waves(cr: &Context, w: f64, h: f64, a: &Anim) {
    // A widget can be allocated 0x0 mid-layout, and every formula below divides by one of these.
    if w <= 0.0 || h <= 0.0 {
        return;
    }
    let p = &a.palette;
    set(cr, p.bg);
    let _ = cr.paint();

    let spacing = h / 3.4;
    let period = 3.0 * spacing;
    let move_y = a.scroll as f64 * period;
    // The swell is sized from the height but the wavelength from the width, so a tall narrow
    // window made steep flanks — where a band reads as thin and the gaps between bands narrow to
    // spikes. The whole range is scaled down until the steepest flank stays under MAX_SLOPE.
    const HARMONIC: f64 = 0.10;
    const MAX_SWELL: f64 = 1.18;
    const MAX_SLOPE: f64 = 0.9;
    // Humps across the window. More of them means a shorter wavelength, which MAX_SLOPE then
    // answers with a lower swell, so the field gets busier without getting steeper.
    let kx = 2.5 * 2.0 * PI;
    let full = h * 0.054;
    let cap = MAX_SLOPE / (kx / w * (1.0 + 2.0 * HARMONIC) * MAX_SWELL);
    let amp = full.min(cap) * (0.014 + 0.04 * a.energy as f64) / 0.054;
    let thickness = spacing * 0.62;
    let ripple = a.time * (2.0 * PI / 3.2); // one full traverse every 3.2 s
                                            // One vertex every ~3 device pixels: fine enough that no facet shows on a crest, and the
                                            // cost scales with the window instead of with a fixed vertex budget.
    let step = 3.0;
    let count = (h / spacing) as i32;

    for k in -4..=count + 4 {
        let y = k as f64 * spacing + spacing * 0.5 + move_y;
        if y < -spacing || y > h + spacing {
            continue;
        }
        let kk = (k.rem_euclid(3)) as usize;
        set(cr, p.bands[kk]);
        // Each band gets its own amplitude and a second harmonic, so the crests lean and the
        // three tonalities never trace the same curve — the field reads as woven, not ruled.
        let a_px = amp * [1.0, 0.78, MAX_SWELL][kk];
        let phase = kk as f64 * 2.094;
        let centre = |x: f64| {
            let u = kx * (x / w);
            y + a_px
                * ((u + ripple + phase).sin()
                    + HARMONIC * (2.0 * u + 1.7 * ripple + phase * 1.6).sin())
        };
        // Filled between the centre line shifted up and down, not stroked: a stroke this thick
        // offsets along the normal, and wherever a crest bends tighter than half the width the
        // outline folds over itself into lumps and corners. A vertical offset never folds; it
        // does thin a band by cos(slope) on the flanks, which MAX_SLOPE keeps to about a quarter.
        let edge = |x: f64, side: f64| centre(x) + side * thickness / 2.0;
        let n = (w / step).ceil() as usize;
        let xs = (0..=n).map(|i| (i as f64 * step).min(w));
        for (i, x) in xs.clone().enumerate() {
            let yy = edge(x, -1.0);
            if i == 0 {
                cr.move_to(x, yy);
            } else {
                cr.line_to(x, yy);
            }
        }
        for x in xs.rev() {
            cr.line_to(x, edge(x, 1.0));
        }
        cr.close_path();
        let _ = cr.fill();
    }

    if a.wash > 0.001 {
        // A slow breath of accent over the whole field: the lock-in confirmation.
        let pulse = 0.86 + 0.14 * (a.time * 2.0 * PI / 1.5).sin();
        set_a(cr, p.accent, 0.12 * a.wash as f64 * pulse);
        let _ = cr.paint();
    }
}

// ---------------------------------------------------------------------------------------------
// The note blob
// ---------------------------------------------------------------------------------------------

/// A 12-lobed scalloped cookie, the answer to `MaterialShapes.Cookie12Sided`.
///
/// One polar radius, sampled finely. Unioning circles gave true round lobes but joined them to
/// the body at a corner, which read as lumpy; a single smooth radius has no joins to go wrong.
///
/// `vib` rings the shape at the note it hears: the twelve lobes pump in and out and the whole
/// sticker pulses with them, in step. Kept radially symmetric on purpose — a travelling mode
/// around the rim just made the silhouette look lopsided.
fn cookie_path(cr: &Context, cx: f64, cy: f64, r: f64, rotation: f64, vib: f64, vib_phase: f64) {
    const LOBES: f64 = 12.0;
    const SCALLOP: f64 = 0.038;
    // Same rule as the wave field: roughly one vertex per two pixels of rim, bounded.
    let samples = ((r * PI) as usize).clamp(240, 900);
    let osc = (2.0 * PI * vib_phase).sin();
    let scallop = SCALLOP * (1.0 + 3.2 * vib * osc);
    let scale = r * (1.0 + 0.3 * vib * osc);
    for i in 0..=samples {
        let t = i as f64 / samples as f64 * 2.0 * PI;
        let rr = scale * (1.0 - scallop + scallop * (LOBES * (t + rotation)).cos());
        let (x, y) = (cx + rr * t.cos(), cy + rr * t.sin());
        if i == 0 {
            cr.move_to(x, y);
        } else {
            cr.line_to(x, y);
        }
    }
    cr.close_path();
}

pub fn draw_blob(cr: &Context, w: f64, h: f64, a: &Anim) {
    if w <= 0.0 || h <= 0.0 {
        return;
    }
    let (cx, cy) = (w / 2.0, h / 2.0);
    // Breathe only once locked — the shape settles into a slow heartbeat.
    let breathe = if a.in_tune { 1.0 + 0.025 * (a.time * 2.0 * PI / 1.8).sin() } else { 1.0 };
    let base = w.min(h).min(MAX_BLOB_PX) / 2.0 * 0.96;
    let r = base * breathe;
    // Triangle-wave sway rather than a sine: reaches the extremes with a touch more character.
    let osc = ((a.wobble_phase as f64 * 2.0 * PI).sin()) * a.wobble_amp as f64;
    let rotation = osc.to_radians();

    // ponytail: no drop shadow. The look is flat saturated colour on flat colour, and a fake
    // blur (stacked fading outlines — cairo has no cheap gaussian) muddied the edge.
    set(cr, a.blob);
    cookie_path(cr, cx, cy, r, rotation, a.vib_amp as f64, a.vib_phase as f64);
    let _ = cr.fill();
}

// ---------------------------------------------------------------------------------------------
// Direction chevrons
// ---------------------------------------------------------------------------------------------

/// Marching chevrons showing which way to turn the peg: up = raise (flat), down = lower (sharp).
pub fn draw_chevrons(cr: &Context, w: f64, h: f64, point_up: bool, appear: f32, a: &Anim) {
    if appear < 0.004 || w <= 0.0 || h <= 0.0 {
        return;
    }
    let p = &a.palette;
    let cx = w / 2.0;
    // Three chevrons plus their gaps come to 4.6 chevron-heights, so size from the space given
    // and never overflow it — that is what keeps the row honest from a phone to a wide window.
    let ch = (h / 4.6).min(22.0);
    let cw = (ch / 0.55).min(w * 0.22);
    let gap = ch * 1.8;
    let top = (h - (gap * 2.0 + ch)) / 2.0;
    // Stroke weights ride on the chevron height. Fixed widths merged the three into one smear
    // as soon as the row got short.
    let (halo_w, ink_w) = (ch * 0.80, ch * 0.46);
    let phase = (a.time / 1.1).rem_euclid(1.0) * 3.0;

    cr.set_line_cap(LineCap::Round);
    cr.set_line_join(LineJoin::Round);
    for i in 0..3 {
        // Brightness travels toward the pointing direction, so the row "marches".
        let order = if point_up { 2 - i } else { i } as f64;
        let d = (phase - order).rem_euclid(3.0);
        let wave = 1.0 - d / 3.0;
        let alpha = appear as f64 * (0.45 + 0.55 * wave * wave);
        let cy = top + gap * i as f64;
        let (y_tip, y_base) = if point_up { (cy, cy + ch) } else { (cy + ch, cy) };

        let chevron = |width: f64, colour: Rgb, alpha: f64| {
            cr.set_line_width(width);
            set_a(cr, colour, alpha);
            cr.move_to(cx - cw, y_base);
            cr.line_to(cx, y_tip);
            cr.line_to(cx + cw, y_base);
            let _ = cr.stroke();
        };
        // A neutral halo first so the arrows read against any wave-band colour.
        chevron(halo_w, p.bg, alpha * 0.75);
        chevron(ink_w, p.accent, alpha);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Scroll after `secs` of a steady reading, unwrapped: negative is up the screen.
    fn drift(freq: f32, cents: f32, secs: usize) -> f32 {
        let mut a = Anim::new(Palette::from_accent(Rgb(0.2, 0.5, 0.9), false));
        a.set_pitch(freq, cents, cents.abs() <= 5.0);
        let mut total = 0.0;
        for _ in 0..secs * 60 {
            a.step(1.0 / 60.0);
            total += a.speed / 60.0;
        }
        total
    }

    #[test]
    fn waves_rise_when_flat_fall_when_sharp_and_rest_in_tune() {
        assert!(drift(430.0, -20.0, 2) < 0.0, "flat must drift up");
        assert!(drift(450.0, 20.0, 2) > 0.0, "sharp must drift down");
        assert!(drift(440.0, 0.0, 2).abs() < 1e-6, "in tune must be still");
        assert!(drift(-1.0, 0.0, 2).abs() < 1e-6, "silence must be still");
        // Further off, faster.
        assert!(drift(420.0, -40.0, 2) < drift(430.0, -10.0, 2));
    }

    #[test]
    fn note_ink_is_legible_on_every_state_colour() {
        // Adwaita's light and dark warning ambers want dark ink; its accent blue and error red
        // keep white, as libadwaita's own accent-fg does.
        assert!(!Rgb::hex(0xe5a50a).wants_light_text());
        assert!(!Rgb::hex(0xcd9309).wants_light_text());
        assert!(Rgb::hex(0x3584e4).wants_light_text());
        assert!(Rgb::hex(0xe01b24).wants_light_text());
        assert!(Rgb::hex(0xc01c28).wants_light_text());
    }

    #[test]
    fn reduced_motion_freezes_ornament_but_keeps_the_drift() {
        let mut a = Anim::new(Palette::from_accent(Rgb(0.2, 0.5, 0.9), false));
        a.reduced = true;
        a.set_pitch(430.0, -20.0, false);
        for _ in 0..120 {
            a.step(1.0 / 60.0);
        }
        assert_eq!(a.time, 0.0);
        assert!(a.vib_amp < 0.001 && a.wobble_amp < 0.1);
        assert!(a.speed < 0.0, "flat must still drift up");
    }
}
