//! Bus dynamics of the Godot layout (`audio_buses.gd`): hard limiter on `Master`, compressors on
//! `Ambiance`, `Bataille` and `Voix`, and the side-chain compressor of `Musique` keyed by `Voix`.
//!
//! `bevy_audio` has no effect graph and gives no access to the mixed samples, so the dynamics run
//! at control rate on an estimate of each bus level: the sum of the volumes the live sinks ask
//! for (see [`BusDemand`]), scaled by a crest factor. The computed gain reduction (threshold,
//! ratio, attack and release of the Godot compressors) multiplies the volume of every sink of the
//! bus. It is a faithful gain law applied to an estimated level, not a sample-accurate effect.

use super::Channel;
use bevy::prelude::*;

/// `AudioEffectHardLimiter.ceiling_db` of Master.
pub const LIMITER_CEILING_DB: f32 = -0.5;
/// Programme level of a sink relative to its volume (clips are not full-scale: about -14 dB).
const PROGRAMME_CREST: f32 = 0.2;
/// Peak level of a sink relative to its volume (what the limiter sees).
const PEAK_CREST: f32 = 0.5;
/// Release of the limiter (s).
const LIMITER_RELEASE_S: f32 = 0.15;
/// Control-rate floor on attacks: Godot's microsecond attacks cannot be tracked per frame.
const MIN_ATTACK_S: f32 = 0.004;

/// Threshold / ratio / attack / release of an `AudioEffectCompressor`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CompressorSpec {
    pub threshold_db: f32,
    pub ratio: f32,
    pub attack_s: f32,
    pub release_s: f32,
}

/// `Ambiance`: `_compressor(-14, 3, 20 us, 250 ms)`.
pub const AMBIENCE: CompressorSpec = CompressorSpec::new(-14.0, 3.0, 20.0, 250.0);
/// `Bataille`: `_compressor(-16, 4, 10 us, 180 ms)`.
pub const BATTLE: CompressorSpec = CompressorSpec::new(-16.0, 4.0, 10.0, 180.0);
/// `Voix`: `_compressor(-12, 3, 5 us, 150 ms)`.
pub const VOICE: CompressorSpec = CompressorSpec::new(-12.0, 3.0, 5.0, 150.0);
/// Side-chain of `Musique` (key = `Voix`): `_compressor(-20, 3, 20 us, 400 ms)`.
pub const MUSIC_SIDECHAIN: CompressorSpec = CompressorSpec::new(-20.0, 3.0, 20.0, 400.0);

impl CompressorSpec {
    /// Godot units: attack in microseconds, release in milliseconds.
    pub const fn new(threshold_db: f32, ratio: f32, attack_us: f32, release_ms: f32) -> Self {
        Self {
            threshold_db,
            ratio,
            attack_s: attack_us / 1_000_000.0,
            release_s: release_ms / 1000.0,
        }
    }

    /// Static curve: gain reduction (dB, >= 0) for an input level.
    pub fn reduction_db(&self, level_db: f32) -> f32 {
        if level_db <= self.threshold_db {
            0.0
        } else {
            (level_db - self.threshold_db) * (1.0 - 1.0 / self.ratio.max(1.0))
        }
    }
}

/// A compressor with its smoothed gain reduction.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Compressor {
    pub spec: CompressorSpec,
    reduction_db: f32,
}

impl Compressor {
    pub const fn new(spec: CompressorSpec) -> Self {
        Self {
            spec,
            reduction_db: 0.0,
        }
    }

    /// One control step for an input `level` (linear); returns the linear gain to apply.
    pub fn step(&mut self, level: f32, dt: f32) -> f32 {
        let target = self.spec.reduction_db(to_db(level));
        let time = if target > self.reduction_db {
            self.spec.attack_s.max(MIN_ATTACK_S)
        } else {
            self.spec.release_s
        };
        let blend = 1.0 - (-dt / time.max(1e-4)).exp();
        self.reduction_db += (target - self.reduction_db) * blend;
        from_db(-self.reduction_db)
    }

    pub fn reduction_db(&self) -> f32 {
        self.reduction_db
    }
}

fn to_db(linear: f32) -> f32 {
    20.0 * linear.max(1e-6).log10()
}

fn from_db(db: f32) -> f32 {
    10f32.powf(db / 20.0)
}

/// Gain that keeps a `peak` (linear) under the limiter ceiling; 1 when it already fits.
pub fn limiter_gain(peak: f32, ceiling_db: f32) -> f32 {
    let ceiling = from_db(ceiling_db);
    if peak <= ceiling { 1.0 } else { ceiling / peak }
}

/// Sum of the volumes the sinks of each bus currently ask for, BEFORE the dynamics gain.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct BusDemand {
    pub interface: f32,
    pub voice: f32,
    pub battle: f32,
    pub ambience: f32,
    pub music: f32,
}

impl BusDemand {
    pub(super) fn add(&mut self, channel: Channel, volume: f32) {
        match channel {
            Channel::Sfx => self.interface += volume,
            Channel::Voice => self.voice += volume,
            Channel::Battle => self.battle += volume,
        }
    }
}

/// Current gains of the buses (resource read by the volume setters of the audio systems).
#[derive(Resource, Debug, Clone)]
pub struct BusDynamics {
    voice: Compressor,
    battle: Compressor,
    ambience: Compressor,
    music: Compressor,
    voice_gain: f32,
    battle_gain: f32,
    ambience_gain: f32,
    music_gain: f32,
    master_gain: f32,
    last_demand: BusDemand,
}

impl Default for BusDynamics {
    fn default() -> Self {
        Self {
            voice: Compressor::new(VOICE),
            battle: Compressor::new(BATTLE),
            ambience: Compressor::new(AMBIENCE),
            music: Compressor::new(MUSIC_SIDECHAIN),
            voice_gain: 1.0,
            battle_gain: 1.0,
            ambience_gain: 1.0,
            music_gain: 1.0,
            master_gain: 1.0,
            last_demand: BusDemand::default(),
        }
    }
}

impl BusDynamics {
    /// Advances the compressors and the limiter by `dt` for the current demand.
    pub fn update(&mut self, demand: BusDemand, dt: f32) {
        self.last_demand = demand;
        self.voice_gain = self.voice.step(demand.voice * PROGRAMME_CREST, dt);
        self.battle_gain = self.battle.step(demand.battle * PROGRAMME_CREST, dt);
        self.ambience_gain = self.ambience.step(demand.ambience * PROGRAMME_CREST, dt);
        // Side-chain: the music is compressed by the (compressed) voice bus.
        self.music_gain = self
            .music
            .step(demand.voice * self.voice_gain * PROGRAMME_CREST, dt);
        // Hard limiter on the sum of every bus after its own gain.
        let peak = (demand.interface
            + demand.voice * self.voice_gain
            + demand.battle * self.battle_gain
            + demand.ambience * self.ambience_gain
            + demand.music * self.music_gain)
            * PEAK_CREST;
        let wanted = limiter_gain(peak, LIMITER_CEILING_DB);
        if wanted < self.master_gain {
            self.master_gain = wanted;
        } else {
            let blend = 1.0 - (-dt / LIMITER_RELEASE_S).exp();
            self.master_gain += (wanted - self.master_gain) * blend;
        }
    }

    /// Gain of the sinks of a channel, limiter included.
    pub(super) fn channel_gain(&self, channel: Channel) -> f32 {
        self.master_gain
            * match channel {
                Channel::Sfx => 1.0,
                Channel::Voice => self.voice_gain,
                Channel::Battle => self.battle_gain,
            }
    }

    pub fn ambience_gain(&self) -> f32 {
        self.master_gain * self.ambience_gain
    }

    /// Music gain: side-chain by the voices, limiter included.
    pub fn music_gain(&self) -> f32 {
        self.master_gain * self.music_gain
    }

    pub fn limiter_gain(&self) -> f32 {
        self.master_gain
    }

    pub fn last_demand(&self) -> BusDemand {
        self.last_demand
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settle(dynamics: &mut BusDynamics, demand: BusDemand) {
        for _ in 0..600 {
            dynamics.update(demand, 1.0 / 60.0);
        }
    }

    #[test]
    fn static_curve_follows_threshold_and_ratio() {
        assert_eq!(BATTLE.reduction_db(-30.0), 0.0);
        assert_eq!(BATTLE.reduction_db(-16.0), 0.0);
        // 8 dB over a 4:1 compressor: 6 dB of reduction.
        assert!((BATTLE.reduction_db(-8.0) - 6.0).abs() < 1e-4);
        assert!((VOICE.reduction_db(-6.0) - 4.0).abs() < 1e-4);
    }

    #[test]
    fn quiet_mix_is_untouched() {
        let mut dynamics = BusDynamics::default();
        let demand = BusDemand {
            battle: 0.1,
            voice: 0.1,
            ..BusDemand::default()
        };
        settle(&mut dynamics, demand);
        assert!((dynamics.channel_gain(Channel::Battle) - 1.0).abs() < 1e-3);
        assert!((dynamics.channel_gain(Channel::Voice) - 1.0).abs() < 1e-3);
        assert!((dynamics.limiter_gain() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn loud_battle_is_compressed_with_attack_and_release() {
        let mut dynamics = BusDynamics::default();
        let loud = BusDemand {
            battle: 2.0,
            ..BusDemand::default()
        };
        dynamics.update(loud, 1.0 / 60.0);
        let first = dynamics.channel_gain(Channel::Battle);
        settle(&mut dynamics, loud);
        let settled = dynamics.channel_gain(Channel::Battle);
        assert!(settled < first, "attack ramps down: {first} -> {settled}");
        // 2.0 * 0.2 = -8 dB programme: 6 dB reduction, then the limiter may add more.
        assert!(settled < 0.6 && settled > 0.2, "{settled}");
        // Release: slower than the attack, back to unity after the load goes away.
        dynamics.update(BusDemand::default(), 1.0 / 60.0);
        assert!(dynamics.channel_gain(Channel::Battle) < 0.99);
        settle(&mut dynamics, BusDemand::default());
        assert!((dynamics.channel_gain(Channel::Battle) - 1.0).abs() < 1e-3);
    }

    #[test]
    fn voices_duck_the_music_by_side_chain() {
        let mut dynamics = BusDynamics::default();
        let talking = BusDemand {
            voice: 1.0,
            music: 0.5,
            ..BusDemand::default()
        };
        settle(&mut dynamics, talking);
        assert!(dynamics.music_gain() < 0.8, "{}", dynamics.music_gain());
        let silent = BusDemand {
            music: 0.5,
            ..BusDemand::default()
        };
        settle(&mut dynamics, silent);
        assert!((dynamics.music_gain() - 1.0).abs() < 1e-3);
    }

    #[test]
    fn limiter_holds_the_sum_under_the_ceiling() {
        assert_eq!(limiter_gain(0.5, LIMITER_CEILING_DB), 1.0);
        let gain = limiter_gain(2.0, LIMITER_CEILING_DB);
        assert!((2.0 * gain - from_db(LIMITER_CEILING_DB)).abs() < 1e-5);
        let mut dynamics = BusDynamics::default();
        let flood = BusDemand {
            interface: 4.0,
            ..BusDemand::default()
        };
        dynamics.update(flood, 1.0 / 60.0);
        // Instant attack: the very first step already respects the ceiling.
        assert!(4.0 * PEAK_CREST * dynamics.channel_gain(Channel::Sfx) <= 0.95 + 1e-4);
    }
}
