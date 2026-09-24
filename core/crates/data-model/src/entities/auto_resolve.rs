//! Coefficients of the phased auto-resolve (lot N1), mirroring
//! `data/schemas/auto_resolve_rules.schema.json` (`data/rules/auto_resolve.json`).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::entities::province::Terrain;

/// Chances (per cent) of each weather in one season; they add up to 100.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WeatherChances {
    pub clear: u32,
    pub rain: u32,
    pub fog: u32,
    pub snow: u32,
}

/// Weather draw per season and its effects.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AutoResolveWeather {
    pub spring: WeatherChances,
    pub summer: WeatherChances,
    pub autumn: WeatherChances,
    pub winter: WeatherChances,
    /// Multiplier on the fire of units with `rain_penalty` in the rain.
    pub rain_bow_factor: f64,
    /// Same in the snow.
    pub snow_bow_factor: f64,
    /// Multiplier on every shot in the fog (shorter ranges: fewer volleys).
    pub fog_ranged_factor: f64,
    /// Multiplier on charges in the rain or snow (mud).
    pub wet_charge_factor: f64,
}

/// Effects of the battlefield terrain.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TerrainEffects {
    /// Multiplier on charges.
    pub charge: f64,
    /// Multiplier on shooting.
    pub ranged: f64,
    /// Multiplier on everything the defender deals (high ground, cover).
    pub defender: f64,
}

impl Default for TerrainEffects {
    fn default() -> Self {
        TerrainEffects {
            charge: 1.0,
            ranged: 1.0,
            defender: 1.0,
        }
    }
}

/// Contents of `data/rules/auto_resolve.json`: a battle is fought in
/// phases (volleys, charge, melee rounds) between unit families; kills
/// are `men / 100 × stat × lethality`, reduced by the target's armour.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AutoResolveRules {
    /// Volleys shot before the lines meet.
    pub volleys: u32,
    /// Rounds of melee after the charge.
    pub melee_rounds: u32,
    /// Kills per 100 men and point of `ranged`, per volley.
    pub ranged_lethality: f64,
    /// Kills per 100 men and point of `melee`, per round.
    pub melee_lethality: f64,
    /// Kills per 100 men and point of `charge`.
    pub charge_lethality: f64,
    /// Share of the damage stopped by 100 points of armour, per phase.
    pub armor_vs_ranged: f64,
    pub armor_vs_melee: f64,
    pub armor_vs_charge: f64,
    /// Share of their volley fire shooters keep during melee rounds.
    pub ranged_in_melee: f64,
    /// Exposure of shooters to enemy foot while their own front line
    /// holds (1 = hit like anyone else).
    pub screened_exposure: f64,
    /// Share of the side's men the front line must keep to screen.
    pub screen_share: f64,
    /// Exposure of shooters to charging and fighting cavalry (flanks).
    pub cavalry_flank_exposure: f64,
    /// Exposure of skirmishers (`skirmish`) to shots and blows.
    pub skirmish_exposure: f64,
    /// Multiplier on a charge received by pikes (`pike_square`).
    pub pike_charge_factor: f64,
    /// Kills inflicted back on the riders, per point of charge received
    /// by pikes.
    pub pike_reflect: f64,
    /// Multiplier on a charge received by defending archers with `stakes`.
    pub stakes_charge_factor: f64,
    /// Multiplier on melee blows between pikes and cavalry: pikes strike
    /// riders harder by this factor, riders strike pikes softer by it.
    pub pike_vs_cavalry: f64,
    /// Multiplier on shots received by mounted units (they close fast).
    pub mounted_target_ranged_factor: f64,
    /// Multiplier on shots received by units with a `pavise`.
    pub pavise_factor: f64,
    /// Multiplier on the melee of cavalry after the charge (horsemen
    /// fight better mounted than they are counted on foot).
    pub cavalry_melee_factor: f64,
    /// Morale points lost per per cent of the side's men lost in a phase.
    pub morale_per_loss_percent: f64,
    /// A side breaks when its morale falls below this (0-100).
    pub break_morale: f64,
    /// Share of its men the loser loses in the rout, plus...
    pub pursuit_base: f64,
    /// ... this share per winner horseman per loser man.
    pub pursuit_per_cavalry: f64,
    /// Caps on the share of men lost (winner, loser).
    pub winner_max_losses: f64,
    pub loser_max_losses: f64,
    /// Floor on the share of men the loser loses.
    pub loser_min_losses: f64,
    /// Fortune of war: every phase's damage is scaled by `1 ± jitter`.
    pub jitter: f64,
    /// Multiplier on the attacker crossing a river.
    pub river_attacker: f64,
    /// Multiplier on the attacker storming walls.
    pub walls_attacker: f64,
    /// Multiplier on the defender's shooting from walls.
    pub walls_defender_ranged: f64,
    /// Defender bonus of the legacy `defender_terrain_bonus` flag, used when
    /// the terrain is unknown.
    pub legacy_defender_bonus: f64,
    pub weather: AutoResolveWeather,
    /// Terrain effects; a terrain left out has no effect.
    #[serde(default)]
    pub terrain: BTreeMap<Terrain, TerrainEffects>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl AutoResolveRules {
    /// Effects of `terrain` (neutral when not listed).
    pub fn terrain_effects(&self, terrain: Terrain) -> TerrainEffects {
        self.terrain.get(&terrain).copied().unwrap_or_default()
    }
}

impl Default for AutoResolveRules {
    /// Fallback when `data/rules/auto_resolve.json` is absent; kept equal
    /// to that file (checked by `tests/real_data.rs`).
    fn default() -> Self {
        let chances = |clear, rain, fog, snow| WeatherChances {
            clear,
            rain,
            fog,
            snow,
        };
        let effects = |charge, ranged, defender| TerrainEffects {
            charge,
            ranged,
            defender,
        };
        AutoResolveRules {
            volleys: 3,
            melee_rounds: 4,
            ranged_lethality: 0.12,
            melee_lethality: 0.12,
            charge_lethality: 0.25,
            armor_vs_ranged: 0.8,
            armor_vs_melee: 0.9,
            armor_vs_charge: 0.6,
            ranged_in_melee: 0.5,
            screened_exposure: 0.3,
            screen_share: 0.25,
            cavalry_flank_exposure: 1.5,
            skirmish_exposure: 0.5,
            pike_charge_factor: 0.25,
            pike_reflect: 1.0,
            stakes_charge_factor: 0.6,
            pike_vs_cavalry: 3.0,
            mounted_target_ranged_factor: 0.5,
            pavise_factor: 0.7,
            cavalry_melee_factor: 1.0,
            morale_per_loss_percent: 1.0,
            break_morale: 25.0,
            pursuit_base: 0.05,
            pursuit_per_cavalry: 0.2,
            winner_max_losses: 0.35,
            loser_max_losses: 0.7,
            loser_min_losses: 0.1,
            jitter: 0.15,
            river_attacker: 0.8,
            walls_attacker: 0.7,
            walls_defender_ranged: 1.3,
            legacy_defender_bonus: 1.15,
            weather: AutoResolveWeather {
                spring: chances(60, 30, 10, 0),
                summer: chances(80, 15, 5, 0),
                autumn: chances(45, 35, 20, 0),
                winter: chances(35, 20, 15, 30),
                rain_bow_factor: 0.6,
                snow_bow_factor: 0.8,
                fog_ranged_factor: 0.7,
                wet_charge_factor: 0.7,
            },
            terrain: [
                (Terrain::Hills, effects(0.8, 1.0, 1.15)),
                (Terrain::Mountains, effects(0.5, 0.9, 1.25)),
                (Terrain::Forest, effects(0.5, 0.7, 1.15)),
                (Terrain::Marsh, effects(0.5, 1.0, 1.1)),
                (Terrain::Bocage, effects(0.7, 0.85, 1.05)),
            ]
            .into_iter()
            .collect(),
            description: None,
        }
    }
}
