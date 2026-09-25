//! Player battles deferred to the 3D battle (M7, spec `docs/design/m7-battles.md` § 2).
//!
//! When [`CampaignState::interactive_battles`] is on and an army of the
//! player attacks or is attacked by an enemy army (lot M2), the battle is stored
//! in [`CampaignState::pending_battles`] instead of being auto-resolved; both
//! armies stop. The UI then either fights it with `sim-battle`
//! ([`CampaignState::battle_setup`] then
//! [`CampaignState::resolve_pending_battle`]) or auto-resolves it
//! ([`CampaignState::auto_resolve_pending`]). Whatever is still pending when
//! the next turn starts is auto-resolved first.

use data_model::{
    Ability, CharacterId, FactionId, GameData, ProvinceId, SettlementId, Terrain, UnitCategory,
    UnitStats,
};
use serde::{Deserialize, Serialize};
use sim_battle::{
    BattleOutcome, BattleSeason, BattleSetup, GeneralSetup, SideId, SideResult, SideSetup,
    SiegeSetup, UnitSetup,
};

use crate::battle_auto::{BattleResult, SideOutcome, Winner};
use crate::events::{EventKind, GameEvent};
use crate::movement;
use crate::research;
use crate::skills;
use crate::state::{Army, ArmyId, BattleRequest, CampaignState, Season};

/// Piety the victorious commander loses when his army gave no quarter (P1):
/// the Church condemned the killing of knights who had surrendered.
pub const NO_QUARTER_PIETY: i32 = 5;

/// A pending battle as shown by the end-of-turn dialog.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingBattle {
    pub index: usize,
    pub attacker: ArmyId,
    pub defender: ArmyId,
    /// Settlement where the battle takes place (lot C4), or the nearest one
    /// to a field battle (lot M2).
    pub location: SettlementId,
    pub province: ProvinceId,
    pub attacker_name: String,
    pub defender_name: String,
    /// Side of the player (`None` if the player is in neither army).
    pub player_side: Option<SideId>,
    /// M8: assault on the town's walls (the defender is the garrison).
    #[serde(default)]
    pub siege: bool,
}

/// Why a pending battle cannot be set up or resolved.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum BattleRequestError {
    #[error("aucune bataille en attente n°{0}")]
    UnknownBattle(usize),
    #[error("la bataille n°{0} n'a plus lieu d'être (armée disparue ou déplacée)")]
    Stale(usize),
    #[error("résultat invalide : {side} a {expected} unités, {found} pertes fournies")]
    UnitCountMismatch {
        side: &'static str,
        expected: usize,
        found: usize,
    },
    #[error("résultat invalide : l'unité {unit} de {side} perd plus d'hommes qu'elle n'en a")]
    LossesExceedStrength { side: &'static str, unit: usize },
    #[error("la bataille n°{0} ne peut être refusée : seul l'assaillant peut se retirer")]
    CannotWithdraw(usize),
}

/// Records a player battle instead of fighting it (called by
/// `movement::fight`). Returns `true` when the battle was deferred.
pub(crate) fn defer_player_battle(
    state: &mut CampaignState,
    data: &GameData,
    attacker_id: &ArmyId,
    defender_id: &ArmyId,
    events: &mut Vec<GameEvent>,
) -> bool {
    if !state.interactive_battles {
        return false;
    }
    let (Some(attacker), Some(defender)) =
        (state.armies.get(attacker_id), state.armies.get(defender_id))
    else {
        return false;
    };
    let player = &state.player_faction;
    // F1: the player may only be an ally present in the province.
    let involved = |lead: &ArmyId, enemy: &FactionId| {
        movement::battle_coalition(state, data, lead, enemy)
            .iter()
            .any(|id| state.armies.get(id).is_some_and(|a| &a.faction == player))
    };
    if !involved(attacker_id, &defender.faction) && !involved(defender_id, &attacker.faction) {
        return false;
    }
    if let Some(ai) = state.ai_turn.clone() {
        // Lot M3: during an AI faction's turn the battle is auto-resolved
        // at once; the player reads about it in the season report.
        let place = defender
            .settlement()
            .cloned()
            .or_else(|| crate::march::nearest_settlement(data, state.army_point(data, defender)));
        let mut event = GameEvent::new(
            EventKind::Battle,
            format!(
                "Pendant le tour {}, bataille livrée{} : {} contre {} (résolution automatique).",
                crate::events::de(&faction_name(data, &ai)),
                place.map_or_else(String::new, |p| format!(
                    " près de {}",
                    crate::siege::settlement_name(data, &p)
                )),
                faction_name(data, &attacker.faction),
                faction_name(data, &defender.faction)
            ),
        )
        .army(attacker_id)
        .faction(player);
        if let Some(province) = state.army_province(data, defender) {
            event = event.province(&province);
        }
        events.push(event);
        return false;
    }
    let Some(location) = defender
        .settlement()
        .cloned()
        .or_else(|| crate::march::nearest_settlement(data, state.army_point(data, defender)))
    else {
        return false;
    };
    let Some(province) = state.army_province(data, defender) else {
        return false;
    };
    events.push(
        GameEvent::new(
            EventKind::Battle,
            format!(
                "Bataille en vue à {} : {} contre {}.",
                crate::siege::settlement_name(data, &location),
                faction_name(data, &attacker.faction),
                faction_name(data, &defender.faction)
            ),
        )
        .province(&province)
        .army(attacker_id)
        .faction(player),
    );
    state.pending_battles.push(BattleRequest {
        attacker: attacker_id.clone(),
        defender: defender_id.clone(),
        location,
        province,
        siege: false,
    });
    true
}

/// Start of `end_turn`: auto-resolves every battle still pending.
pub(crate) fn auto_resolve_all_pending(
    state: &mut CampaignState,
    data: &GameData,
    events: &mut Vec<GameEvent>,
) {
    let pending = std::mem::take(&mut state.pending_battles);
    for request in pending {
        if request.siege {
            if is_live(state, &request) {
                crate::siege::auto_assault(state, data, &request.attacker, events);
            }
            continue;
        }
        if is_live(state, &request) {
            movement::auto_fight(state, data, &request.attacker, &request.defender, events);
        }
    }
}

/// Both armies still exist and are at war (they cannot move once the battle
/// is pending, lot M2); siege: the besiegers still besiege a garrisoned
/// settlement.
pub(crate) fn is_live(state: &CampaignState, request: &BattleRequest) -> bool {
    if request.siege {
        let (Some(army), Some(settlement)) = (
            state.armies.get(&request.attacker),
            state.settlements.get(&request.location),
        ) else {
            return false;
        };
        return army.is_at(&request.location)
            && !settlement.garrison.is_empty()
            && settlement
                .siege
                .as_ref()
                .is_some_and(|s| s.attacker == army.faction)
            && state.is_at_war(&army.faction, &settlement.controller);
    }
    match (
        state.armies.get(&request.attacker),
        state.armies.get(&request.defender),
    ) {
        (Some(a), Some(d)) => state.is_at_war(&a.faction, &d.faction),
        _ => false,
    }
}

fn faction_name(data: &GameData, id: &FactionId) -> String {
    data.factions
        .get(id)
        .map_or_else(|| id.to_string(), |f| f.short_or_display_name().to_owned())
}

fn province_name(data: &GameData, id: &ProvinceId) -> String {
    data.provinces
        .get(id)
        .map_or_else(|| id.to_string(), |p| p.name.display.clone())
}

/// L3 (ADR 0026): the besieged town of a landmark city is drawn from its plan
/// (`data/landmarks/<id>.json`, block `siege.battle`); `None` elsewhere or
/// when the block is missing or broken (the generic town is used).
fn siege_layout(
    data: &GameData,
    settlement: &data_model::SettlementId,
) -> Option<sim_battle::SiegeLayout> {
    data.landmarks
        .values()
        .find(|l| l.settlement == settlement.as_str())
        .and_then(|l| sim_battle::SiegeLayout::from_landmark(l).ok())
}

fn battle_season(season: Season) -> BattleSeason {
    match season {
        Season::Spring => BattleSeason::Spring,
        Season::Summer => BattleSeason::Summer,
        Season::Autumn => BattleSeason::Autumn,
        Season::Winter => BattleSeason::Winter,
    }
}

fn fallback_stats() -> UnitStats {
    UnitStats {
        melee: 30,
        ranged: 0,
        range: 0,
        armor: 20,
        morale: 50,
        speed: 40,
        ammo: 0,
        charge: None,
        siege_attack: None,
    }
}

fn side_setup(state: &CampaignState, data: &GameData, id: &ArmyId, army: &Army) -> SideSetup {
    let units: Vec<UnitSetup> = army
        .units
        .iter()
        .map(|unit| match data.unit_types.get(&unit.unit_type) {
            Some(unit_type) => {
                let mut setup = UnitSetup::from_unit_type(
                    unit_type,
                    unit.strength,
                    unit.morale,
                    unit.experience,
                );
                setup.max_soldiers = unit.max_strength.max(unit.strength);
                // G1: technology bonuses of the army's faction, per category
                // (same source as the auto-resolver's `side_from_army`), plus
                // the levying province's buildings (armoury, butts).
                let tech =
                    research::tech_unit_bonus(state, data, &army.faction, unit_type.category);
                setup.morale = research::boosted(setup.morale, tech.morale, 100);
                setup.stats.melee = research::boosted(setup.stats.melee, tech.melee, 255);
                setup.stats.armor = research::boosted(
                    setup.stats.armor,
                    tech.armor + f64::from(unit.levy_armor),
                    100,
                );
                if setup.stats.ranged > 0 {
                    setup.stats.ranged = research::boosted(
                        setup.stats.ranged,
                        tech.ranged + f64::from(unit.levy_ranged),
                        255,
                    );
                }
                setup
            }
            None => UnitSetup {
                unit_type: unit.unit_type.to_string(),
                name: unit.unit_type.to_string(),
                category: UnitCategory::Infantry,
                mounted: false,
                soldiers: unit.strength,
                max_soldiers: unit.max_strength.max(unit.strength),
                morale: unit.morale,
                experience: unit.experience,
                stats: fallback_stats(),
                abilities: Vec::<Ability>::new(),
            },
        })
        .collect();
    let general = army.general.as_ref().and_then(|character| {
        let c = state.characters.get(character).filter(|c| c.alive)?;
        let effects = skills::character_effects(state, data, character);
        let unit_index = units
            .iter()
            .position(|u| u.category == UnitCategory::Cavalry && u.soldiers > 0)
            .or_else(|| units.iter().position(|u| u.soldiers > 0))
            .unwrap_or(0);
        Some(GeneralSetup {
            character: character.to_string(),
            name: state.character_name(data, character),
            command: c.skills.command,
            unit_index,
            morale_bonus: effects.army_morale.apply(0.0),
            charge_percent: effects.battle_charge.apply(0.0),
            ranged_percent: effects.battle_ranged.apply(0.0),
            defense_percent: effects.battle_defense.apply(0.0),
        })
    });
    SideSetup {
        faction: army.faction.to_string(),
        faction_name: faction_name(data, &army.faction),
        army: id.to_string(),
        units,
        general,
    }
}

fn side_outcome(
    side: &'static str,
    army: &Army,
    result: &SideResult,
) -> Result<SideOutcome, BattleRequestError> {
    if result.losses.len() != army.units.len() {
        return Err(BattleRequestError::UnitCountMismatch {
            side,
            expected: army.units.len(),
            found: result.losses.len(),
        });
    }
    for (unit, (campaign, lost)) in army.units.iter().zip(&result.losses).enumerate() {
        if *lost > campaign.strength {
            return Err(BattleRequestError::LossesExceedStrength { side, unit });
        }
    }
    Ok(SideOutcome {
        power: 0.0,
        losses: result.losses.clone(),
        total_losses: result.losses.iter().sum(),
        morale_delta: result.morale_delta.clamp(-100, 100),
        routed: result.routed,
        general_captured: result.general_captured && !result.general_killed,
    })
}

impl CampaignState {
    /// Coalitions `(attackers, defenders)` of a field battle request (F1),
    /// each led by the army of the encounter.
    pub(crate) fn coalitions(
        &self,
        data: &GameData,
        request: &BattleRequest,
    ) -> (Vec<ArmyId>, Vec<ArmyId>) {
        let faction = |id: &ArmyId| self.armies.get(id).map(|a| a.faction.clone());
        let (Some(attacker), Some(defender)) =
            (faction(&request.attacker), faction(&request.defender))
        else {
            return (
                vec![request.attacker.clone()],
                vec![request.defender.clone()],
            );
        };
        (
            movement::battle_coalition(self, data, &request.attacker, &defender),
            movement::battle_coalition(self, data, &request.defender, &attacker),
        )
    }

    /// Side of the player in a field battle, allies included (F1).
    fn player_side_of(&self, data: &GameData, request: &BattleRequest) -> Option<SideId> {
        let (attackers, defenders) = self.coalitions(data, request);
        let has_player = |ids: &[ArmyId]| {
            ids.iter().any(|id| {
                self.armies
                    .get(id)
                    .is_some_and(|a| a.faction == self.player_faction)
            })
        };
        if has_player(&attackers) {
            Some(SideId::Attacker)
        } else if has_player(&defenders) {
            Some(SideId::Defender)
        } else {
            None
        }
    }

    /// Pending battles with display names, in index order.
    pub fn pending_battle_views(&self, data: &GameData) -> Vec<PendingBattle> {
        self.pending_battles
            .iter()
            .enumerate()
            .map(|(index, request)| {
                let faction = |id: &ArmyId| self.armies.get(id).map(|a| a.faction.clone());
                let attacker_faction = faction(&request.attacker);
                let defender_faction = if request.siege {
                    self.settlements
                        .get(&request.location)
                        .map(|s| s.controller.clone())
                } else {
                    faction(&request.defender)
                };
                let player_side = if request.siege {
                    if attacker_faction.as_ref() == Some(&self.player_faction) {
                        Some(SideId::Attacker)
                    } else if defender_faction.as_ref() == Some(&self.player_faction) {
                        Some(SideId::Defender)
                    } else {
                        None
                    }
                } else {
                    self.player_side_of(data, request)
                };
                let name =
                    |f: Option<FactionId>| f.map_or_else(String::new, |f| faction_name(data, &f));
                PendingBattle {
                    index,
                    attacker: request.attacker.clone(),
                    defender: request.defender.clone(),
                    location: request.location.clone(),
                    province: request.province.clone(),
                    attacker_name: name(attacker_faction),
                    defender_name: name(defender_faction),
                    player_side,
                    siege: request.siege,
                }
            })
            .collect()
    }

    /// Everything `sim-battle` needs to fight pending battle `index`.
    pub fn battle_setup(
        &self,
        data: &GameData,
        index: usize,
    ) -> Result<BattleSetup, BattleRequestError> {
        let request = self
            .pending_battles
            .get(index)
            .ok_or(BattleRequestError::UnknownBattle(index))?;
        if !is_live(self, request) {
            return Err(BattleRequestError::Stale(index));
        }
        if request.siege {
            return Ok(self.siege_battle_setup(data, request));
        }
        // F1: the allied armies of the province fight alongside.
        let (attackers, defenders) = self.coalitions(data, request);
        let attacker = movement::coalition_army(self, &attackers).expect("live battle");
        let defender = movement::coalition_army(self, &defenders).expect("live battle");
        let province = data.provinces.get(&request.province);
        let player_side = self.player_side_of(data, request);
        Ok(BattleSetup {
            province: request.province.to_string(),
            province_name: province_name(data, &request.province),
            terrain: province.map_or(Terrain::Plains, |p| p.terrain),
            river: province.is_some_and(|p| !p.rivers.is_empty()),
            season: battle_season(self.season),
            coastal: province.is_some_and(|p| p.coastal),
            village: None,
            attacker: side_setup(self, data, &request.attacker, &attacker),
            defender: side_setup(self, data, &request.defender, &defender),
            player_side,
            siege: None,
            siege_layout: None,
            orders: data.battle_orders.values().cloned().collect(),
        })
    }

    /// Applies the result of a battle fought outside the campaign (3D battle):
    /// losses, morale, captures, fallen generals, XP/traits, retreat, journal.
    /// The request is removed; the events are appended to the turn journal
    /// and returned.
    pub fn resolve_pending_battle(
        &mut self,
        data: &GameData,
        index: usize,
        outcome: &BattleOutcome,
    ) -> Result<Vec<GameEvent>, BattleRequestError> {
        let request = self
            .pending_battles
            .get(index)
            .cloned()
            .ok_or(BattleRequestError::UnknownBattle(index))?;
        if !is_live(self, &request) {
            self.pending_battles.remove(index);
            return Err(BattleRequestError::Stale(index));
        }
        let garrison = request
            .siege
            .then(|| crate::siege::garrison_army(self, &request.location))
            .flatten();
        // F1: field battles include the allied armies of the province, in
        // the same order as `battle_setup`.
        // G1: so do siege assaults (the garrison fights alone).
        let (attackers, defenders) = if request.siege {
            (
                crate::siege::assault_coalition(self, &request.attacker),
                vec![request.defender.clone()],
            )
        } else {
            self.coalitions(data, &request)
        };
        let attacker_combined = movement::coalition_army(self, &attackers).expect("live battle");
        let defender_combined = match &garrison {
            Some(_) => None,
            None => Some(movement::coalition_army(self, &defenders).expect("live battle")),
        };
        let attacker = &attacker_combined;
        let defender = garrison
            .as_ref()
            .or(defender_combined.as_ref())
            .expect("one of them");
        let mut attacker_outcome = side_outcome("l'attaquant", attacker, &outcome.attacker)?;
        let mut defender_outcome = side_outcome("le défenseur", defender, &outcome.defender)?;
        // P1 « pas de quartier »: when the victors gave no quarter, a beaten
        // general they caught is put to the sword, never held for ransom.
        let no_quarter = outcome.side(outcome.winner).no_quarter;
        let loser_side = outcome.winner.other();
        let mut fallen: Vec<CharacterId> = Vec::new();
        for (side, army, result, side_outcome) in [
            (
                SideId::Attacker,
                attacker,
                &outcome.attacker,
                &mut attacker_outcome,
            ),
            (
                SideId::Defender,
                defender,
                &outcome.defender,
                &mut defender_outcome,
            ),
        ] {
            let slain = no_quarter && side == loser_side && result.general_captured;
            if slain {
                side_outcome.general_captured = false;
            }
            if result.general_killed || slain {
                fallen.extend(army.general.clone());
            }
        }
        let winner_general = match outcome.winner {
            SideId::Attacker => attacker.general.clone(),
            SideId::Defender => defender.general.clone(),
        };
        let winner_faction = match outcome.winner {
            SideId::Attacker => attacker.faction.clone(),
            SideId::Defender => defender.faction.clone(),
        };
        self.pending_battles.remove(index);

        let mut events = Vec::new();
        for general in &fallen {
            if self.characters.get(general).is_some_and(|c| c.alive) {
                let name = self.character_name(data, general);
                let mut event = GameEvent::new(
                    EventKind::Death,
                    format!("{name} est tombé sur le champ de bataille."),
                )
                .province(&request.province);
                if let Some(faction) = self.characters.get(general).map(|c| c.faction.clone()) {
                    event = event.faction(&faction);
                }
                events.push(event);
                crate::characters::kill(self, data, general, &mut events);
            }
        }
        let result = BattleResult {
            winner: match outcome.winner {
                SideId::Attacker => Winner::Attacker,
                SideId::Defender => Winner::Defender,
            },
            attacker: attacker_outcome,
            defender: defender_outcome,
        };
        if request.siege {
            let walls = crate::siege::walls_stand(self, data, &request.attacker, &request.location);
            crate::siege::apply_assault_result(
                self,
                data,
                &attackers,
                &request.location,
                &result,
                walls,
                &mut events,
            );
            if no_quarter {
                self.no_quarter_toll(
                    data,
                    &winner_faction,
                    winner_general.as_ref(),
                    &request.province,
                    &mut events,
                );
            }
            self.events.extend(events.iter().cloned());
            return Ok(events);
        }
        movement::apply_battle_result(self, data, &attackers, &defenders, &result, &mut events);
        if no_quarter {
            self.no_quarter_toll(
                data,
                &winner_faction,
                winner_general.as_ref(),
                &request.province,
                &mut events,
            );
        }
        self.events.extend(events.iter().cloned());
        Ok(events)
    }

    /// P1: the victors gave no quarter. No noble of the beaten side is held
    /// for ransom (the caught general is slain by the caller), and the
    /// victorious commander loses [`NO_QUARTER_PIETY`] piety for the
    /// slaughter of Christian knights who had yielded.
    fn no_quarter_toll(
        &mut self,
        data: &GameData,
        faction: &FactionId,
        general: Option<&CharacterId>,
        province: &ProvinceId,
        events: &mut Vec<GameEvent>,
    ) {
        let faction_name = data.factions.get(faction).map_or_else(
            || faction.to_string(),
            |f| f.short_or_display_name().to_owned(),
        );
        let alive_general = general.filter(|id| self.characters.get(*id).is_some_and(|c| c.alive));
        let text = match alive_general {
            Some(id) => {
                let name = self.character_name(data, id);
                let character = self.characters.get_mut(id).expect("checked");
                character.piety =
                    (i32::from(character.piety) - NO_QUARTER_PIETY).clamp(0, 100) as u8;
                format!(
                    "Pas de quartier : {faction_name} a passé les vaincus au fil de l'épée, \
                     sans prisonnier ni rançon. {name} en perd {NO_QUARTER_PIETY} de piété."
                )
            }
            None => format!(
                "Pas de quartier : {faction_name} a passé les vaincus au fil de l'épée, \
                 sans prisonnier ni rançon."
            ),
        };
        events.push(
            GameEvent::new(EventKind::Battle, text)
                .province(province)
                .faction(faction),
        );
    }

    /// Setup of a siege battle: the besieging army against the garrison
    /// (general: the governor) behind walls of the town's fortification
    /// level, with the campaign breach already done.
    fn siege_battle_setup(&self, data: &GameData, request: &BattleRequest) -> BattleSetup {
        // G1: the allied armies of the province storm alongside.
        let attackers = crate::siege::assault_coalition(self, &request.attacker);
        let attacker = &movement::coalition_army(self, &attackers).expect("live siege");
        let garrison = crate::siege::garrison_army(self, &request.location).expect("live siege");
        let province = data.provinces.get(&request.province);
        let player_attacks = attackers.iter().any(|id| {
            self.armies
                .get(id)
                .is_some_and(|a| a.faction == self.player_faction)
        });
        let player_side = if player_attacks {
            Some(SideId::Attacker)
        } else if garrison.faction == self.player_faction {
            Some(SideId::Defender)
        } else {
            None
        };
        let breach = self
            .settlements
            .get(&request.location)
            .and_then(|s| s.siege.as_ref())
            .map_or(0, |s| s.breach);
        let mut defender = side_setup(self, data, &request.attacker, &garrison);
        defender.army = String::new();
        BattleSetup {
            province: request.province.to_string(),
            province_name: province_name(data, &request.province),
            terrain: province.map_or(Terrain::Plains, |p| p.terrain),
            river: false,
            season: battle_season(self.season),
            coastal: false,
            village: Some(false),
            attacker: side_setup(self, data, &request.attacker, attacker),
            defender,
            player_side,
            siege: Some(SiegeSetup {
                fortification: self.fortification_level(data, &request.location),
                breach,
            }),
            siege_layout: siege_layout(data, &request.location),
            orders: data.battle_orders.values().cloned().collect(),
        }
    }

    /// Debug helper for headless tests and screenshots: puts `army` in
    /// siege of the city of the enemy province `province` (moving it there,
    /// giving the town a garrison if it has none) and records a pending
    /// siege battle. Returns the battle index.
    pub fn debug_stage_siege(
        &mut self,
        data: &GameData,
        army: &ArmyId,
        province: &ProvinceId,
    ) -> Result<usize, BattleRequestError> {
        let index = self.pending_battles.len();
        let Some(a) = self.armies.get(army) else {
            return Err(BattleRequestError::Stale(index));
        };
        let faction = a.faction.clone();
        let Some(city) = self.province_city_id(province).cloned() else {
            return Err(BattleRequestError::Stale(index));
        };
        let controller = self.settlements[&city].controller.clone();
        if !self.is_at_war(&faction, &controller) {
            return Err(BattleRequestError::Stale(index));
        }
        let army_state = self.armies.get_mut(army).expect("exists");
        army_state.position = crate::state::ArmyPosition::Settlement(city.clone());
        army_state.clear_plan();
        army_state.stance = crate::state::Stance::Siege;
        if let Some(general) = army_state.general.clone() {
            if let Some(c) = self.characters.get_mut(&general) {
                c.location = Some(province.clone());
            }
        }
        let p = self.settlements.get_mut(&city).expect("exists");
        if p.garrison.is_empty() {
            if let Some(unit_type) = data
                .unit_types
                .values()
                .find(|t| t.id.as_str() == "unit_urban_militia")
            {
                for _ in 0..3 {
                    p.garrison.push(crate::state::Unit {
                        unit_type: unit_type.id.clone(),
                        strength: unit_type.soldiers,
                        max_strength: unit_type.soldiers,
                        morale: unit_type.stats.morale,
                        experience: 0,
                        levy_armor: 0,
                        levy_ranged: 0,
                    });
                }
            }
        }
        if p.siege.is_none() {
            p.siege = Some(crate::state::SiegeState {
                attacker: faction,
                turns_left: 4,
                turns_elapsed: 1,
                supplies: 80,
                breach: 0,
                started_turn: 0,
            });
        }
        self.pending_battles.push(BattleRequest {
            attacker: army.clone(),
            defender: army.clone(),
            location: city,
            province: province.clone(),
            siege: true,
        });
        Ok(index)
    }

    /// Debug helper for headless tests and screenshots: moves `defender` to
    /// `attacker`'s position and records a pending battle between them
    /// (they must be at war; the attacker must stand in a settlement).
    /// Returns the battle index.
    pub fn debug_stage_battle(
        &mut self,
        attacker: &ArmyId,
        defender: &ArmyId,
    ) -> Result<usize, BattleRequestError> {
        let index = self.pending_battles.len();
        let (Some(a), Some(d)) = (self.armies.get(attacker), self.armies.get(defender)) else {
            return Err(BattleRequestError::Stale(index));
        };
        if !self.is_at_war(&a.faction, &d.faction) {
            return Err(BattleRequestError::Stale(index));
        }
        let Some(location) = a.settlement().cloned() else {
            return Err(BattleRequestError::Stale(index));
        };
        let Some(province) = self.settlement_province(&location).cloned() else {
            return Err(BattleRequestError::Stale(index));
        };
        let army = self.armies.get_mut(defender).expect("exists");
        army.position = crate::state::ArmyPosition::Settlement(location.clone());
        army.clear_plan();
        if let Some(general) = army.general.clone() {
            if let Some(c) = self.characters.get_mut(&general) {
                c.location = Some(province.clone());
            }
        }
        self.pending_battles.push(BattleRequest {
            attacker: attacker.clone(),
            defender: defender.clone(),
            location,
            province,
            siege: false,
        });
        Ok(index)
    }

    /// Auto-resolves pending battle `index` now (the "Résolution
    /// automatique" button); events are appended to the journal and returned.
    pub fn auto_resolve_pending(
        &mut self,
        data: &GameData,
        index: usize,
    ) -> Result<Vec<GameEvent>, BattleRequestError> {
        if index >= self.pending_battles.len() {
            return Err(BattleRequestError::UnknownBattle(index));
        }
        let request = self.pending_battles.remove(index);
        if !is_live(self, &request) {
            return Err(BattleRequestError::Stale(index));
        }
        if request.siege {
            let mut events = Vec::new();
            crate::siege::auto_assault(self, data, &request.attacker, &mut events);
            self.events.extend(events.iter().cloned());
            return Ok(events);
        }
        let mut events = Vec::new();
        movement::auto_fight(
            self,
            data,
            &request.attacker,
            &request.defender,
            &mut events,
        );

        self.events.extend(events.iter().cloned());
        Ok(events)
    }
}
