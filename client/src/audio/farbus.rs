//! Real low-pass and reverb of the far battle bus (`AudioBuses.set_battle_distance`,
//! `game/scripts/audio/audio_buses.gd`): a [`FarSource`] audio asset whose decoder wraps rodio's
//! decoder in a biquad low-pass followed by a small Schroeder reverb (4 combs + 2 all-pass per
//! channel). The settings live in one [`FarParams`] shared by every wrapped sound (the "bus"):
//! the battle audio writes them each frame from the camera height, the decoders read them every
//! few hundred samples, so a sound already playing follows the zoom.
//!
//! Replaces the gain approximations (`muffle_gain`, `distant_bed_gain`) of the first port.
//! `bevy_audio` has no effect graph, hence a wrapped [`Decodable`] rather than a bus.

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use bevy::asset::{Asset, AssetLoader, LoadContext, io::Reader};
use bevy::audio::{
    AddAudioSource, AudioSource, ChannelCount, Decodable, Sample, SampleRate, Source,
};
use bevy::prelude::*;
use bevy::reflect::TypePath;

/// Frames between two reads of the shared settings (about 6 ms at 44.1 kHz).
const CONTROL_PERIOD: u32 = 256;

/// Settings of the far bus, shared between the game thread and the audio thread.
#[derive(Debug)]
pub struct FarParams {
    cutoff_hz: AtomicU32,
    wet: AtomicU32,
    room_size: AtomicU32,
}

impl Default for FarParams {
    fn default() -> Self {
        // Close setting of `far_bus(0.0)`.
        Self::new(7000.0, 0.12, 0.45)
    }
}

impl FarParams {
    pub fn new(cutoff_hz: f32, wet: f32, room_size: f32) -> Self {
        Self {
            cutoff_hz: AtomicU32::new(cutoff_hz.to_bits()),
            wet: AtomicU32::new(wet.to_bits()),
            room_size: AtomicU32::new(room_size.to_bits()),
        }
    }

    pub fn set(&self, cutoff_hz: f32, wet: f32, room_size: f32) {
        self.cutoff_hz.store(cutoff_hz.to_bits(), Ordering::Relaxed);
        self.wet.store(wet.to_bits(), Ordering::Relaxed);
        self.room_size.store(room_size.to_bits(), Ordering::Relaxed);
    }

    pub fn get(&self) -> (f32, f32, f32) {
        (
            f32::from_bits(self.cutoff_hz.load(Ordering::Relaxed)),
            f32::from_bits(self.wet.load(Ordering::Relaxed)),
            f32::from_bits(self.room_size.load(Ordering::Relaxed)),
        )
    }
}

/// Resource handle on the shared settings.
#[derive(Resource, Clone, Default)]
pub struct FarBusParams(pub Arc<FarParams>);

/// Direct-form-I biquad low-pass (RBJ cookbook), one state per channel.
#[derive(Debug, Clone, Copy, Default)]
pub struct Biquad {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
}

impl Biquad {
    /// Low-pass at `cutoff_hz` for `sample_rate`, Butterworth Q.
    pub fn low_pass(cutoff_hz: f32, sample_rate: f32) -> Self {
        let cutoff = cutoff_hz.clamp(20.0, sample_rate * 0.45);
        let w0 = std::f32::consts::TAU * cutoff / sample_rate;
        let alpha = w0.sin() / (2.0 * std::f32::consts::FRAC_1_SQRT_2);
        let cos = w0.cos();
        let a0 = 1.0 + alpha;
        Self {
            b0: (1.0 - cos) / 2.0 / a0,
            b1: (1.0 - cos) / a0,
            b2: (1.0 - cos) / 2.0 / a0,
            a1: -2.0 * cos / a0,
            a2: (1.0 - alpha) / a0,
        }
    }

    /// Magnitude of the response at `freq_hz`.
    pub fn gain_at(&self, freq_hz: f32, sample_rate: f32) -> f32 {
        let w = std::f32::consts::TAU * freq_hz / sample_rate;
        let (c1, s1) = (w.cos(), w.sin());
        let (c2, s2) = ((2.0 * w).cos(), (2.0 * w).sin());
        let num_re = self.b0 + self.b1 * c1 + self.b2 * c2;
        let num_im = -(self.b1 * s1 + self.b2 * s2);
        let den_re = 1.0 + self.a1 * c1 + self.a2 * c2;
        let den_im = -(self.a1 * s1 + self.a2 * s2);
        ((num_re * num_re + num_im * num_im) / (den_re * den_re + den_im * den_im)).sqrt()
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct BiquadState {
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
}

impl BiquadState {
    fn run(&mut self, c: &Biquad, x: f32) -> f32 {
        let y = c.b0 * x + c.b1 * self.x1 + c.b2 * self.x2 - c.a1 * self.y1 - c.a2 * self.y2;
        self.x2 = self.x1;
        self.x1 = x;
        self.y2 = self.y1;
        self.y1 = y;
        y
    }
}

/// Lowpassed feedback comb (Freeverb).
#[derive(Debug, Clone)]
struct Comb {
    buffer: Vec<f32>,
    index: usize,
    store: f32,
}

impl Comb {
    fn new(len: usize) -> Self {
        Self {
            buffer: vec![0.0; len.max(1)],
            index: 0,
            store: 0.0,
        }
    }
    fn run(&mut self, x: f32, feedback: f32, damp: f32) -> f32 {
        let out = self.buffer[self.index];
        self.store = out * (1.0 - damp) + self.store * damp;
        self.buffer[self.index] = x + self.store * feedback;
        self.index = (self.index + 1) % self.buffer.len();
        out
    }
}

#[derive(Debug, Clone)]
struct AllPass {
    buffer: Vec<f32>,
    index: usize,
}

impl AllPass {
    fn new(len: usize) -> Self {
        Self {
            buffer: vec![0.0; len.max(1)],
            index: 0,
        }
    }
    fn run(&mut self, x: f32) -> f32 {
        let delayed = self.buffer[self.index];
        let out = delayed - x;
        self.buffer[self.index] = x + delayed * 0.5;
        self.index = (self.index + 1) % self.buffer.len();
        out
    }
}

/// Freeverb tunings at 44.1 kHz (samples), scaled to the sample rate; `spread` offsets the
/// second channel.
const COMBS: [usize; 4] = [1116, 1188, 1277, 1356];
const ALLPASSES: [usize; 2] = [556, 441];

#[derive(Debug, Clone)]
struct Reverb {
    combs: Vec<Comb>,
    allpasses: Vec<AllPass>,
}

impl Reverb {
    fn new(sample_rate: f32, spread: usize) -> Self {
        let scale = sample_rate / 44100.0;
        let size = |n: usize| ((n + spread) as f32 * scale) as usize;
        Self {
            combs: COMBS.iter().map(|&n| Comb::new(size(n))).collect(),
            allpasses: ALLPASSES.iter().map(|&n| AllPass::new(size(n))).collect(),
        }
    }
    fn run(&mut self, x: f32, feedback: f32) -> f32 {
        let input = x * 0.25;
        let mut sum = 0.0;
        for comb in &mut self.combs {
            sum += comb.run(input, feedback, 0.3);
        }
        for allpass in &mut self.allpasses {
            sum = allpass.run(sum);
        }
        sum
    }
}

/// Low-pass then reverb over any rodio source, parameters read from [`FarParams`].
pub struct FarFilter<S: Source> {
    inner: S,
    params: Arc<FarParams>,
    channels: usize,
    sample_rate: f32,
    channel: usize,
    frames: u32,
    coefficients: Biquad,
    wet: f32,
    feedback: f32,
    filters: Vec<BiquadState>,
    reverbs: Vec<Reverb>,
}

impl<S: Source> FarFilter<S> {
    pub fn new(inner: S, params: Arc<FarParams>) -> Self {
        let channels = inner.channels().get() as usize;
        let sample_rate = inner.sample_rate().get() as f32;
        let mut filter = Self {
            inner,
            params,
            channels,
            sample_rate,
            channel: 0,
            frames: 0,
            coefficients: Biquad::default(),
            wet: 0.0,
            feedback: 0.7,
            filters: vec![BiquadState::default(); channels],
            reverbs: (0..channels)
                .map(|c| Reverb::new(sample_rate, c * 23))
                .collect(),
        };
        filter.refresh();
        filter
    }

    fn refresh(&mut self) {
        let (cutoff, wet, room) = self.params.get();
        self.coefficients = Biquad::low_pass(cutoff, self.sample_rate);
        self.wet = wet.clamp(0.0, 1.0);
        self.feedback = 0.7 + 0.28 * room.clamp(0.0, 1.0);
    }
}

impl<S: Source> Iterator for FarFilter<S> {
    type Item = Sample;

    fn next(&mut self) -> Option<Sample> {
        let x = self.inner.next()?;
        let c = self.channel;
        if c == 0 {
            if self.frames == 0 {
                self.refresh();
            }
            self.frames = (self.frames + 1) % CONTROL_PERIOD;
        }
        self.channel = (c + 1) % self.channels;
        let filtered = self.filters[c].run(&self.coefficients, x);
        let tail = self.reverbs[c].run(filtered, self.feedback);
        Some(filtered + self.wet * tail)
    }
}

impl<S: Source> Source for FarFilter<S> {
    fn current_span_len(&self) -> Option<usize> {
        self.inner.current_span_len()
    }
    fn channels(&self) -> ChannelCount {
        self.inner.channels()
    }
    fn sample_rate(&self) -> SampleRate {
        self.inner.sample_rate()
    }
    fn total_duration(&self) -> Option<Duration> {
        self.inner.total_duration()
    }
}

/// An audio file played through the far bus.
#[derive(Asset, Debug, Clone, TypePath)]
pub struct FarSource {
    bytes: Arc<[u8]>,
    params: Arc<FarParams>,
}

impl Decodable for FarSource {
    type Decoder = FarFilter<<AudioSource as Decodable>::Decoder>;

    fn decoder(&self) -> Self::Decoder {
        let source = AudioSource {
            bytes: self.bytes.clone(),
        };
        FarFilter::new(source.decoder(), self.params.clone())
    }
}

/// Loads the same files as `AudioLoader`, as [`FarSource`] (the type picks the loader).
#[derive(TypePath)]
pub struct FarLoader {
    params: Arc<FarParams>,
}

impl AssetLoader for FarLoader {
    type Asset = FarSource;
    type Settings = ();
    type Error = std::io::Error;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &(),
        _context: &mut LoadContext<'_>,
    ) -> Result<FarSource, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        Ok(FarSource {
            bytes: bytes.into(),
            params: self.params.clone(),
        })
    }

    fn extensions(&self) -> &[&str] {
        &["mp3", "wav", "ogg", "flac"]
    }
}

pub struct FarBusPlugin;

impl Plugin for FarBusPlugin {
    fn build(&self, app: &mut App) {
        let params = FarBusParams::default();
        app.add_audio_source::<FarSource>()
            .register_asset_loader(FarLoader {
                params: params.0.clone(),
            })
            .insert_resource(params);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Mono sine source of `frames` samples.
    struct Sine {
        freq: f32,
        n: usize,
        len: usize,
    }
    impl Iterator for Sine {
        type Item = Sample;
        fn next(&mut self) -> Option<Sample> {
            if self.n >= self.len {
                return None;
            }
            let v = (std::f32::consts::TAU * self.freq * self.n as f32 / 44100.0).sin();
            self.n += 1;
            Some(v)
        }
    }
    impl Source for Sine {
        fn current_span_len(&self) -> Option<usize> {
            None
        }
        fn channels(&self) -> ChannelCount {
            ChannelCount::new(1).unwrap()
        }
        fn sample_rate(&self) -> SampleRate {
            SampleRate::new(44100).unwrap()
        }
        fn total_duration(&self) -> Option<Duration> {
            None
        }
    }

    fn rms(samples: &[f32]) -> f32 {
        (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt()
    }

    fn run(freq: f32, params: FarParams) -> Vec<f32> {
        let source = Sine {
            freq,
            n: 0,
            len: 22050,
        };
        FarFilter::new(source, Arc::new(params)).collect()
    }

    #[test]
    fn biquad_passes_lows_and_cuts_highs() {
        let filter = Biquad::low_pass(1800.0, 44100.0);
        assert!((filter.gain_at(100.0, 44100.0) - 1.0).abs() < 0.01);
        let at_cutoff = filter.gain_at(1800.0, 44100.0);
        assert!(
            (at_cutoff - std::f32::consts::FRAC_1_SQRT_2).abs() < 0.01,
            "-3 dB at cutoff: {at_cutoff}"
        );
        assert!(filter.gain_at(7000.0, 44100.0) < 0.1);
    }

    #[test]
    fn filter_attenuates_a_high_tone_but_not_a_low_one() {
        let dry = FarParams::new(1800.0, 0.0, 0.5);
        let low = rms(&run(200.0, FarParams::new(1800.0, 0.0, 0.5))[4000..]);
        let high = rms(&run(8000.0, dry)[4000..]);
        assert!(
            (low - std::f32::consts::FRAC_1_SQRT_2).abs() < 0.03,
            "low tone kept: {low}"
        );
        assert!(high < 0.05, "high tone cut: {high}");
    }

    #[test]
    fn reverb_adds_a_tail_after_the_source_stops() {
        // 100 ms burst then silence: the tail energy after the burst is nil with wet 0 and
        // clearly present with wet 0.38.
        fn tail(wet: f32) -> f32 {
            let params = Arc::new(FarParams::new(7000.0, wet, 0.8));
            let burst: Vec<f32> = (0..4410)
                .map(|n| (std::f32::consts::TAU * 440.0 * n as f32 / 44100.0).sin())
                .chain(std::iter::repeat_n(0.0, 8820))
                .collect();
            struct Buf(std::vec::IntoIter<f32>);
            impl Iterator for Buf {
                type Item = f32;
                fn next(&mut self) -> Option<f32> {
                    self.0.next()
                }
            }
            impl Source for Buf {
                fn current_span_len(&self) -> Option<usize> {
                    None
                }
                fn channels(&self) -> ChannelCount {
                    ChannelCount::new(1).unwrap()
                }
                fn sample_rate(&self) -> SampleRate {
                    SampleRate::new(44100).unwrap()
                }
                fn total_duration(&self) -> Option<Duration> {
                    None
                }
            }
            let out: Vec<f32> = FarFilter::new(Buf(burst.into_iter()), params).collect();
            rms(&out[6000..])
        }
        assert!(tail(0.0) < 1e-3);
        assert!(tail(0.38) > 0.02);
    }

    #[test]
    fn params_are_read_live() {
        let params = Arc::new(FarParams::default());
        let mut filter = FarFilter::new(
            Sine {
                freq: 8000.0,
                n: 0,
                len: 44100,
            },
            params.clone(),
        );
        let first: Vec<f32> = filter.by_ref().take(3000).collect();
        params.set(500.0, 0.0, 0.5);
        let _: Vec<f32> = filter.by_ref().take(2000).collect();
        let later: Vec<f32> = filter.by_ref().take(2000).collect();
        assert!(rms(&later) < rms(&first[1000..]) * 0.2);
    }
}
