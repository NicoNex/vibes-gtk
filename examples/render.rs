//! Offscreen preview of the painted layer — `cargo run --example render`.
//! Writes light/dark PNGs so the wave field, the blob and the chevrons can be eyeballed
//! without launching the app. Dev tool only; it ships in no binary.

#[path = "../src/paint.rs"]
mod paint;

use paint::{Anim, Palette, Rgb};
use relm4::gtk::cairo::{Context, Format, ImageSurface};

const W: i32 = 460;
const H: i32 = 880;

fn scene(name: &str, dark: bool, freq: f32, cents: f32) {
    // Adwaita's default blue accent, so the preview matches a stock GNOME session.
    let palette = Palette::from_accent(Rgb(0.208, 0.518, 0.894), dark);
    let mut anim = Anim::new(palette);
    let in_tune = freq > 0.0 && cents.abs() <= 5.0;
    anim.set_pitch(freq, cents, in_tune);
    // Settle the chase so the still frame shows the state it converges to.
    for _ in 0..240 {
        anim.step(1.0 / 60.0);
    }

    let surface = ImageSurface::create(Format::ARgb32, W, H).unwrap();
    let cr = Context::new(&surface).unwrap();
    paint::draw_waves(&cr, W as f64, H as f64, &anim);

    let (w, h) = (W as f64, H as f64);
    let blob = 312.0;
    let top = (h - (44.0 + 76.0 + blob + 76.0 + 48.0)) / 2.0;

    section(&cr, 0.0, top + 44.0, w, |cr| {
        paint::draw_chevrons(cr, 300.0, 76.0, true, anim.up, &anim)
    });
    section(&cr, (w - blob) / 2.0, top + 120.0, blob, |cr| {
        paint::draw_blob(cr, blob, blob, &anim)
    });
    section(&cr, 0.0, top + 120.0 + blob, w, |cr| {
        paint::draw_chevrons(cr, 300.0, 76.0, false, anim.down, &anim)
    });

    let mut out = std::fs::File::create(format!("/tmp/vibes-{name}.png")).unwrap();
    surface.write_to_png(&mut out).unwrap();
    println!("wrote /tmp/vibes-{name}.png");
}

fn section(cr: &Context, x: f64, y: f64, w: f64, draw: impl FnOnce(&Context)) {
    cr.save().unwrap();
    // Chevrons are drawn centred in their own 300-wide box, so centre that box in the window.
    cr.translate(if w > 300.0 { x + (w - 300.0) / 2.0 } else { x }, y);
    draw(cr);
    cr.restore().unwrap();
}

fn main() {
    scene("light-flat", false, 194.0, -22.0);
    scene("dark-in-tune", true, 440.0, 0.0);
    scene("light-sharp", false, 330.0, 31.0);
}
