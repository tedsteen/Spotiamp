use std::sync::{Arc, Mutex};

use librespot::playback::audio_backend::{self, Sink, SinkResult};
use librespot::playback::config::AudioFormat;
use librespot::playback::convert::Converter;
use librespot::playback::decoder::AudioPacket;

use crate::visualizer::Visualizer;

pub struct SpotiampSink {
    backend_delegate: Box<dyn Sink>,
    visualizer: Arc<Mutex<Visualizer>>,
    volume: Arc<Mutex<u16>>,
}

impl SpotiampSink {
    pub fn new(
        file: Option<String>,
        format: AudioFormat,
        visualizer: Arc<Mutex<Visualizer>>,
        volume: Arc<Mutex<u16>>,
    ) -> Self {
        Self {
            backend_delegate: audio_backend::find(None).unwrap()(file, format),
            visualizer,
            volume,
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
            let volume = *self.volume.lock().unwrap();
            if volume > 0 {
                let mut visualizer = self.visualizer.lock().unwrap();
                let samples = samples
                    .iter()
                    .map(|s| *s as f32 * (100_f32 / volume as f32))
                    .collect();
                visualizer.push_samples(samples);
            }
        }

        self.backend_delegate.write(packet, converter)
    }
}
