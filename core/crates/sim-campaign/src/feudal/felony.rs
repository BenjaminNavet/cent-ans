//! Felony and forfeiture (lot F3, spec § 4.4). A refused host, an alliance
//! with the suzerain's enemy or a revolt opens a felony case for
//! `feudal_rules.felony_window_turns` turns; the suzerain may then declare
//! forfeiture, a casus belli against the felon alone; at the peace, the
//! forfeited titles go to the suzerain if it won.
//!
//! Call points for the other lots: [`on_host_refused`] (F1, `rally_vassals`),
//! [`open_felony_towards`] with [`FelonyReason::Revolt`] (vassal revolt in
//! `diplomacy::resolve_diplomacy`), [`has_forfeiture`] (casus belli),
//! [`settle_forfeitures`] (`make_peace_between`). Alliances with an enemy
//! of the suzerain are detected each turn by `resolve_feudal`.

use data_model::{FactionId, GameData, TitleId};

use super::{holder_of, title_vassals, FelonyCase, FelonyReason, FeudalError, Forfeiture};
use crate::events::{EventKind, GameEvent};
use crate::state::CampaignState;

fn reason_text(reason: FelonyReason) -> &'static str {
    match reason {
        FelonyReason::RefusedHost => "refus de l'ost",
        FelonyReason::AlliedWithEnemy => "alliance avec l'ennemi de son suzerain",
        FelonyReason::Revolt => "révolte",
        FelonyReason::HarbouredFelon => "asile donné à un banni du royaume",
    }
}

/// Titles `vassal` holds directly of a title held by `liege`.
pub(super) fn titles_held_of(
    state: &CampaignState,
    data: &GameData,
    vassal: &FactionId,
    liege: &FactionId,
) -> Vec<TitleId> {
    state
        .feudal
        .holders
        .iter()
        .filter(|(_, holder)| *holder == vassal)
        .filter(|(title, _)| {
            data.titles
                .get(*title)
                .and_then(|t| t.de_jure_liege.as_ref())
                .and_then(|l| holder_of(state, l))
                == Some(liege)
        })
        .map(|(title, _)| title.clone())
        .collect()
}

/// Opens (or prolongs) a felony case of `vassal` towards `liege`, which must
/// hold a title directly above one of `vassal`'s (the double allegiance of
/// England through Guyenne counts). `None` otherwise.
pub fn open_felony_towards(
    state: &mut CampaignState,
    data: &GameData,
    vassal: &FactionId,
    liege: &FactionId,
    reason: FelonyReason,
) -> Option<FelonyCase> {
    if vassal == liege || !title_vassals(state, data, liege).contains(vassal) {
        return None;
    }
    let expires_turn = state.turn + data.feudal_rules.felony_window_turns;
    if let Some(case) = state
        .feudal
        .felonies
        .iter_mut()
        .find(|c| &c.vassal == vassal && &c.liege == liege)
    {
        case.expires_turn = case.expires_turn.max(expires_turn);
        case.reason = reason;
        return Some(case.clone());
    }
    let case = FelonyCase {
        vassal: vassal.clone(),
        liege: liege.clone(),
        reason,
        expires_turn,
    };
    state.feudal.felonies.push(case.clone());
    state.push_order_event(
        GameEvent::new(
            EventKind::Vassalage,
            format!(
                "Félonie de {} envers {} ({}) : la commise peut être prononcée.",
                data.faction_name(vassal),
                data.faction_name(liege),
                reason_text(reason)
            ),
        )
        .faction(liege),
    );
    Some(case)
}

/// Call point for F1: `vassal` refused the host of `liege`.
pub fn on_host_refused(
    state: &mut CampaignState,
    data: &GameData,
    vassal: &FactionId,
    liege: &FactionId,
) -> Option<FelonyCase> {
    open_felony_towards(state, data, vassal, liege, FelonyReason::RefusedHost)
}

/// Call point: `vassal` revolted against `liege`.
pub fn on_revolt(
    state: &mut CampaignState,
    data: &GameData,
    vassal: &FactionId,
    liege: &FactionId,
) -> Option<FelonyCase> {
    open_felony_towards(state, data, vassal, liege, FelonyReason::Revolt)
}

impl CampaignState {
    /// [`settle_forfeitures`] from `make_peace_between`, its events queued
    /// for the journal.
    pub(crate) fn settle_forfeitures_at_peace(
        &mut self,
        data: &GameData,
        a: &FactionId,
        b: &FactionId,
    ) {
        if self.feudal.forfeitures.is_empty() {
            return;
        }
        let mut events = Vec::new();
        settle_forfeitures(self, data, a, b, &mut events);
        for event in events {
            self.push_order_event(event);
        }
    }
}

/// `true` while `liege` pursues a forfeiture against `vassal` (casus belli).
pub fn has_forfeiture(state: &CampaignState, liege: &FactionId, vassal: &FactionId) -> bool {
    state
        .feudal
        .forfeitures
        .iter()
        .any(|f| &f.liege == liege && &f.vassal == vassal)
}

/// See [`super::declare_commise`].
pub(super) fn declare_commise(
    state: &mut CampaignState,
    data: &GameData,
    liege: &FactionId,
    vassal: &FactionId,
) -> Result<(), FeudalError> {
    let turn = state.turn;
    let Some(index) = state
        .feudal
        .felonies
        .iter()
        .position(|c| &c.vassal == vassal && &c.liege == liege && c.expires_turn >= turn)
    else {
        return Err(FeudalError::NoFelonyCase(vassal.clone()));
    };
    let titles = titles_held_of(state, data, vassal, liege);
    if titles.is_empty() {
        return Err(FeudalError::NotVassal(vassal.clone(), liege.clone()));
    }
    state.feudal.felonies.remove(index);
    state.feudal.forfeitures.push(Forfeiture {
        liege: liege.clone(),
        vassal: vassal.clone(),
        titles: titles.clone(),
        declared_turn: turn,
    });
    let names: Vec<String> = titles
        .iter()
        .map(|t| {
            data.titles
                .get(t)
                .map_or_else(|| t.to_string(), |d| d.name.display.clone())
        })
        .collect();
    state.push_order_event(
        GameEvent::new(
            EventKind::Vassalage,
            format!(
                "{} prononce la commise contre {} : {} confisqué.",
                data.faction_name(liege),
                data.faction_name(vassal),
                names.join(", ")
            ),
        )
        .faction(liege),
    );
    super::record_peer_forfeiture(state, data, liege, vassal);
    if !state.is_at_war(liege, vassal) {
        if let Err(error) = state.declare_war(data, liege, vassal) {
            state
                .feudal
                .forfeitures
                .retain(|f| !(&f.liege == liege && &f.vassal == vassal));
            return Err(FeudalError::War(error.to_string()));
        }
    }
    Ok(())
}

/// Settles the forfeitures between `a` and `b` at their peace (called by
/// `make_peace_between` before the war score is cleared): the suzerain
/// seizes the forfeited titles still held by the felon if its war score
/// reaches `feudal_rules.forfeiture_win_war_score`; otherwise the
/// forfeiture lapses.
pub fn settle_forfeitures(
    state: &mut CampaignState,
    data: &GameData,
    a: &FactionId,
    b: &FactionId,
    events: &mut Vec<GameEvent>,
) {
    let (settled, kept): (Vec<Forfeiture>, Vec<Forfeiture>) =
        std::mem::take(&mut state.feudal.forfeitures)
            .into_iter()
            .partition(|f| (&f.liege == a && &f.vassal == b) || (&f.liege == b && &f.vassal == a));
    state.feudal.forfeitures = kept;
    for forfeiture in settled {
        let (liege, vassal) = (&forfeiture.liege, &forfeiture.vassal);
        let score = state.war_score(data, liege, vassal);
        let won = score >= data.feudal_rules.forfeiture_win_war_score
            && state.factions.get(liege).is_some_and(|f| f.alive);
        if won {
            for title in &forfeiture.titles {
                if holder_of(state, title) == Some(vassal) {
                    let _ = super::transfer::transfer(state, data, title, liege, events);
                }
            }
            events.push(
                GameEvent::new(
                    EventKind::Vassalage,
                    format!(
                        "La commise est exécutée : {} reprend les fiefs de {}.",
                        data.faction_name(liege),
                        data.faction_name(vassal)
                    ),
                )
                .faction(liege),
            );
        } else {
            events.push(
                GameEvent::new(
                    EventKind::Vassalage,
                    format!(
                        "La commise prononcée par {} contre {} reste lettre morte.",
                        data.faction_name(liege),
                        data.faction_name(vassal)
                    ),
                )
                .faction(vassal),
            );
        }
    }
}

/// Drops expired felony cases.
pub(super) fn expire_felonies(state: &mut CampaignState) {
    let turn = state.turn;
    state.feudal.felonies.retain(|c| c.expires_turn >= turn);
}

/// Opens a case for every title vassal allied with a faction at war with
/// its liege (while not itself at war with it).
pub(super) fn detect_enemy_alliances(state: &mut CampaignState, data: &GameData) {
    let mut pairs: Vec<(FactionId, FactionId)> = Vec::new();
    for (title, vassal) in &state.feudal.holders {
        let Some(liege) = data
            .titles
            .get(title)
            .and_then(|t| t.de_jure_liege.as_ref())
            .and_then(|l| holder_of(state, l))
        else {
            continue;
        };
        if liege == vassal || pairs.iter().any(|(v, l)| v == vassal && l == liege) {
            continue;
        }
        let Some(v) = state.factions.get(vassal).filter(|f| f.alive) else {
            continue;
        };
        if state.is_at_war(vassal, liege) {
            continue;
        }
        let traitor = v
            .allies
            .iter()
            .any(|ally| ally != liege && state.is_at_war(ally, liege));
        if traitor {
            pairs.push((vassal.clone(), liege.clone()));
        }
    }
    for (vassal, liege) in pairs {
        let open = state
            .feudal
            .felonies
            .iter()
            .any(|c| c.vassal == vassal && c.liege == liege);
        if !open {
            open_felony_towards(state, data, &vassal, &liege, FelonyReason::AlliedWithEnemy);
        }
    }
}
