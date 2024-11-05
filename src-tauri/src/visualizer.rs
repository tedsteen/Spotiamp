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
pub fn stereo_to_mono(in_v: Vec<f32>) -> Vec<f32> {
    let new_size = in_v.len() / 2;
    let mut result = Vec::with_capacity(new_size);
    for i in 0..new_size {
        #[allow(clippy::identity_op)]
        let lv = in_v[i * 2 + 0];
        let rv = in_v[i * 2 + 1];
        result.push((lv + rv) / 2.0);
    }
    result
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
    pub fn push_samples(&mut self, samples: Vec<f32>) {
        self.stream.push_data(stereo_to_mono(samples));
        self.stream.update();
    }

    pub fn take_latest_spectrum(&mut self) -> Vec<(f32, f32)> {
        let freqs = self.stream.get_frequencies();
        if freqs.is_empty() {
            return vec![];
        }
        let data = &self.stream.get_frequencies()[0];
        data.iter().map(|d| (d.freq, d.volume)).collect()
    }
}
