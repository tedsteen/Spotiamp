use std::sync::atomic::AtomicU16;
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
    scratch: Vec<f32>,
}

impl SpotiampSink {
    pub fn new(
        file: Option<String>,
        format: AudioFormat,
        visualizer: Arc<Mutex<Visualizer>>,
        volume: Arc<AtomicU16>,
    ) -> Self {
        Self {
            backend_delegate: audio_backend::find(None).unwrap()(file, format),
            visualizer,
            volume,
            scratch: Vec::new(),
        }
    }
}

impl Sink for SpotiampSink {
    fn start(&mut self) -> SinkResult<()> {
        self.backend_delegate.start()
    }

    fn stop(&mut self) -> SinkResult<()> {
        self.backend_delegate.stop()
    }

    fn write(&mut self, packet: AudioPacket, converter: &mut Converter) -> SinkResult<()> {
        if let Ok(samples) = packet.samples() {
            if samples.len() > self.scratch.len() {
                self.scratch.resize(samples.len().next_power_of_two(), 0.0);
            }
            let volume = 100.0 / self.volume.load(std::sync::atomic::Ordering::Relaxed) as f32;
            if volume > 0.0 {
                let mut visualizer = self.visualizer.lock().unwrap();
                for (idx, s) in samples.iter().enumerate() {
                    self.scratch[idx] = *s as f32 * volume;
                }
                visualizer.push_samples(&self.scratch[..samples.len()]);
            }
        }

        self.backend_delegate.write(packet, converter)
    }
}
