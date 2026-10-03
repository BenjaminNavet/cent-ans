//! Pre-battle forecast and withdrawal (lot UB1, pre-battle
//! screen).
//!
//! [`CampaignState::battle_forecast`] estimates, without touching the RNG, the
//! balance of power of a pending battle: it builds both sides exactly as the
//! auto-resolver does (coalitions, garrison, generals, technologies, levy
//! bonuses), applies the same situation modifiers (terrain, river, walls)
//! and computes the effective powers with [`battle_auto::side_power`]. The
//! chance of victory is the probability that the attacker's roll beats the
//! defender's under the auto-resolver's "fortune of war" (a uniform ±10 %
//! jitter on each side), integrated numerically.
//!
//! This is an *estimate* shown to the player: it deliberately reuses only
//! the stable public pieces of the resolver ([`battle_auto::side_power`],
//! [`battle_auto::effective_armor`]) and never changes its rules. If the
//! resolver grows extra phases (lot N1), the forecast stays a simple and
//! readable approximation of them.
//!
//! [`CampaignState::withdraw_pending_battle`] lets the player decline a
//! battle he started: an assault is called off and the siege goes on; an
//! attacking army pulls back and its regiments lose
//! [`WITHDRAW_MORALE_LOSS`] morale. A defender cannot slip away.

use data_model::{GameData, Terrain};
use serde::{Deserialize, Serialize};
use sim_battle::SideId;

use crate::battle_auto::{self, BattleContext, FieldConditions, Side, UnitProfile, Winner};
use crate::battle_request::{is_live, BattleRequestError};
use crate::events::{EventKind, GameEvent};
use crate::movement;
use crate::rng::CampaignRng;
use crate::state::{BattleRequest, CampaignState};

/// Morale lost by every regiment of an army that calls off its attack.
pub const WITHDRAW_MORALE_LOSS: u8 = 10;
/// First seed of the forecast's auto-resolutions (fixed: the forecast is
/// pure and never touches the campaign RNG).
const FORECAST_SEED: u64 = 0xF02_ECA57;

/// Estimated balance of power of a pending battle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BattleForecast {
    /// Effective power of the attackers (all modifiers applied).
    pub attacker_power: f64,
    pub defender_power: f64,
    /// Share of the attackers in the total power, 0-1 (the balance bar).
    pub attacker_share: f64,
    /// Estimated chance that the attackers win an auto-resolved battle, 0-1.
    pub attacker_win_chance: f64,
    /// Head counts, allies included.
    pub attacker_soldiers: u32,
    pub defender_soldiers: u32,
    /// Allied armies joining each side (reinforcements, F1/G1).
    pub attacker_reinforcements: Vec<Reinforcement>,
    pub defender_reinforcements: Vec<Reinforcement>,
    /// Situation modifiers taken into account, in French.
    pub modifiers: Vec<String>,
    /// The player may call the battle off (attacker only).
    pub can_withdraw: bool,
    /// Siege: withdrawing keeps the siege going.
    pub siege: bool,
}

/// An allied army that joins the battle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reinforcement {
    pub army: String,
    pub faction_name: String,
    pub soldiers: u32,
    pub regiments: u32,
    /// Name of its general, empty if none.
    pub general: String,
}

fn head_count(side: &Side) -> u32 {
    side.units.iter().map(|u| u.strength).sum()
}

impl CampaignState {
    /// Forecast of pending battle `index` (see the module docs).
    pub fn battle_forecast(
        &self,
        data: &GameData,
        index: usize,
    ) -> Result<BattleForecast, BattleRequestError> {
        let request = self
            .pending_battles
            .get(index)
            .ok_or(BattleRequestError::UnknownBattle(index))?;
        if !is_live(self, request) {
            return Err(BattleRequestError::Stale(index));
        }
        self.forecast_request(data, request)
            .ok_or(BattleRequestError::Stale(index))
    }

    /// Chance (0-1) that `army` storms the settlement it besieges, computed
    /// exactly like the pre-battle screen of that assault (Q5: the army bar
    /// and the assault dialog used to show two different numbers).
    pub fn assault_win_chance(&self, data: &GameData, army: &crate::state::ArmyId) -> Option<f64> {
        let a = self.armies.get(army)?;
        let place = a.settlement()?.clone();
        let siege = self.settlements.get(&place)?.siege.as_ref()?;
        if siege.attacker != a.faction {
            return None;
        }
        let request = BattleRequest {
            attacker: army.clone(),
            defender: army.clone(),
            province: crate::siege::province_of(self, &place),
            location: place,
            siege: true,
            opening: Default::default(),
        };
        self.forecast_request(data, &request)
            .map(|f| f.attacker_win_chance)
    }

    /// Forecast of any battle request, pending or hypothetical; `None` when
    /// the garrison of an assault is gone.
    fn forecast_request(&self, data: &GameData, request: &BattleRequest) -> Option<BattleForecast> {
        let mut modifiers = Vec::new();
        let mut garrison_is_player = false;
        let mut site = None;
        let mut field_province = None;
        let mut defender_profiles = None;
        let (mut attacker_side, mut defender_side, context, attackers, defenders) = if request.siege
        {
            let attackers = crate::siege::assault_coalition(self, &request.attacker);
            let garrison = crate::siege::garrison_army(self, &request.location)?;
            garrison_is_player = garrison.faction == self.player_faction;
            let walls = crate::siege::walls_stand(self, data, &request.attacker, &request.location);
            let context = BattleContext {
                defender_terrain_bonus: false,
                river_crossing: false,
                walls,
                assault_bonus_percent: self.engine_assault_bonus(data, &request.location),
                wall: self.wall_stand(data, &request.location),
                crossing: None,
            };
            defender_profiles = Some(battle_auto::army_profiles(data, &garrison));
            (
                movement::coalition_side(self, data, &attackers),
                movement::side_from_army(self, data, &garrison),
                context,
                attackers,
                Vec::new(),
            )
        } else {
            let (attackers, defenders) = self.coalitions(data, request);
            let province = self
                .armies
                .get(&request.defender)
                .and_then(|d| self.army_province(data, d))
                .and_then(|p| data.provinces.get(&p));
            site = crate::river_crossing::crossing_site(
                self,
                data,
                &request.attacker,
                &request.defender,
            );
            field_province = province;
            let context = BattleContext {
                defender_terrain_bonus: province.is_some_and(|p| {
                    matches!(
                        p.terrain,
                        Terrain::Hills | Terrain::Forest | Terrain::Mountains
                    )
                }),
                // RC: a real crossing replaces the province river flag.
                river_crossing: site.is_none() && province.is_some_and(|p| !p.rivers.is_empty()),
                walls: false,
                assault_bonus_percent: 0,
                wall: Default::default(),
                crossing: site.as_ref().map(|c| c.effect()),
            };
            (
                movement::coalition_side(self, data, &attackers),
                movement::coalition_side(self, data, &defenders),
                context,
                attackers,
                defenders,
            )
        };
        // CV3: stances (entrenched camp) and the ambush opening, as in the
        // auto-resolver (field battles only).
        if !request.siege {
            for (side, lead, id) in [
                (
                    &mut attacker_side,
                    self.armies.get(&request.attacker),
                    SideId::Attacker,
                ),
                (
                    &mut defender_side,
                    self.armies.get(&request.defender),
                    SideId::Defender,
                ),
            ] {
                let ambusher = request.opening.ambush_victim() == Some(id.other());
                let (charge, defense) = crate::posture::auto_resolve_bonus(data, lead, ambusher);
                side.general_charge_percent += charge;
                side.general_defense_percent += defense;
            }
        }
        // DF1: the AI's morale against the player, as in the auto-resolver.
        let attacker_has_player = self.coalition_has_player(&attackers);
        let defender_has_player = garrison_is_player || self.coalition_has_player(&defenders);
        self.apply_difficulty_morale(
            data,
            &mut attacker_side,
            attacker_has_player,
            &mut defender_side,
            defender_has_player,
        );
        let ai_morale = self.difficulty_modifiers(data).ai_morale_vs_player;
        if ai_morale != 0 && attacker_has_player != defender_has_player {
            modifiers.push(format!(
                "Niveau de difficulté : moral de l'IA {ai_morale:+}"
            ));
        }
        if context.river_crossing {
            modifiers.push("L'assaillant franchit une rivière (−20 %)".to_owned());
        }
        if let Some(site) = &site {
            modifiers.push(crate::river_crossing::forecast_line(data, site));
        }
        if context.walls {
            modifiers.push("Murailles intactes (−30 % à l'assaillant)".to_owned());
            let wall = &context.wall;
            let rules = &data.auto_resolve;
            let factor = rules.wall_attacker_factor(
                wall.level,
                wall.breach_percent,
                wall.engines_ready_percent,
            );
            // Display only: the resolver sampled below applies the factor.
            if factor < 0.995 {
                modifiers.push(format!(
                    "Murailles de niveau {} sans brèche suffisante ({:+.0} % à l'assaillant)",
                    wall.level,
                    (factor - 1.0) * 100.0
                ));
            }
            if context.assault_bonus_percent > 0 {
                modifiers.push(format!(
                    "Porte enfoncée par le bélier (+{} %)",
                    context.assault_bonus_percent
                ));
            }
        }
        if context.defender_terrain_bonus {
            modifiers.push("Le défenseur tient un terrain favorable (+15 %)".to_owned());
        }
        // The chance is the share of auto-resolutions the attackers win:
        // the very resolver of the campaign, on fixed seeds (ADR 0177).
        let attacker_profiles = battle_auto::coalition_profiles(self, data, &attackers);
        let defender_profiles = defender_profiles
            .unwrap_or_else(|| battle_auto::coalition_profiles(self, data, &defenders));
        let checked = |side: &Side, profiles: Vec<UnitProfile>| {
            if profiles.len() == side.units.len() {
                profiles
            } else {
                side.units.iter().map(UnitProfile::infer).collect()
            }
        };
        let attacker_profiles = checked(&attacker_side, attacker_profiles);
        let defender_profiles = checked(&defender_side, defender_profiles);
        let conditions = FieldConditions {
            terrain: field_province.map(|p| p.terrain),
            season: Some(self.season),
            weather: None,
        };
        let samples = data.auto_resolve.forecast_samples.max(1);
        let mut wins = 0u32;
        let (mut attacker_power, mut defender_power) = (0.0, 0.0);
        for seed in 0..samples {
            let result = battle_auto::resolve_with_crossings(
                &attacker_side,
                &attacker_profiles,
                &defender_side,
                &defender_profiles,
                &context,
                &conditions,
                &data.auto_resolve,
                &data.river_crossing_rules,
                &mut CampaignRng::from_seed(FORECAST_SEED + u64::from(seed)),
            );
            wins += u32::from(result.winner == Winner::Attacker);
            attacker_power += result.attacker.power / f64::from(samples);
            defender_power += result.defender.power / f64::from(samples);
        }
        let win_chance = f64::from(wins) / f64::from(samples);
        let player_attacks = attackers.iter().any(|id| {
            self.armies
                .get(id)
                .is_some_and(|a| a.faction == self.player_faction)
        });
        Some(BattleForecast {
            attacker_power,
            defender_power,
            attacker_share: win_chance,
            attacker_win_chance: win_chance,
            attacker_soldiers: head_count(&attacker_side),
            defender_soldiers: head_count(&defender_side),
            attacker_reinforcements: self.reinforcements(data, &attackers),
            defender_reinforcements: self.reinforcements(data, &defenders),
            modifiers,
            can_withdraw: player_attacks
                && self
                    .armies
                    .get(&request.attacker)
                    .is_some_and(|a| a.faction == self.player_faction),
            siege: request.siege,
        })
    }

    /// Allied armies of a coalition, lead army (first) excluded.
    fn reinforcements(&self, data: &GameData, ids: &[crate::state::ArmyId]) -> Vec<Reinforcement> {
        ids.iter()
            .skip(1)
            .filter_map(|id| {
                let army = self.armies.get(id)?;
                Some(Reinforcement {
                    army: id.to_string(),
                    faction_name: data.factions.get(&army.faction).map_or_else(
                        || army.faction.to_string(),
                        |f| f.short_or_display_name().to_owned(),
                    ),
                    soldiers: army.total_strength(),
                    regiments: army.units.len() as u32,
                    general: army
                        .general
                        .as_ref()
                        .map(|g| self.character_name(data, g))
                        .unwrap_or_default(),
                })
            })
            .collect()
    }

    /// The player calls off pending battle `index` (see the module docs).
    /// Returns the journal events.
    pub fn withdraw_pending_battle(
        &mut self,
        data: &GameData,
        index: usize,
    ) -> Result<Vec<GameEvent>, BattleRequestError> {
        let forecast = self.battle_forecast(data, index)?;
        if !forecast.can_withdraw {
            return Err(BattleRequestError::CannotWithdraw(index));
        }
        let request: BattleRequest = self.pending_battles.remove(index);
        let place = crate::siege::settlement_name(data, &request.location);
        let mut event = if request.siege {
            GameEvent::new(
                EventKind::Battle,
                format!("L'assaut de {place} est remis : le siège continue."),
            )
        } else {
            if let Some(army) = self.armies.get_mut(&request.attacker) {
                for unit in &mut army.units {
                    unit.morale = unit.morale.saturating_sub(WITHDRAW_MORALE_LOSS);
                }
            }
            GameEvent::new(
                EventKind::Battle,
                format!(
                    "Notre ost refuse la bataille près de {place} et se replie (moral −{WITHDRAW_MORALE_LOSS})."
                ),
            )
        };
        event = event
            .province(&request.province)
            .army(&request.attacker)
            .faction(&self.player_faction.clone());
        self.events.push(event.clone());
        Ok(vec![event])
    }
}
