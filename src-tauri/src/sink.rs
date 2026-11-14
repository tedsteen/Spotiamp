use std::sync::atomic::{AtomicI16, AtomicU16, Ordering};
use std::sync::{Arc, Mutex};

use librespot::playback::audio_backend::{self, Sink, SinkResult};
use librespot::playback::config::AudioFormat;
use librespot::playback::convert::Converter;
use librespot::playback::decoder::AudioPacket;

use crate::visualizer::Visualizer;

pub struct SpotiampSink {
    backend_delegate: Box<dyn Sink>,
    visualizer: Arc<Mutex<Visualizer>>,
    volume: Arc<AtomicU16>,
    /// -100 (left only) ..= 100 (right only), 0 is centered.
    balance: Arc<AtomicI16>,
    scratch: Vec<f32>,
}

impl SpotiampSink {
    pub fn new(
        file: Option<String>,
        format: AudioFormat,
        visualizer: Arc<Mutex<Visualizer>>,
        volume: Arc<AtomicU16>,
        balance: Arc<AtomicI16>,
    ) -> Self {
        Self {
            backend_delegate: audio_backend::find(None).unwrap()(file, format),
            visualizer,
            volume,
            balance,
            scratch: Vec::new(),
        }
    }
}

/// Winamp-style balance: turn down the channel opposite to the balance
/// (interleaved stereo samples, left first).
fn apply_balance(samples: &mut [f64], balance: i16) {
    if balance == 0 {
        return;
    }
    let toward_right = f64::from(balance.clamp(-100, 100)) / 100.0;
    let left_gain = (1.0 - toward_right).min(1.0);
    let right_gain = (1.0 + toward_right).min(1.0);
    for frame in samples.as_chunks_mut::<2>().0 {
        frame[0] *= left_gain;
        frame[1] *= right_gain;
    }
}

impl Sink for SpotiampSink {
    fn start(&mut self) -> SinkResult<()> {
        self.backend_delegate.start()
    }

    fn stop(&mut self) -> SinkResult<()> {
        self.backend_delegate.stop()
    }

    fn write(&mut self, mut packet: AudioPacket, converter: &mut Converter) -> SinkResult<()> {
        if let Ok(samples) = packet.samples() {
            if samples.len() > self.scratch.len() {
                self.scratch.resize(samples.len().next_power_of_two(), 0.0);
            }
            // Undo the volume attenuation so the visualizer shows the same levels
            // regardless of volume. At volume 0 there is nothing to show.
            let volume = self.volume.load(Ordering::Relaxed);
            if volume > 0 {
                let gain = 100.0 / volume as f32;
                let mut visualizer = self.visualizer.lock().unwrap();
                for (idx, s) in samples.iter().enumerate() {
                    self.scratch[idx] = *s as f32 * gain;
                }
                visualizer.push_samples(&self.scratch[..samples.len()]);
            }
        }

        if let AudioPacket::Samples(samples) = &mut packet {
            apply_balance(samples, self.balance.load(Ordering::Relaxed));
        }
        self.backend_delegate.write(packet, converter)
    }
}

#[cfg(test)]
mod tests {
    use super::apply_balance;

    #[test]
    fn balance_turns_down_the_opposite_channel() {
        let mut samples = [1.0, 1.0, 0.5, 0.5];
        apply_balance(&mut samples, 0);
        assert_eq!(samples, [1.0, 1.0, 0.5, 0.5]);

        let mut samples = [1.0, 1.0, 0.5, 0.5];
        apply_balance(&mut samples, 50);
        assert_eq!(samples, [0.5, 1.0, 0.25, 0.5]);

        let mut samples = [1.0, 1.0];
        apply_balance(&mut samples, -100);
        assert_eq!(samples, [1.0, 0.0]);
    }
}
