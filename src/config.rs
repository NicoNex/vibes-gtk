//! Settings, persisted as `key=value` lines under the user config dir.
//! ponytail: a flat key=value file over GSettings — three values, and no schema to compile
//! and install before the app will even start. Move to GSettings if this ever ships in a
//! distro package that wants `gsettings` introspection.

use std::path::PathBuf;

use crate::pitch::{NOTE_NAMES_LETTER, NOTE_NAMES_SOLFEGE};

/// Seconds a note is held after the string fades.
pub const SUSTAIN_DEFAULT: f32 = 1.2;
pub const A4_RANGE: (f32, f32) = (415.0, 466.0);
pub const SUSTAIN_RANGE: (f32, f32) = (0.5, 2.5);

#[derive(Clone, Copy, Debug)]
pub struct Config {
    pub a4: f32,
    pub sustain: f32,
    pub solfege: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self { a4: 440.0, sustain: SUSTAIN_DEFAULT, solfege: false }
    }
}

/// `"nan"` and `"inf"` both parse as f32, and `clamp` passes NaN straight through — which would
/// reach a GtkAdjustment and wedge the settings window. Only finite numbers get past here.
fn number(text: &str) -> Option<f32> {
    text.parse::<f32>().ok().filter(|v| v.is_finite())
}

fn path() -> PathBuf {
    let mut p = relm4::gtk::glib::user_config_dir();
    p.push("vibes");
    p.push("config");
    p
}

impl Config {
    pub fn load() -> Self {
        match std::fs::read_to_string(path()) {
            Ok(text) => Config::parse(&text),
            Err(_) => Config::default(),
        }
    }

    fn parse(text: &str) -> Self {
        let mut cfg = Config::default();
        for line in text.lines() {
            let Some((key, value)) = line.split_once('=') else { continue };
            match (key.trim(), value.trim()) {
                ("a4", v) => cfg.a4 = number(v).unwrap_or(cfg.a4),
                ("sustain", v) => cfg.sustain = number(v).unwrap_or(cfg.sustain),
                ("solfege", v) => cfg.solfege = v == "true",
                _ => {}
            }
        }
        // A corrupt or hand-edited file must not put the UI in an impossible state.
        cfg.a4 = cfg.a4.clamp(A4_RANGE.0, A4_RANGE.1);
        cfg.sustain = cfg.sustain.clamp(SUSTAIN_RANGE.0, SUSTAIN_RANGE.1);
        cfg
    }

    pub fn save(&self) {
        let p = path();
        if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let body = format!("a4={}\nsustain={}\nsolfege={}\n", self.a4, self.sustain, self.solfege);
        if let Err(e) = std::fs::write(&p, body) {
            eprintln!("could not save settings to {}: {e}", p.display());
        }
    }

    pub fn note_name(&self, pitch_class: usize) -> &'static str {
        if self.solfege {
            NOTE_NAMES_SOLFEGE[pitch_class]
        } else {
            NOTE_NAMES_LETTER[pitch_class]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_normal_file() {
        let cfg = Config::parse("a4=442\nsustain=0.8\nsolfege=true\n");
        assert_eq!(cfg.a4, 442.0);
        assert_eq!(cfg.sustain, 0.8);
        assert!(cfg.solfege);
    }

    #[test]
    fn a_corrupt_file_never_yields_an_unusable_setting() {
        // NaN survives f32::clamp, and inf and junk must not get through either.
        for text in ["a4=nan", "a4=inf", "a4=banana", "a4=", "a4=1e9", "nonsense"] {
            let cfg = Config::parse(text);
            assert!(cfg.a4.is_finite(), "{text} produced {}", cfg.a4);
            assert!((A4_RANGE.0..=A4_RANGE.1).contains(&cfg.a4), "{text} produced {}", cfg.a4);
        }
        let cfg = Config::parse("sustain=nan");
        assert!(cfg.sustain.is_finite());
    }

    #[test]
    fn unknown_keys_and_blank_lines_are_ignored() {
        let cfg = Config::parse("\n# a comment\ncolour=blue\na4=415\n");
        assert_eq!(cfg.a4, 415.0);
        assert_eq!(cfg.sustain, SUSTAIN_DEFAULT);
    }
}
