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

use crate::battle_auto::{self, BattleContext, Side};
use crate::battle_request::{is_live, BattleRequestError};
use crate::events::{EventKind, GameEvent};
use crate::movement;
use crate::state::{BattleRequest, CampaignState};

/// Morale lost by every regiment of an army that calls off its attack.
pub const WITHDRAW_MORALE_LOSS: u8 = 10;
/// Half-width of the auto-resolver's fortune of war (±10 %).
const FORTUNE: f64 = 0.10;
/// Integration steps of the win chance.
const STEPS: usize = 400;

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

/// Probability that `a × u1 ≥ d × u2` with `u1`, `u2` uniform on
/// `[1 − FORTUNE, 1 + FORTUNE]` (the auto-resolver's rolls).
pub fn win_chance(attacker_power: f64, defender_power: f64) -> f64 {
    if defender_power <= f64::EPSILON {
        return if attacker_power > f64::EPSILON {
            1.0
        } else {
            0.5
        };
    }
    if attacker_power <= f64::EPSILON {
        return 0.0;
    }
    let low = 1.0 - FORTUNE;
    let width = 2.0 * FORTUNE;
    let ratio = attacker_power / defender_power;
    let total: f64 = (0..STEPS)
        .map(|i| {
            let u1 = low + width * (i as f64 + 0.5) / STEPS as f64;
            ((ratio * u1 - low) / width).clamp(0.0, 1.0)
        })
        .sum();
    total / STEPS as f64
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
            };
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
            let context = BattleContext {
                defender_terrain_bonus: province.is_some_and(|p| {
                    matches!(
                        p.terrain,
                        Terrain::Hills | Terrain::Forest | Terrain::Mountains
                    )
                }),
                river_crossing: province.is_some_and(|p| !p.rivers.is_empty()),
                walls: false,
                assault_bonus_percent: 0,
            };
            (
                movement::coalition_side(self, data, &attackers),
                movement::coalition_side(self, data, &defenders),
                context,
                attackers,
                defenders,
            )
        };
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
        let mut attacker_modifier = 1.0;
        if context.river_crossing {
            attacker_modifier *= 0.8;
            modifiers.push("L'assaillant franchit une rivière (−20 %)".to_owned());
        }
        if context.walls {
            attacker_modifier *= 0.7;
            modifiers.push("Murailles intactes (−30 % à l'assaillant)".to_owned());
            if context.assault_bonus_percent > 0 {
                attacker_modifier *= 1.0 + f64::from(context.assault_bonus_percent) / 100.0;
                modifiers.push(format!(
                    "Porte enfoncée par le bélier (+{} %)",
                    context.assault_bonus_percent
                ));
            }
        }
        let defender_modifier = if context.defender_terrain_bonus {
            modifiers.push("Le défenseur tient un terrain favorable (+15 %)".to_owned());
            1.15
        } else {
            1.0
        };
        let attacker_power = battle_auto::side_power(
            &attacker_side,
            battle_auto::effective_armor(&defender_side),
            attacker_modifier,
        );
        let defender_power = battle_auto::side_power(
            &defender_side,
            battle_auto::effective_armor(&attacker_side),
            defender_modifier,
        );
        let total = attacker_power + defender_power;
        let player_attacks = attackers.iter().any(|id| {
            self.armies
                .get(id)
                .is_some_and(|a| a.faction == self.player_faction)
        });
        Some(BattleForecast {
            attacker_power,
            defender_power,
            attacker_share: if total > f64::EPSILON {
                attacker_power / total
            } else {
                0.5
            },
            attacker_win_chance: win_chance(attacker_power, defender_power),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn win_chance_is_symmetric_and_bounded() {
        assert!((win_chance(100.0, 100.0) - 0.5).abs() < 1e-3);
        assert_eq!(win_chance(130.0, 100.0), 1.0);
        assert_eq!(win_chance(70.0, 100.0), 0.0);
        let a = win_chance(105.0, 100.0);
        let b = win_chance(100.0, 105.0);
        assert!((a + b - 1.0).abs() < 1e-3, "{a} + {b}");
        assert!(a > 0.5 && a < 1.0);
        assert_eq!(win_chance(10.0, 0.0), 1.0);
        assert_eq!(win_chance(0.0, 10.0), 0.0);
    }
}
