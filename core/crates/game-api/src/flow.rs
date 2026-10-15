//! Campaign ↔ battle flow views: pending battles, the launch package of a 3D battle, the result
//! of a battle (class band, aftermath), map encounters.
//!
//! Replaces the state-reading half of `godot-bridge` `battle_sim_campaign.rs`
//! (`get_pending_battles`, `get_battle_setup`, `get_last_battle_outcome`),
//! `campaign_sim_encounters.rs` and `game/scripts/battle/battle_aftermath.gd`. No rule here: the
//! core computes, these functions only shape and compare.

use std::collections::BTreeMap;

use data_model::{FactionId, GameData};
use serde::Serialize;
use sim_battle::BattleSetup;
use sim_campaign::battle_forecast::BattleForecast;
use sim_campaign::battle_request::AutoResolveReport;
use sim_campaign::{ArmyId, CampaignState, GameEvent, Order};

/// A pending battle of the campaign (`get_pending_battles`).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PendingBattleView {
    pub index: usize,
    /// Army ids (the attacker of a sortie is the garrison; ids are those of the request).
    pub attacker: String,
    pub defender: String,
    pub province: String,
    pub province_name: String,
    pub attacker_name: String,
    pub defender_name: String,
    /// `attacker`, `defender` or empty (the player is not in the battle).
    pub player_side: String,
    pub attacker_strength: i64,
    pub defender_strength: i64,
    pub siege: bool,
    pub sortie: bool,
    pub fortification: i64,
    pub location: String,
    pub settlement_name: String,
    pub settlement_kind: String,
    pub breach: i64,
    pub seed: u64,
}

/// Pending battles in index order, as the Godot bridge lists them.
pub fn pending_battles(data: &GameData, state: &CampaignState) -> Vec<PendingBattleView> {
    state
        .pending_battle_views(data)
        .iter()
        .map(|view| {
            let strength =
                |id: &ArmyId| state.army(id).map_or(0, |a| i64::from(a.total_strength()));
            let settlement = state.settlement_state(&view.location);
            let garrison: i64 = settlement.map_or(0, |s| {
                s.garrison.iter().map(|u| i64::from(u.strength)).sum()
            });
            let seed =
                state.seed ^ (u64::from(state.turn()) << 20) ^ ((view.index as u64) << 8) ^ 0xBA77;
            PendingBattleView {
                index: view.index,
                attacker: view.attacker.to_string(),
                defender: view.defender.to_string(),
                province: view.province.to_string(),
                province_name: data.province_name(&view.province),
                attacker_name: view.attacker_name.clone(),
                defender_name: view.defender_name.clone(),
                player_side: view.player_side.map_or("", |s| s.key()).to_owned(),
                attacker_strength: if view.sortie {
                    garrison
                } else {
                    strength(&view.attacker)
                },
                defender_strength: if view.siege {
                    garrison
                } else {
                    strength(&view.defender)
                },
                siege: view.siege,
                sortie: view.sortie,
                fortification: i64::from(state.fortification_level(data, &view.location)),
                location: view.location.to_string(),
                settlement_name: data.settlement_name(&view.location),
                settlement_kind: state.settlement_kind(&view.location).key().to_owned(),
                breach: settlement
                    .and_then(|s| s.siege.as_ref())
                    .map_or(0, |s| i64::from(s.breach)),
                seed: seed & 0x7FFF_FFFF_FFFF,
            }
        })
        .collect()
}

/// Estimated balance of pending battle `index` (`get_battle_forecast`).
pub fn battle_forecast(
    data: &GameData,
    state: &CampaignState,
    index: usize,
) -> Result<BattleForecast, String> {
    state
        .battle_forecast(data, index)
        .map_err(|e| e.to_string())
}

/// What the 3D battle needs to be built from a pending battle (`get_battle_setup` + `seed`).
#[derive(Debug, Clone)]
pub struct BattleLaunch {
    pub setup: BattleSetup,
    pub seed: u64,
    /// Starting hour drawn by the campaign (EP8), no random stream consumed.
    pub hour: f64,
    pub index: usize,
}

/// Setup, seed and hour of pending battle `index`.
pub fn battle_launch(
    data: &GameData,
    state: &CampaignState,
    index: usize,
) -> Result<BattleLaunch, String> {
    let setup = state.battle_setup(data, index).map_err(|e| e.to_string())?;
    let seed = pending_battles(data, state)
        .into_iter()
        .find(|view| view.index == index)
        .map_or(state.seed, |view| view.seed);
    let key = sim_battle::time_of_day::campaign_battle_key(state.turn(), index, &setup.province);
    let hour = sim_battle::TimeOfDayRules::bundled().campaign_hour(key);
    Ok(BattleLaunch {
        setup,
        seed,
        hour,
        index,
    })
}

/// Class of the last field battle (`get_last_battle_outcome`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OutcomeView {
    pub turn: u32,
    pub province: String,
    pub attacker_faction: String,
    pub defender_faction: String,
    pub attacker_class: String,
    pub attacker_label: String,
    pub defender_class: String,
    pub defender_label: String,
    pub player_class: Option<String>,
    pub player_label: Option<String>,
}

/// The last battle classification, `None` before any field battle.
pub fn last_battle_outcome(state: &CampaignState) -> Option<OutcomeView> {
    let report = state.last_battle_outcome.as_ref()?;
    let player = report.class_of(state.player_faction());
    Some(OutcomeView {
        turn: report.turn,
        province: report.province.to_string(),
        attacker_faction: report.attacker_faction.to_string(),
        defender_faction: report.defender_faction.to_string(),
        attacker_class: report.attacker.key.clone(),
        attacker_label: report.attacker.label.clone(),
        defender_class: report.defender.key.clone(),
        defender_label: report.defender.label.clone(),
        player_class: player.map(|v| v.key.clone()),
        player_label: player.map(|v| v.label.clone()),
    })
}

/// Result of an auto-resolution.
#[derive(Debug, Clone)]
pub struct AutoResolved {
    pub events: Vec<GameEvent>,
    /// `None` for an assault (the result is in the events).
    pub report: Option<AutoResolveReport>,
}

/// Auto-resolves pending battle `index` (`auto_resolve_battle_report`).
pub fn auto_resolve(
    data: &GameData,
    state: &mut CampaignState,
    index: usize,
) -> Result<AutoResolved, String> {
    state
        .auto_resolve_pending_report(data, index)
        .map(|(events, report)| AutoResolved { events, report })
        .map_err(|e| e.to_string())
}

/// The player calls off pending battle `index` (`withdraw_pending_battle`).
pub fn withdraw(
    data: &GameData,
    state: &mut CampaignState,
    index: usize,
) -> Result<Vec<GameEvent>, String> {
    state
        .withdraw_pending_battle(data, index)
        .map_err(|e| e.to_string())
}

/// Applies the outcome of a 3D battle (`resolve_battle`); the class changes with it.
pub fn resolve_battle(
    data: &GameData,
    state: &mut CampaignState,
    index: usize,
    outcome: &sim_battle::BattleOutcome,
) -> Result<Vec<GameEvent>, String> {
    state
        .resolve_pending_battle(data, index, outcome)
        .map_err(|e| e.to_string())
}

// --- Pre-battle brief ----------------------------------------------------------------------

/// A general shown on the brief.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BriefGeneral {
    pub name: String,
    pub command: u8,
}

/// One regiment card of the brief.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BriefUnit {
    pub name: String,
    pub unit_type: String,
    pub soldiers: u32,
    pub max_soldiers: u32,
    pub experience: u8,
    pub is_general: bool,
}

/// One side of the brief, seen from the player (slot 0 = player).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BriefSide {
    /// `attacker` or `defender`.
    pub side: String,
    pub faction: String,
    pub name: String,
    pub role: String,
    pub soldiers: u32,
    pub regiments: usize,
    pub general: Option<BriefGeneral>,
    pub reinforcements: String,
    pub composition: String,
    pub units: Vec<BriefUnit>,
}

/// Everything the pre-battle dialog shows (`pre_battle_dialog.gd` `show_battle`), computed from
/// the pending battle, its setup and the core's forecast. French texts; the player is slot 0.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BattleBrief {
    pub index: usize,
    pub seed: u64,
    pub title: String,
    pub subtitle: String,
    pub siege: bool,
    pub sortie: bool,
    /// The player's side (`attacker` when the player is not in the battle).
    pub player_side: String,
    /// Share of the balance bar on the player's side, 0-1.
    pub share: f64,
    pub chance: Option<f64>,
    pub verdict: String,
    /// `good`, `fair` or `poor`.
    pub verdict_tone: String,
    pub chance_text: String,
    pub auto_risk: String,
    pub conditions: String,
    pub modifiers: Vec<String>,
    pub fight_label: String,
    pub withdraw_label: String,
    pub can_withdraw: bool,
    pub sides: [BriefSide; 2],
}

/// Verdict of an auto-resolution chance (`BattleUiKit.verdict`).
pub fn verdict(chance: f64) -> (&'static str, &'static str) {
    if chance >= 0.95 {
        ("Victoire presque certaine", "good")
    } else if chance >= 0.7 {
        ("Victoire probable", "good")
    } else if chance > 0.3 {
        ("Issue incertaine", "fair")
    } else if chance > 0.05 {
        ("Défaite probable", "poor")
    } else {
        ("Défaite presque certaine", "poor")
    }
}

fn terrain_fr(key: &str) -> &str {
    match key {
        "plains" => "plaines",
        "hills" => "collines",
        "mountains" => "montagnes",
        "forest" => "forêt",
        "marsh" => "marais",
        "heath" => "lande",
        "bocage" => "bocage",
        "steppe" => "steppe",
        "desert" => "désert",
        other => other,
    }
}

fn season_fr(key: &str) -> &str {
    match key {
        "spring" => "printemps",
        "summer" => "été",
        "autumn" => "automne",
        "winter" => "hiver",
        other => other,
    }
}

fn digits(value: i64) -> String {
    let text = value.abs().to_string();
    let mut out = String::new();
    for (i, c) in text.chars().enumerate() {
        if i > 0 && (text.len() - i).is_multiple_of(3) {
            out.push('\u{202f}');
        }
        out.push(c);
    }
    if value < 0 {
        format!("-{out}")
    } else {
        out
    }
}

fn key_of<T: Serialize>(value: &T) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap_or_default()
}

/// Builds the brief of pending battle `view`.
pub fn battle_brief(
    data: &GameData,
    state: &CampaignState,
    view: &PendingBattleView,
) -> Result<BattleBrief, String> {
    let launch = battle_launch(data, state, view.index)?;
    let setup = &launch.setup;
    let forecast = battle_forecast(data, state, view.index).ok();
    let player_side = if view.player_side.is_empty() {
        "attacker"
    } else {
        view.player_side.as_str()
    };
    let enemy_side = if player_side == "attacker" {
        "defender"
    } else {
        "attacker"
    };
    let place = if view.siege || view.sortie {
        view.settlement_name.as_str()
    } else {
        view.province_name.as_str()
    };
    let ambush_victim = setup.opening.ambush_victim();
    let mut title = if view.siege {
        format!("Assaut de {place}")
    } else if view.sortie {
        format!("Sortie de {place}")
    } else {
        format!("Bataille en vue\u{a0}: {place}")
    };
    if ambush_victim.is_some() {
        title = format!("Embuscade\u{a0}! {place}");
    }
    let season = season_fr(&key_of(&setup.season)).to_owned();
    let mut subtitle = format!("{} contre {}", view.attacker_name, view.defender_name);
    if view.siege {
        subtitle += &format!(
            " · murailles de niveau {}, brèche {} %",
            view.fortification, view.breach
        );
    } else if !view.province_name.is_empty() {
        subtitle += &format!(" · {}", view.province_name);
    }
    if !season.is_empty() {
        subtitle += &format!(" · {season}");
    }
    if let Some(victim) = ambush_victim {
        let caught = if victim.key() == player_side {
            "Votre ost est surpris"
        } else {
            "L’ennemi est surpris"
        };
        subtitle = format!("{caught} en colonne de marche · {subtitle}");
    }
    // Balance bar and verdict, from the player's side.
    let (share, chance, verdict_text, tone, chance_text, auto_risk) = match &forecast {
        None => {
            let a = view.attacker_strength as f64;
            let d = view.defender_strength as f64;
            let attacker_share = a / (a + d).max(1.0);
            let share = if player_side == "attacker" {
                attacker_share
            } else {
                1.0 - attacker_share
            };
            (
                share,
                None,
                "Rapport de forces".to_owned(),
                "fair",
                "Estimation indisponible\u{a0}: effectifs seuls.".to_owned(),
                String::new(),
            )
        }
        Some(f) => {
            let mine = player_side == "attacker";
            let pick = |a: f64, d: f64| if mine { a } else { d };
            let share = pick(f.attacker_share, 1.0 - f.attacker_share);
            let chance = pick(f.attacker_win_chance, 1.0 - f.attacker_win_chance);
            let (text, tone) = verdict(chance);
            let power = |own: bool| {
                let attacker = own == mine;
                digits(
                    (if attacker {
                        f.attacker_power
                    } else {
                        f.defender_power
                    })
                    .round() as i64,
                )
            };
            let losses = format!(
                " · pertes estimées\u{a0}: vous {} %, ennemi {} %",
                pick(f.attacker_losses_pct, f.defender_losses_pct).round() as i64,
                pick(f.defender_losses_pct, f.attacker_losses_pct).round() as i64
            );
            let own_risk =
                pick(f.attacker_general_loss_pct, f.defender_general_loss_pct).round() as i64;
            let foe_risk =
                pick(f.defender_general_loss_pct, f.attacker_general_loss_pct).round() as i64;
            let mut parts = Vec::new();
            if own_risk > 0 {
                parts.push(format!("Risque\u{a0}: général perdu {own_risk} %"));
            }
            if foe_risk > 0 {
                parts.push(if own_risk > 0 {
                    format!("chef ennemi pris {foe_risk} %")
                } else {
                    format!("Chef ennemi pris\u{a0}: {foe_risk} %")
                });
            }
            (
                share,
                Some(chance),
                text.to_owned(),
                tone,
                format!(
                    "En résolution automatique\u{a0}: {} % de chances · puissance {} contre {}{}{} · une bataille menée peut renverser l’issue",
                    (chance * 100.0).round() as i64,
                    power(true),
                    power(false),
                    if view.siege { " (assaut)" } else { "" },
                    losses
                ),
                parts.join(" · "),
            )
        }
    };
    // Conditions line.
    let mut terrain_text = terrain_fr(&key_of(&setup.terrain)).to_owned();
    if setup.river {
        terrain_text += ", rivière et gués";
    }
    let conditions = format!(
        "Terrain\u{a0}: {terrain_text}   ·   Saison\u{a0}: {season}   ·   Heure\u{a0}: {:.0} h",
        launch.hour
    );
    let mut modifiers: Vec<String> = forecast
        .as_ref()
        .map(|f| f.modifiers.clone())
        .unwrap_or_default();
    if view.siege && modifiers.is_empty() {
        modifiers.push(
            "Brèche ouverte ou tours de siège\u{a0}: les murailles ne gênent plus l’assaut"
                .to_owned(),
        );
    }
    if let Some(victim) = ambush_victim {
        let name = if victim.key() == "attacker" {
            &view.attacker_name
        } else {
            &view.defender_name
        };
        modifiers.push(format!(
            "Embuscade\u{a0}! {name} en colonne de marche, sans déploiement\u{a0}; l’embusqué se range sur ses flancs"
        ));
    }
    for (side, setup_side, name) in [
        ("attacker", &setup.attacker, &view.attacker_name),
        ("defender", &setup.defender, &view.defender_name),
    ] {
        let _ = side;
        if setup_side.forced_march {
            modifiers.push(format!(
                "Marche forcée ({name})\u{a0}: troupes fatiguées, placement sans déploiement"
            ));
        }
        if setup_side.entrenched {
            modifiers.push(format!(
                "Camp retranché ({name})\u{a0}: pieux plantés et palissade devant la ligne"
            ));
        }
    }
    let side_brief = |side: &str| -> BriefSide {
        let (setup_side, name) = if side == "attacker" {
            (&setup.attacker, &view.attacker_name)
        } else {
            (&setup.defender, &view.defender_name)
        };
        let role = match (view.sortie, view.siege, side == "attacker") {
            (true, _, true) => "garnison",
            (true, _, false) => "assiégeants",
            (_, true, true) => "assiégeants",
            (_, true, false) => "garnison",
            (_, _, true) => "assaillant",
            (_, _, false) => "défenseur",
        };
        let general_index = setup_side.general.as_ref().map(|g| g.unit_index);
        let units: Vec<BriefUnit> = setup_side
            .units
            .iter()
            .enumerate()
            .map(|(i, u)| BriefUnit {
                name: u.name.clone(),
                unit_type: u.unit_type.clone(),
                soldiers: u.soldiers,
                max_soldiers: u.max_soldiers,
                experience: u.experience,
                is_general: general_index == Some(i),
            })
            .collect();
        let mut order: Vec<(&str, usize)> = Vec::new();
        for unit in &units {
            match order.iter_mut().find(|(n, _)| *n == unit.name) {
                Some(entry) => entry.1 += 1,
                None => order.push((unit.name.as_str(), 1)),
            }
        }
        let reinforcements = forecast
            .as_ref()
            .map(|f| {
                if side == "attacker" {
                    &f.attacker_reinforcements
                } else {
                    &f.defender_reinforcements
                }
            })
            .map(|list| {
                list.iter()
                    .map(|r| {
                        let mut text = format!(
                            "{} ({} h., {} rég.)",
                            r.faction_name,
                            digits(i64::from(r.soldiers)),
                            r.regiments
                        );
                        if r.late {
                            text += &format!(" à {} km", r.distance_km.round() as i64);
                            if r.committed_percent < 100 {
                                text += &format!(", ~{} %", r.committed_percent);
                            }
                        }
                        if !r.general.is_empty() {
                            text += &format!(" sous {}", r.general);
                        }
                        text
                    })
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .unwrap_or_default();
        BriefSide {
            side: side.to_owned(),
            faction: setup_side.faction.clone(),
            name: name.clone(),
            role: role.to_owned(),
            soldiers: units.iter().map(|u| u.soldiers).sum(),
            regiments: units.len(),
            general: setup_side.general.as_ref().map(|g| BriefGeneral {
                name: g.name.clone(),
                command: g.command,
            }),
            reinforcements,
            composition: order
                .iter()
                .map(|(n, c)| format!("{c} × {n}"))
                .collect::<Vec<_>>()
                .join(", "),
            units,
        }
    };
    Ok(BattleBrief {
        index: view.index,
        seed: launch.seed,
        title,
        subtitle,
        siege: view.siege,
        sortie: view.sortie,
        player_side: player_side.to_owned(),
        share,
        chance,
        verdict: verdict_text,
        verdict_tone: tone.to_owned(),
        chance_text,
        auto_risk,
        conditions,
        modifiers,
        fight_label: if view.siege {
            "Donner l’assaut"
        } else if view.sortie {
            "Livrer bataille"
        } else {
            "Combattre"
        }
        .to_owned(),
        withdraw_label: if view.siege {
            "Maintenir le siège"
        } else if view.sortie {
            "Rester dans la place"
        } else {
            "Retraite"
        }
        .to_owned(),
        can_withdraw: forecast.as_ref().is_some_and(|f| f.can_withdraw),
        sides: [side_brief(player_side), side_brief(enemy_side)],
    })
}

// --- Aftermath -----------------------------------------------------------------------------

/// A captive for the aftermath.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CaptiveLine {
    pub name: String,
    /// Faction of the captive (`held`) or of the captor (`lost`).
    pub faction: String,
    pub ransom: i64,
    pub rank: String,
}

/// State read before and after a battle of the player (`BattleAftermath.snapshot`).
#[derive(Debug, Clone, Default)]
pub struct AftermathSnapshot {
    /// `(unit type, experience)` of the army's regiments.
    units: Vec<(String, u8)>,
    general: Option<GeneralReading>,
    /// Captives held by the player, by character id.
    held: BTreeMap<String, CaptiveLine>,
    /// The player's captives held by others.
    ours: BTreeMap<String, CaptiveLine>,
}

#[derive(Debug, Clone)]
struct GeneralReading {
    name: String,
    experience: u32,
    skill_points: u32,
    alive: bool,
    captive: bool,
}

/// Reads army, general and captives before / after `resolve`.
pub fn aftermath_snapshot(
    data: &GameData,
    state: &CampaignState,
    army: Option<&ArmyId>,
) -> AftermathSnapshot {
    let mut snap = AftermathSnapshot::default();
    let player: &FactionId = state.player_faction();
    if let Some(army) = army.and_then(|id| state.army(id)) {
        snap.units = army
            .units
            .iter()
            .map(|u| (u.unit_type.to_string(), u.experience))
            .collect();
        snap.general = army
            .general
            .as_ref()
            .and_then(|id| state.character(id).map(|c| (id, c)))
            .map(|(id, c)| GeneralReading {
                name: state.character_name(data, id),
                experience: c.experience,
                skill_points: c.skill_points,
                alive: c.alive,
                captive: c.captive,
            });
    }
    for (id, c) in &state.characters {
        if !c.alive || !c.captive {
            continue;
        }
        let line = |faction: &FactionId| CaptiveLine {
            name: state.character_name(data, id),
            faction: faction.to_string(),
            ransom: sim_campaign::ransom::ransom_amount(state, data, id),
            rank: sim_campaign::ransom::captive_rank(state, id)
                .label_fr()
                .to_owned(),
        };
        if &c.faction == player {
            if let Some(captor) = &c.captor {
                snap.ours.insert(id.to_string(), line(captor));
            }
        } else if c.captor.as_ref() == Some(player) {
            snap.held.insert(id.to_string(), line(&c.faction));
        }
    }
    snap
}

/// What changed for the player's army and captives (`BattleAftermath.diff`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Aftermath {
    pub general_name: String,
    pub general_xp: u32,
    pub skill_points: u32,
    /// `""`, `tombé au combat` or `prisonnier`.
    pub general_fate: String,
    pub veterans: u32,
    pub captives: Vec<CaptiveLine>,
    pub lost: Vec<CaptiveLine>,
    pub ransom_total: i64,
}

pub fn aftermath_diff(before: &AftermathSnapshot, after: &AftermathSnapshot) -> Aftermath {
    let mut result = Aftermath::default();
    if let Some(g0) = &before.general {
        result.general_name = g0.name.clone();
        if let Some(g1) = &after.general {
            result.general_xp = g1.experience.saturating_sub(g0.experience);
            result.skill_points = g1.skill_points.saturating_sub(g0.skill_points);
            if !g1.alive {
                result.general_fate = "tombé au combat".into();
            } else if g1.captive && !g0.captive {
                result.general_fate = "prisonnier".into();
            }
        }
    }
    result.veterans = veterans(&before.units, &after.units);
    for (id, captive) in &after.held {
        if !before.held.contains_key(id) {
            result.ransom_total += captive.ransom;
            result.captives.push(captive.clone());
        }
    }
    for (id, captive) in &after.ours {
        if !before.ours.contains_key(id) {
            result.lost.push(captive.clone());
        }
    }
    result
}

/// Regiments whose experience rose. Annihilated regiments disappear from the army: the lists
/// are paired in order by unit type, skipping the ones missing afterwards.
fn veterans(before: &[(String, u8)], after: &[(String, u8)]) -> u32 {
    let mut count = 0;
    let mut i = 0;
    for (unit_type, experience) in after {
        while i < before.len() && before[i].0 != *unit_type {
            i += 1;
        }
        let Some(old) = before.get(i) else { break };
        if *experience > old.1 {
            count += 1;
        }
        i += 1;
    }
    count
}

// --- Encounters ----------------------------------------------------------------------------

pub use sim_campaign::encounter::{EncounterOptionView, EncounterSiteView, PendingEncounterView};

/// Sites the player sees (`get_encounter_sites`).
pub fn encounter_sites(data: &GameData, state: &CampaignState) -> Vec<EncounterSiteView> {
    state.encounter_site_views(data, state.player_faction())
}

/// Encounters of the player awaiting a choice (`get_pending_encounters`).
pub fn pending_encounters(data: &GameData, state: &CampaignState) -> Vec<PendingEncounterView> {
    state.pending_encounter_views(data, state.player_faction())
}

/// Answers a pending encounter (`choose_encounter_option`).
pub fn choose_encounter_option(
    data: &GameData,
    state: &mut CampaignState,
    army: &str,
    site: u32,
    option: usize,
) -> Result<(), String> {
    let army = ArmyId::parse(army).ok_or_else(|| format!("armée inconnue : {army}"))?;
    state
        .submit_order(data, Order::ChooseEncounterOption { army, site, option })
        .map_err(|e| e.to_string())
}

/// Army and general of the player in pending battle `view` (the aftermath is read for them).
pub fn player_army_of(state: &CampaignState, view: &PendingBattleView) -> Option<ArmyId> {
    let id = if view.player_side == "defender" {
        &view.defender
    } else {
        &view.attacker
    };
    let id = ArmyId::parse(id)?;
    state
        .army(&id)
        .filter(|a| a.faction == *state.player_faction())?;
    Some(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verdict_thresholds() {
        assert_eq!(verdict(0.97).0, "Victoire presque certaine");
        assert_eq!(verdict(0.5).1, "fair");
        assert_eq!(verdict(0.01).0, "Défaite presque certaine");
        assert_eq!(digits(12_345), "12\u{202f}345");
    }

    #[test]
    fn veterans_pair_in_order_skipping_lost_regiments() {
        let before = vec![
            ("a".to_owned(), 0),
            ("b".to_owned(), 1),
            ("c".to_owned(), 2),
        ];
        let after = vec![("a".to_owned(), 1), ("c".to_owned(), 2)];
        assert_eq!(veterans(&before, &after), 1);
        assert_eq!(veterans(&before, &[]), 0);
    }

    #[test]
    fn staged_battle_lists_launches_and_resolves() {
        let data = data_model::test_support::game_data();
        let mut state = sim_campaign::test_support::start_quiet(data, "fac_france", 1337);
        assert!(pending_battles(data, &state).is_empty());
        assert!(last_battle_outcome(&state).is_none());
        // Stage a battle between the player's army and any army at war with it.
        let player = state.player_faction().clone();
        let mine: Vec<ArmyId> = state
            .armies
            .iter()
            .filter(|(_, a)| a.faction == player)
            .map(|(id, _)| id.clone())
            .collect();
        let mut staged = None;
        'search: for a in &mine {
            for (d, army) in &state.armies {
                if army.faction != player && state.is_at_war(&player, &army.faction) {
                    let mut probe = state.clone();
                    if probe.debug_stage_battle(a, d).is_ok() {
                        staged = Some((a.clone(), d.clone()));
                        break 'search;
                    }
                }
            }
        }
        let (a, d) = staged.expect("an enemy army at war");
        let index = state.debug_stage_battle(&a, &d).expect("staged");
        let views = pending_battles(data, &state);
        assert_eq!(views.len(), 1);
        assert_eq!(views[0].index, index);
        assert!(views[0].attacker_strength > 0 && views[0].defender_strength > 0);
        let brief = battle_brief(data, &state, &views[0]).expect("brief");
        assert!(brief.title.contains("Bataille"), "{}", brief.title);
        assert_eq!(brief.sides[0].side, brief.player_side);
        assert!(brief.sides[0].regiments > 0 && brief.sides[1].regiments > 0);
        assert!(brief.chance.is_some() && !brief.verdict.is_empty());
        let launch = battle_launch(data, &state, index).expect("setup");
        assert!(!launch.setup.attacker.units.is_empty());
        assert!((0.0..24.0).contains(&launch.hour));
        assert!(battle_forecast(data, &state, index).is_ok());
        let army = player_army_of(&state, &views[0]);
        let before = aftermath_snapshot(data, &state, army.as_ref());
        let resolved = auto_resolve(data, &mut state, index).expect("auto");
        assert!(resolved.report.is_some());
        assert!(!resolved.events.is_empty());
        assert!(pending_battles(data, &state).is_empty());
        let outcome = last_battle_outcome(&state).expect("class");
        assert!(outcome.player_class.is_some());
        let after = aftermath_snapshot(data, &state, army.as_ref());
        let _ = aftermath_diff(&before, &after);
    }
}
