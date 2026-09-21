//! Everything hand-drawn: the palette derived from the libadwaita accent, the flowing wave
//! field, the scalloped note blob, and the marching chevrons.

use std::f64::consts::PI;

use relm4::adw;
use relm4::gtk::cairo::{Context, FillRule, LineCap, LineJoin};

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

    /// True when white text sits better on this colour than black — WCAG relative luminance.
    pub fn wants_light_text(self) -> bool {
        fn lin(c: f64) -> f64 {
            if c <= 0.03928 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
        }
        0.2126 * lin(self.0) + 0.7152 * lin(self.1) + 0.0722 * lin(self.2) < 0.45
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
    pub dark: bool,
    pub bg: Rgb,
    pub fg: Rgb,
    /// Three related-but-distinct tonalities for the wave field.
    pub bands: [Rgb; 3],
    pub accent: Rgb,
    pub error: Rgb,
    pub warning: Rgb,
    pub muted: Rgb,
}

impl Palette {
    /// Reads the user's current libadwaita accent and light/dark preference.
    pub fn current() -> Self {
        let sm = adw::StyleManager::default();
        let a = sm.accent_color().to_rgba();
        Palette::from_accent(
            Rgb(a.red() as f64, a.green() as f64, a.blue() as f64),
            sm.is_dark(),
        )
    }

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
                (s * sats[i] * 1.6).clamp(if dark { 0.12 } else { 0.28 }, if dark { 0.30 } else { 0.68 }),
                lights[i],
            )
        });

        Palette {
            dark,
            bg,
            fg,
            bands,
            accent,
            error,
            warning,
            muted: bg.mix(fg, 0.18),
        }
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
}

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
        }
    }

    /// Feed the current reading. Kept separate from `step` so the UI only touches targets.
    pub fn set_pitch(&mut self, freq: f32, cents: f32, in_tune: bool) {
        let has_pitch = freq > 0.0;
        let closeness = if has_pitch { 1.0 - (cents.abs() / 50.0).min(1.0) } else { 0.0 };
        self.energy_target = closeness;
        // Flat (cents < 0) drifts the bands UP, sharp (cents > 0) DOWN, faster the further from
        // centre, still when perfectly in tune.
        self.speed_target = if has_pitch {
            MAX_WAVE_PPS * (cents / 50.0).clamp(-1.0, 1.0)
        } else {
            0.0
        };
        self.in_tune = in_tune;
        self.blob_target = match () {
            _ if !has_pitch => self.palette.muted,
            _ if in_tune => self.palette.accent,
            _ if cents > 0.0 => self.palette.warning,
            _ => self.palette.error,
        };
        // Wobble character encodes state: a calm sway when idle, a nervous fast wobble while a
        // note is off pitch, settling once locked.
        let (amp, period) = match () {
            _ if !has_pitch => (3.0, 2.8),
            _ if in_tune => (1.5, 2.2),
            _ => (7.0, 0.78),
        };
        self.wobble_amp_target = amp;
        self.wobble_period = period;
        // The blob vibrates at the note itself, dropped by whole octaves until it lands in a
        // range the eye can follow. A4 = 440 Hz → 6.9 Hz; the low E of a guitar → 5.2 Hz. Same
        // note, same shimmer, every time.
        if has_pitch {
            let mut hz = freq;
            while hz >= 8.0 {
                hz *= 0.5;
            }
            while hz < 4.0 {
                hz *= 2.0;
            }
            self.vib_hz = hz;
        }
        // It rings hard while the note is off, and calms to a shimmer once locked.
        self.vib_amp_target = match () {
            _ if !has_pitch => 0.0,
            _ if in_tune => 0.016,
            _ => 0.020 + 0.030 * (cents.abs() / 50.0).min(1.0),
        };
        self.up_target = if has_pitch && !in_tune && cents < 0.0 { 1.0 } else { 0.0 };
        self.down_target = if has_pitch && !in_tune && cents > 0.0 { 1.0 } else { 0.0 };
    }

    /// Advance one frame. Exponential chase rather than fixed-duration tweens: changing the
    /// target mid-flight never snaps or restarts, which is what makes the motion feel physical.
    pub fn step(&mut self, dt: f64) {
        let dt32 = dt as f32;
        self.time += dt;
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
        self.wobble_phase =
            (self.wobble_phase + dt32 / self.wobble_period).rem_euclid(1.0);
        self.vib_amp = approach(self.vib_amp, self.vib_amp_target, 0.20, dt32);
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
    let p = &a.palette;
    set(cr, p.bg);
    let _ = cr.paint();

    let spacing = h / 3.4;
    let period = 3.0 * spacing;
    let move_y = a.scroll as f64 * period;
    let amp = h * (0.014 + 0.05 * a.energy as f64);
    let thickness = spacing * 0.62;
    let kx = 1.5 * 2.0 * PI;
    let ripple = a.time * (2.0 * PI / 3.2); // one full traverse every 3.2 s
    let step = (w / 320.0).max(1.0);
    let count = (h / spacing) as i32;

    cr.set_line_width(thickness);
    cr.set_line_cap(LineCap::Round);
    cr.set_line_join(LineJoin::Round);
    for k in -4..=count + 4 {
        let y = k as f64 * spacing + spacing * 0.5 + move_y;
        if y < -spacing || y > h + spacing {
            continue;
        }
        let kk = (k.rem_euclid(3)) as usize;
        set(cr, p.bands[kk]);
        // Each band gets its own amplitude and a second harmonic, so the crests lean and the
        // three tonalities never trace the same curve — the field reads as woven, not ruled.
        let swell = [1.0, 0.78, 1.18][kk];
        let phase = kk as f64 * 2.094;
        let mut x = 0.0;
        let mut first = true;
        while x <= w {
            let u = kx * (x / w);
            let yy = y
                + amp * swell * ((u + ripple + phase).sin()
                    + 0.26 * (2.0 * u + 1.7 * ripple + phase * 1.6).sin());
            if first {
                cr.move_to(x, yy);
                first = false;
            } else {
                cr.line_to(x, yy);
            }
            x += step;
        }
        let _ = cr.stroke();
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
    const SAMPLES: usize = 720;
    let osc = (2.0 * PI * vib_phase).sin();
    let scallop = SCALLOP * (1.0 + 3.2 * vib * osc);
    let scale = r * (1.0 + 0.5 * vib * osc);
    for i in 0..=SAMPLES {
        let t = i as f64 / SAMPLES as f64 * 2.0 * PI;
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
    let (cx, cy) = (w / 2.0, h / 2.0);
    // Breathe only once locked — the shape settles into a slow heartbeat.
    let breathe = if a.in_tune {
        1.0 + 0.025 * (a.time * 2.0 * PI / 1.8).sin()
    } else {
        1.0
    };
    // Fills whatever box it is given, up to a ceiling — so it scales down to a phone and stops
    // growing into a dinner plate on a maximised desktop window.
    let base = w.min(h).min(330.0) / 2.0 * 0.96;
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
    if appear < 0.004 {
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
