//! Microphone capture + the detection loop: publishes the fundamental in Hz, or -1.0 when
//! nothing confident is heard.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{mpsc, Arc};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use crate::pitch::yin_pitch;

/// ~93 ms at 44.1 kHz — enough resolution down to low bass.
const WINDOW: usize = 4096;

pub struct Engine {
    // Dropping the stream stops the capture; cpal's Stream is !Send so it lives on the UI thread.
    _stream: cpal::Stream,
    hold_ms: Arc<AtomicU32>,
}

impl Engine {
    /// Grace-hold — how long a note is chased after the string fades. Live-adjustable.
    pub fn set_hold_seconds(&self, seconds: f32) {
        self.hold_ms.store((seconds * 1000.0) as u32, Ordering::Relaxed);
    }
}

/// Opens the default input device and reports on every window from a worker thread:
/// the fundamental in Hz (or -1.0), and the input level as 0.0..=1.0.
pub fn start(on_reading: impl Fn(f32, f32) + Send + 'static) -> Result<Engine, String> {
    let device = cpal::default_host()
        .default_input_device()
        .ok_or("no microphone found")?;
    let supported = device
        .default_input_config()
        .map_err(|e| format!("no usable microphone config: {e}"))?;
    let sample_rate = supported.sample_rate() as usize;
    let channels = supported.channels() as usize;
    let stream_config: cpal::StreamConfig = supported.config();

    // The audio callback must never block, so it only hands samples off; YIN runs on the worker.
    let (tx, rx) = mpsc::channel::<Vec<f32>>();
    let err_fn = |e| eprintln!("audio stream error: {e}");
    let stream = match supported.sample_format() {
        cpal::SampleFormat::F32 => device.build_input_stream(
            stream_config.clone(),
            move |data: &[f32], _: &_| {
                let _ = tx.send(data.iter().step_by(channels).copied().collect());
            },
            err_fn,
            None,
        ),
        cpal::SampleFormat::I16 => device.build_input_stream(
            stream_config.clone(),
            move |data: &[i16], _: &_| {
                let _ = tx.send(
                    data.iter()
                        .step_by(channels)
                        .map(|v| *v as f32 / 32768.0)
                        .collect(),
                );
            },
            err_fn,
            None,
        ),
        other => return Err(format!("unsupported sample format: {other}")),
    }
    .map_err(|e| format!("could not open the microphone: {e}"))?;
    stream.play().map_err(|e| e.to_string())?;

    let hold_ms = Arc::new(AtomicU32::new(1200));
    let hold = hold_ms.clone();
    std::thread::spawn(move || {
        // Median smoothing rejects single-frame outliers (octave slips, transient noise).
        // A grace hold keeps chasing a note as a plucked string decays below the gate, only
        // reverting to "listening" after the configured silence.
        let mut buf: Vec<f32> = Vec::with_capacity(WINDOW * 2);
        let mut history: Vec<f32> = Vec::with_capacity(5);
        let mut silent = 0u32;
        let mut reported = -1.0f32;
        while let Ok(chunk) = rx.recv() {
            buf.extend_from_slice(&chunk);
            while buf.len() >= WINDOW {
                let window: Vec<f32> = buf.drain(..WINDOW).collect();
                let p = yin_pitch(&window, sample_rate, 0.15);
                if p > 0.0 {
                    silent = 0;
                    let mut pc = p;
                    if let Some(med) = median(&history) {
                        // Fold obvious octave slips toward the running estimate.
                        if (pc - 2.0 * med).abs() < 0.04 * 2.0 * med {
                            pc /= 2.0;
                        } else if (pc - 0.5 * med).abs() < 0.04 * 0.5 * med {
                            pc *= 2.0;
                        }
                    }
                    history.push(pc);
                    if history.len() > 5 {
                        history.remove(0);
                    }
                    reported = median(&history).unwrap_or(pc);
                } else {
                    silent += 1;
                    let hold_s = hold.load(Ordering::Relaxed) as f32 / 1000.0;
                    let hold_frames = ((hold_s * sample_rate as f32 / WINDOW as f32) as u32).max(3);
                    if silent >= hold_frames {
                        history.clear();
                        reported = -1.0;
                    }
                    // else: hold the last note through the decay tail / a brief dropout.
                }
                on_reading(reported, level(&window));
            }
        }
    });

    Ok(Engine { _stream: stream, hold_ms })
}

/// Input level as 0.0..=1.0. RMS spans several orders of magnitude between a whisper and a
/// struck string, so the meter reads in dB: -58 dBFS is the floor, -8 dBFS is full.
fn level(window: &[f32]) -> f32 {
    let rms = (window.iter().map(|v| v * v).sum::<f32>() / window.len() as f32).sqrt();
    if rms <= 1e-6 {
        return 0.0;
    }
    ((20.0 * rms.log10() + 58.0) / 50.0).clamp(0.0, 1.0)
}

fn median(values: &[f32]) -> Option<f32> {
    if values.is_empty() {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.total_cmp(b));
    Some(sorted[sorted.len() / 2])
}
