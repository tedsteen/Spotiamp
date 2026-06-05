use audioviz::spectrum::{
    config::{
        Interpolation, PositionNormalisation, ProcessorConfig, StreamConfig, VolumeNormalisation,
    },
    stream::Stream,
};
use librespot::playback::SAMPLE_RATE;

pub struct Visualizer {
    stream: Stream,
}
pub fn stereo_to_mono(in_v: &[f32]) -> Vec<f32> {
    in_v.chunks_exact(2)
        .map(|chunk| (chunk[0] + chunk[1]) / 2.0)
        .collect()
}

impl Visualizer {
    pub fn new() -> Self {
        Self {
            stream: Stream::new(StreamConfig {
                channel_count: 1,
                processor: ProcessorConfig {
                    sampling_rate: SAMPLE_RATE,
                    frequency_bounds: [40, 20000],
                    resolution: Some(19),
                    volume: 0.8,
                    volume_normalisation: VolumeNormalisation::Mixture,
                    position_normalisation: PositionNormalisation::Harmonic,
                    manual_position_distribution: None,
                    interpolation: Interpolation::Cubic,
                },
                fft_resolution: 1024 * 2,
                refresh_rate: 60,
                gravity: Some(2.0),
            }),
        }
    }
    pub fn push_samples(&mut self, samples: &[f32]) {
        self.stream.push_data(stereo_to_mono(samples));
        self.stream.update();
    }

    pub fn take_latest_spectrum(&mut self) -> Vec<(f32, f32)> {
        let freqs = self.stream.get_frequencies();
        freqs
            .first()
            .map(|data| data.iter().map(|d| (d.freq, d.volume)).collect())
            .unwrap_or_default()
    }
}
