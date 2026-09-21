//! Pure pitch DSP + note math. No GTK here, so it runs under plain `cargo test`.

/// Chromatic pitch classes, index 0 = C. Two naming conventions.
pub const NOTE_NAMES_LETTER: [&str; 12] =
    ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
pub const NOTE_NAMES_SOLFEGE: [&str; 12] =
    ["Do", "Do#", "Re", "Re#", "Mi", "Fa", "Fa#", "Sol", "Sol#", "La", "La#", "Si"];

pub struct NoteInfo {
    pub pitch_class: usize,
    /// Scientific pitch notation octave.
    pub octave: i32,
    /// Deviation from the nearest note, -50..+50.
    pub cents: f32,
}

/// Map a frequency (Hz) to the nearest note given the A4 reference (e.g. 440).
pub fn note_info(freq: f32, a4: f32) -> NoteInfo {
    let midi = 69.0 + 12.0 * (freq as f64 / a4 as f64).log2();
    let nearest = midi.round() as i32;
    NoteInfo {
        pitch_class: nearest.rem_euclid(12) as usize,
        // div_euclid, not `/`: `/` truncates toward zero while pitch_class uses
        // rem_euclid, and the two disagree below MIDI 0.
        octave: nearest.div_euclid(12) - 1,
        cents: ((midi - nearest as f64) * 100.0) as f32,
    }
}

/// YIN pitch detector. Returns the fundamental in Hz, or `None` if no confident pitch.
/// Reference: de Cheveigné & Kawahara (2002).
/// ponytail: O(N^2) difference function — fine for N <= 4096 off the audio thread; swap to an
/// FFT autocorrelation only if a profiler shows it hot.
pub fn yin_pitch(samples: &[f32], sample_rate: usize, threshold: f32) -> Option<f32> {
    let tau_max = samples.len() / 2;
    if tau_max < 2 {
        return None;
    }

    // Remove DC offset (mic bias / low rumble) so quiet signals aren't swamped.
    let mean = (samples.iter().map(|s| *s as f64).sum::<f64>() / samples.len() as f64) as f32;
    let x: Vec<f32> = samples.iter().map(|s| s - mean).collect();

    // Low gate: only skip true near-silence. YIN's clarity check (threshold below) rejects
    // broadband noise regardless of level, so a low gate keeps quiet notes usable.
    let energy: f64 = x.iter().map(|v| (*v as f64) * (*v as f64)).sum();
    if (energy / x.len() as f64) < 1e-7 {
        return None;
    }

    let mut diff = vec![0f32; tau_max];
    for tau in 1..tau_max {
        // Zipped rather than indexed: same arithmetic, without a bounds check per sample in the
        // one loop that runs tau_max^2 times.
        diff[tau] =
            x[..tau_max].iter().zip(&x[tau..tau + tau_max]).map(|(a, b)| (a - b) * (a - b)).sum();
    }

    // Cumulative mean normalized difference.
    let mut cmnd = vec![1f32; tau_max];
    let mut running = 0f32;
    for tau in 1..tau_max {
        running += diff[tau];
        cmnd[tau] = if running == 0.0 { 1.0 } else { diff[tau] * tau as f32 / running };
    }

    // First tau below the threshold that is a local minimum.
    let mut tau_estimate = None;
    let mut tau = 2;
    while tau < tau_max - 1 {
        if cmnd[tau] < threshold {
            while tau + 1 < tau_max && cmnd[tau + 1] < cmnd[tau] {
                tau += 1;
            }
            tau_estimate = Some(tau);
            break;
        }
        tau += 1;
    }
    let t = tau_estimate?;

    // Parabolic interpolation around the minimum for sub-sample accuracy. The search above
    // starts at 2 and stops before tau_max - 1, so both neighbours always exist.
    let (s0, s1, s2) = (cmnd[t - 1], cmnd[t], cmnd[t + 1]);
    let denom = 2.0 * (2.0 * s1 - s2 - s0);
    let better_tau = if denom == 0.0 { t as f32 } else { t as f32 + (s2 - s0) / denom };

    (better_tau > 0.0).then(|| sample_rate as f32 / better_tau)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    fn sine(freq: f64, sample_rate: usize, n: usize) -> Vec<f32> {
        (0..n).map(|i| (2.0 * PI * freq * i as f64 / sample_rate as f64).sin() as f32).collect()
    }

    #[test]
    fn note_info_a440_is_a4_zero_cents() {
        let info = note_info(440.0, 440.0);
        assert_eq!(info.pitch_class, 9); // A
        assert_eq!(info.octave, 4);
        assert!(info.cents.abs() < 0.01);
        assert_eq!(NOTE_NAMES_LETTER[info.pitch_class], "A");
        assert_eq!(NOTE_NAMES_SOLFEGE[info.pitch_class], "La");
    }

    #[test]
    fn note_info_middle_c() {
        let info = note_info(261.63, 440.0);
        assert_eq!(info.pitch_class, 0); // C
        assert_eq!(info.octave, 4);
        assert!(info.cents.abs() < 1.0);
    }

    #[test]
    fn note_info_slightly_sharp_reads_positive_cents() {
        let info = note_info(440.0 * 1.0116, 440.0); // A4 raised ~20 cents
        assert_eq!(info.pitch_class, 9);
        assert!((15.0..=25.0).contains(&info.cents), "got {}", info.cents);
    }

    #[test]
    fn yin_detects_440() {
        let sr = 44100;
        let f = yin_pitch(&sine(440.0, sr, 4096), sr, 0.15).expect("440 Hz sine");
        assert!((f - 440.0).abs() < 2.0, "got {f}");
    }

    #[test]
    fn yin_detects_low_e82() {
        let sr = 44100;
        // low E, guitar 6th string
        let f = yin_pitch(&sine(82.41, sr, 4096), sr, 0.15).expect("82.41 Hz sine");
        assert!((f - 82.41).abs() < 2.0, "got {f}");
    }

    #[test]
    fn yin_hears_nothing_in_silence() {
        assert_eq!(yin_pitch(&vec![0f32; 4096], 44100, 0.15), None);
        assert_eq!(yin_pitch(&[], 44100, 0.15), None);
    }
}
