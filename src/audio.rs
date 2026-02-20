use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

const TARGET_SAMPLE_RATE: u32 = 16000;

const WAVEFORM_SIZE: usize = 2048;

pub fn record_until_stopped(
    stop: Arc<AtomicBool>,
    waveform_out: Option<Arc<Mutex<Vec<f32>>>>,
) -> Result<Vec<f32>, String> {
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or("no input device found")?;

    // Use the device's default config instead of forcing our own
    let default_config = device
        .default_input_config()
        .map_err(|e| format!("failed to get default input config: {e}"))?;

    let native_rate = default_config.sample_rate().0;
    let native_channels = default_config.channels();

    let config = cpal::StreamConfig {
        channels: native_channels,
        sample_rate: cpal::SampleRate(native_rate),
        buffer_size: cpal::BufferSize::Default,
    };

    let samples: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));
    let samples_clone = samples.clone();

    let stream = device
        .build_input_stream(
            &config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                // Mix down to mono if multi-channel
                let mono: Vec<f32> = if native_channels > 1 {
                    data.chunks(native_channels as usize)
                        .map(|frame| frame.iter().sum::<f32>() / native_channels as f32)
                        .collect()
                } else {
                    data.to_vec()
                };
                samples_clone.lock().unwrap().extend_from_slice(&mono);

                // Feed waveform display
                if let Some(ref wf) = waveform_out {
                    let mut wf = wf.lock().unwrap();
                    wf.extend_from_slice(&mono);
                    if wf.len() > WAVEFORM_SIZE {
                        let excess = wf.len() - WAVEFORM_SIZE;
                        wf.drain(..excess);
                    }
                }
            },
            |err| eprintln!("audio stream error: {err}"),
            None,
        )
        .map_err(|e| format!("failed to build input stream: {e}"))?;

    stream
        .play()
        .map_err(|e| format!("failed to start stream: {e}"))?;

    while !stop.load(Ordering::SeqCst) {
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    drop(stream);

    let raw = samples.lock().unwrap().clone();

    // Resample to 16kHz if needed
    let resampled = if native_rate != TARGET_SAMPLE_RATE {
        resample(&raw, native_rate, TARGET_SAMPLE_RATE)
    } else {
        raw
    };

    Ok(resampled)
}

/// Simple linear interpolation resampler
fn resample(samples: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
    if samples.is_empty() || from_rate == to_rate {
        return samples.to_vec();
    }

    let ratio = from_rate as f64 / to_rate as f64;
    let out_len = (samples.len() as f64 / ratio) as usize;
    let mut out = Vec::with_capacity(out_len);

    for i in 0..out_len {
        let src_idx = i as f64 * ratio;
        let idx = src_idx as usize;
        let frac = src_idx - idx as f64;

        let s = if idx + 1 < samples.len() {
            samples[idx] * (1.0 - frac as f32) + samples[idx + 1] * frac as f32
        } else {
            samples[idx.min(samples.len() - 1)]
        };
        out.push(s);
    }

    out
}

pub fn samples_to_wav(samples: &[f32]) -> Vec<u8> {
    let mut buf = std::io::Cursor::new(Vec::new());
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: TARGET_SAMPLE_RATE,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut writer = hound::WavWriter::new(&mut buf, spec).unwrap();
    for &s in samples {
        let val = (s * i16::MAX as f32).clamp(i16::MIN as f32, i16::MAX as f32) as i16;
        writer.write_sample(val).unwrap();
    }
    writer.finalize().unwrap();

    buf.into_inner()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wav_output_has_valid_header() {
        let samples = vec![0.0f32; 16000];
        let wav = samples_to_wav(&samples);

        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
    }

    #[test]
    fn wav_output_correct_for_empty_input() {
        let samples: Vec<f32> = vec![];
        let wav = samples_to_wav(&samples);

        assert_eq!(&wav[0..4], b"RIFF");
        assert!(wav.len() >= 44);
    }

    #[test]
    fn wav_clamps_extreme_values() {
        let samples = vec![-2.0, 2.0, 0.5, -0.5];
        let wav = samples_to_wav(&samples);

        assert_eq!(&wav[0..4], b"RIFF");
    }

    #[test]
    fn wav_preserves_sample_count() {
        let n = 480;
        let samples = vec![0.1f32; n];
        let wav = samples_to_wav(&samples);

        let reader = hound::WavReader::new(std::io::Cursor::new(wav)).unwrap();
        assert_eq!(reader.spec().channels, 1);
        assert_eq!(reader.spec().sample_rate, TARGET_SAMPLE_RATE);
        assert_eq!(reader.len() as usize, n);
    }

    #[test]
    fn resample_same_rate_is_identity() {
        let input = vec![1.0, 2.0, 3.0, 4.0];
        let output = resample(&input, 16000, 16000);
        assert_eq!(input, output);
    }

    #[test]
    fn resample_downsample_halves_length() {
        let input: Vec<f32> = (0..1000).map(|i| i as f32).collect();
        let output = resample(&input, 48000, 16000);
        // 48kHz -> 16kHz = 1/3 the samples
        let expected_len = (1000.0 / 3.0) as usize;
        assert!((output.len() as i32 - expected_len as i32).abs() <= 1);
    }

    #[test]
    fn resample_empty_input() {
        let output = resample(&[], 48000, 16000);
        assert!(output.is_empty());
    }
}
