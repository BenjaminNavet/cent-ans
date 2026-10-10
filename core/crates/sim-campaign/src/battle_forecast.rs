//! Pre-battle forecast and withdrawal (pre-battle screen).
//!
//! [`CampaignState::battle_forecast`] estimates, without touching the
//! campaign RNG, the balance of power of a pending battle. The sides and
//! the situation (coalitions, garrison, generals, technologies, levy
//! bonuses, stances, ambush, difficulty, terrain, river, walls) are built by
//! the very functions the auto-resolver fights with, and the chance of
//! victory is the share of wins of the auto-resolver itself over
//! `forecast_samples` resolutions on private generators ([`forecast_sides`]).
//!
//! [`CampaignState::withdraw_pending_battle`] lets the player decline a
//! battle he started: an assault is called off and the siege goes on; an
//! attacking army pulls back and its regiments lose
//! [`WITHDRAW_MORALE_LOSS`] morale. A defender cannot slip away.

use data_model::{AutoResolveRules, GameData, RiverCrossingRules};
use serde::{Deserialize, Serialize};

use crate::battle_auto::{self, BattleContext, FieldConditions, Side, UnitProfile, Winner};
use crate::battle_request::{is_live, BattleRequestError};
use crate::events::{EventKind, GameEvent};
use crate::movement;
use crate::rng::CampaignRng;
use crate::state::{BattleRequest, CampaignState};

/// Morale lost by every regiment of an army that calls off its attack.
pub const WITHDRAW_MORALE_LOSS: u8 = 10;
/// Estimated balance of power of a pending battle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BattleForecast {
    /// Effective power of the attackers (all modifiers applied).
    pub attacker_power: f64,
    pub defender_power: f64,
    /// Balance bar, 0-1: the attackers' chance of winning (A6-L1, ADR 0181: bar and verdict share one probability).
    pub attacker_share: f64,
    /// Estimated chance that the attackers win an auto-resolved battle, 0-1.
    pub attacker_win_chance: f64,
    /// Mean casualties of each side over the simulated battles, per cent of
    /// its head count (WH uicards top5).
    #[serde(default)]
    pub attacker_losses_pct: f64,
    #[serde(default)]
    pub defender_losses_pct: f64,
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
    /// ADR 0272/WH armya: winning this field battle lifts the siege of
    /// `siege_place` (the beaten side is the besieging army).
    #[serde(default)]
    pub lifts_siege: bool,
    /// Name of the besieged place when `lifts_siege`.
    #[serde(default)]
    pub siege_place: String,
}

/// An allied army that joins the battle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Reinforcement {
    pub army: String,
    pub faction_name: String,
    pub soldiers: u32,
    pub regiments: u32,
    /// Name of its general, empty if none.
    pub general: String,
    /// ADR 0273: distance to the lead army, km.
    #[serde(default)]
    pub distance_km: f64,
    /// ADR 0273: joins from beyond the engagement radius (marches in with
    /// the movement it has left).
    #[serde(default)]
    pub late: bool,
}

/// Odds of a battle between two given sides (LR-13).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SideOdds {
    /// Share of the attacker's wins over the simulated auto-resolutions, 0-1.
    pub win_chance: f64,
    /// Kills each side would deal over a whole battle, before fortune and
    /// morale (the resolver's own pre-battle estimate).
    pub attacker_power: f64,
    pub defender_power: f64,
    /// Mean casualties of each side, per cent of its head count (0-100).
    pub attacker_losses_pct: f64,
    pub defender_losses_pct: f64,
}

/// Odds of `attacker` against `defender` under the auto-resolver itself:
/// `forecast_samples` resolutions on private generators seeded from `seed`
/// (the campaign RNG is never touched, the result is stable for a seed).
pub fn forecast_sides(
    attacker: (&Side, &[UnitProfile]),
    defender: (&Side, &[UnitProfile]),
    context: &BattleContext,
    conditions: &FieldConditions,
    rules: &AutoResolveRules,
    crossing_rules: &RiverCrossingRules,
    seed: u64,
) -> SideOdds {
    let matchup = battle_auto::Matchup::prepare(
        attacker,
        defender,
        context,
        conditions,
        rules,
        crossing_rules,
    );
    let (mut wins, mut attacker_power, mut defender_power) = (0u64, 0.0, 0.0);
    let (mut attacker_losses, mut defender_losses) = (0.0, 0.0);
    let samples = u64::from(rules.forecast_samples.max(1));
    for run in 0..samples {
        let mut rng = CampaignRng::from_seed(seed.wrapping_add(run));
        let result = matchup.play(&mut rng);
        if result.winner == Winner::Attacker {
            wins += 1;
        }
        attacker_power += result.attacker.power;
        defender_power += result.defender.power;
        attacker_losses += f64::from(result.attacker.total_losses);
        defender_losses += f64::from(result.defender.total_losses);
    }
    let runs = samples as f64;
    let loss_pct = |losses: f64, side: &Side| {
        let men = f64::from(head_count(side)).max(1.0);
        (losses / runs / men * 100.0).clamp(0.0, 100.0)
    };
    SideOdds {
        win_chance: wins as f64 / runs,
        attacker_power: attacker_power / runs,
        defender_power: defender_power / runs,
        attacker_losses_pct: loss_pct(attacker_losses, attacker.0),
        defender_losses_pct: loss_pct(defender_losses, defender.0),
    }
}

/// Seed of the forecast's private generators: the odds of a given battle do
/// not flicker from one call to the next.
const FORECAST_SEED: u64 = 0xF02_ECA57;

/// The resolver's own fallback: guessed profiles when the list does not
/// match the side (an army vanished between the two lookups).
fn checked_profiles(profiles: Vec<UnitProfile>, side: &Side) -> Vec<UnitProfile> {
    if profiles.len() == side.units.len() {
        profiles
    } else {
        side.units.iter().map(UnitProfile::infer).collect()
    }
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
            sortie: false,
            opening: Default::default(),
        };
        self.forecast_request(data, &request)
            .map(|f| f.attacker_win_chance)
    }

    /// Forecast of any battle request, pending or hypothetical; `None` when
    /// the garrison of an assault is gone. The sides are built by the very
    /// functions the auto-resolver uses ([`movement::field_battle_setup`],
    /// [`crate::siege::assault_setup`]) and the odds come from the resolver
    /// itself ([`forecast_sides`], LR-13).
    fn forecast_request(&self, data: &GameData, request: &BattleRequest) -> Option<BattleForecast> {
        let rules = &data.auto_resolve;
        let mut modifiers = Vec::new();
        let (attacker_side, defender_side, attacker_profiles, defender_profiles, context);
        let (attackers, defenders, terrain, defender_has_player);
        if request.sortie {
            // WR sortie (ADR 0306): the garrison attacks the besiegers in
            // the open; the same sides as the auto-resolved sortie.
            let setup =
                crate::siege::sortie_setup(self, data, &request.location, &request.attacker)?;
            defender_has_player = self.coalition_has_player(&setup.targets);
            attackers = Vec::new();
            defenders = setup.targets;
            terrain = None;
            context = BattleContext::default();
            attacker_profiles = checked_profiles(setup.sallying_profiles, &setup.sallying);
            defender_profiles = checked_profiles(setup.besieging_profiles, &setup.besieging);
            attacker_side = setup.sallying;
            defender_side = setup.besieging;
            modifiers.push(format!(
                "Sortie de la garnison de {} contre les assiégeants",
                data.settlement_name(&request.location)
            ));
        } else if request.siege {
            let setup = crate::siege::assault_setup(self, data, &request.attacker)?;
            defender_has_player = setup.garrison_is_player;
            attackers = setup.attackers;
            defenders = Vec::new();
            terrain = None;
            context = setup.context;
            attacker_profiles = setup.attacker_profiles;
            defender_profiles = setup.defender_profiles;
            attacker_side = setup.attacker_side;
            defender_side = setup.defender_side;
        } else {
            let setup = movement::field_battle_setup(
                self,
                data,
                &request.attacker,
                &request.defender,
                request.opening,
            )?;
            defender_has_player = self.coalition_has_player(&setup.defenders);
            attacker_profiles = checked_profiles(
                battle_auto::coalition_profiles(self, data, &setup.attackers),
                &setup.attacker_side,
            );
            defender_profiles = checked_profiles(
                battle_auto::coalition_profiles(self, data, &setup.defenders),
                &setup.defender_side,
            );
            if let Some(site) = &setup.crossing {
                modifiers.push(crate::river_crossing::forecast_line(data, site));
            }
            for (lead, label) in [
                (&request.attacker, "L'assaillant"),
                (&request.defender, "Le défenseur"),
            ] {
                let entrenched = self
                    .armies
                    .get(lead)
                    .is_some_and(|a| a.stance == crate::state::Stance::Entrenched);
                if entrenched {
                    modifiers.push(format!("{label} est retranché"));
                }
            }
            if request.opening.ambush_victim().is_some() {
                modifiers.push("Embuscade : l'embusqué charge plus fort".to_owned());
            }
            terrain = setup.province.map(|p| p.terrain);
            context = setup.context;
            attackers = setup.attackers;
            defenders = setup.defenders;
            attacker_side = setup.attacker_side;
            defender_side = setup.defender_side;
        }
        let attacker_has_player = if request.sortie {
            self.settlements
                .get(&request.location)
                .is_some_and(|s| s.controller == self.player_faction)
        } else {
            self.coalition_has_player(&attackers)
        };
        let ai_morale = self.difficulty_modifiers(data).ai_morale_vs_player;
        if ai_morale != 0 && attacker_has_player != defender_has_player {
            modifiers.push(format!(
                "Niveau de difficulté : moral de l'IA {ai_morale:+}"
            ));
        }
        let percent = |factor: f64| ((factor - 1.0) * 100.0).round() as i64;
        if context.river_crossing {
            modifiers.push(format!(
                "L'assaillant franchit une rivière ({:+} %)",
                percent(rules.river_attacker)
            ));
        }
        if context.walls {
            modifiers.push(format!(
                "Murailles intactes ({:+} % à l'assaillant, {:+} % au tir du défenseur)",
                percent(rules.walls_attacker),
                percent(rules.walls_defender_ranged)
            ));
            let wall = &context.wall;
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
        if let Some(effects) = terrain.map(|t| rules.terrain_effects(t)) {
            let defender = percent(effects.defender);
            if defender > 0 {
                modifiers.push(format!(
                    "Le défenseur tient un terrain favorable (+{defender} %)"
                ));
            } else if defender < 0 {
                modifiers.push(format!("Le terrain dessert le défenseur ({defender} %)"));
            }
            let charge = percent(effects.charge);
            if charge < 0 {
                modifiers.push(format!("Terrain malaisé : charges {charge} %"));
            }
        }
        let conditions = FieldConditions {
            terrain,
            season: Some(self.season),
            weather: None,
        };
        let odds = forecast_sides(
            (&attacker_side, &attacker_profiles),
            (&defender_side, &defender_profiles),
            &context,
            &conditions,
            rules,
            &data.river_crossing_rules,
            FORECAST_SEED,
        );
        let player_attacks = attacker_has_player
            && (request.sortie
                || attackers.iter().any(|id| {
                    self.armies
                        .get(id)
                        .is_some_and(|a| a.faction == self.player_faction)
                }));
        let (lifts_siege, siege_place) = if request.siege {
            (false, String::new())
        } else if request.sortie {
            (true, data.settlement_name(&request.location))
        } else {
            self.siege_lifted_by(data, &request.attacker, &request.defender)
        };
        if lifts_siege {
            modifiers.push(format!(
                "Vaincre les assiégeants lève le siège de {siege_place}"
            ));
        }
        let late = [&attackers, &defenders]
            .iter()
            .flat_map(|ids| self.reinforcements(data, ids))
            .filter(|r| r.late)
            .count();
        if late > 0 {
            modifiers.push(format!(
                "{late} armée(s) alliée(s) rejoignent de loin avec leur mouvement restant"
            ));
        }
        Some(BattleForecast {
            attacker_power: odds.attacker_power,
            defender_power: odds.defender_power,
            // A6-L1 (ADR 0181): the balance bar and the verdict share one probability.
            attacker_share: odds.win_chance,
            attacker_win_chance: odds.win_chance,
            attacker_losses_pct: odds.attacker_losses_pct,
            defender_losses_pct: odds.defender_losses_pct,
            attacker_soldiers: head_count(&attacker_side),
            defender_soldiers: head_count(&defender_side),
            attacker_reinforcements: self.reinforcements(data, &attackers),
            defender_reinforcements: self.reinforcements(data, &defenders),
            modifiers,
            can_withdraw: player_attacks
                && (request.sortie
                    || self
                        .armies
                        .get(&request.attacker)
                        .is_some_and(|a| a.faction == self.player_faction)),
            siege: request.siege,
            lifts_siege,
            siege_place,
        })
    }

    /// Whether beating one side of a field battle lifts a siege: the place
    /// besieged by the army of the other side (it stands in it, in
    /// `Siege` stance) and the army that would relieve it (WH armya).
    fn siege_lifted_by(
        &self,
        data: &GameData,
        attacker: &crate::state::ArmyId,
        defender: &crate::state::ArmyId,
    ) -> (bool, String) {
        for id in [attacker, defender] {
            let Some(army) = self.armies.get(id) else {
                continue;
            };
            if army.stance != crate::state::Stance::Siege {
                continue;
            }
            let Some(place) = army.settlement() else {
                continue;
            };
            let besieged = self
                .settlements
                .get(place)
                .and_then(|s| s.siege.as_ref())
                .is_some_and(|siege| siege.attacker == army.faction);
            if besieged {
                return (true, data.settlement_name(place));
            }
        }
        (false, String::new())
    }

    /// Allied armies of a coalition, lead army (first) excluded.
    fn reinforcements(&self, data: &GameData, ids: &[crate::state::ArmyId]) -> Vec<Reinforcement> {
        let lead = ids.first().and_then(|id| self.armies.get(id));
        ids.iter()
            .skip(1)
            .filter_map(|id| {
                let army = self.armies.get(id)?;
                let distance_km = lead.map_or(0.0, |l| self.army_distance_km(data, army, l));
                let late = distance_km > data.free_movement_rules().engage_radius_km
                    && !(army.settlement().is_some()
                        && army.settlement() == lead.and_then(|l| l.settlement()));
                Some(Reinforcement {
                    army: id.to_string(),
                    faction_name: data.faction_name(&army.faction),
                    soldiers: army.total_strength(),
                    regiments: army.units.len() as u32,
                    general: army
                        .general
                        .as_ref()
                        .map(|g| self.character_name(data, g))
                        .unwrap_or_default(),
                    distance_km,
                    late,
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
        let place = data.settlement_name(&request.location);
        let mut event = if request.sortie {
            GameEvent::new(
                EventKind::Battle,
                format!("La sortie de {place} est annulée : la garnison reste dans la place."),
            )
        } else if request.siege {
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
