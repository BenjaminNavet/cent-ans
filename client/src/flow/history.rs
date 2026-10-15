//! Battles played this session: the consumer of the result screen's [`ResultExtras`].
//!
//! Each battle left ([`LeaveBattle`]) adds an entry with its title, outcome, the « suites » of the
//! campaign ([`ResultExtras::aftermath`], as the lines of the return notice) and, for a historical
//! battle, whether History repeated ([`ResultExtras::history`]). Bounded to [`MAX_ENTRIES`], newest
//! last; read by the dump (`flow.played_battles`). Not saved with the game: the persistent battle
//! history is the core's (`CampaignState::battle_history`, scars of the map), which has no field
//! for the client's extras.

use bevy::prelude::*;
use serde::Serialize;
use sim_battle::{BattleOutcome, SideId};

use super::LeaveBattle;
use super::notice;
use crate::battle::hud::aftermath::ResultExtras;
use crate::battle::scene::BattleSession;

pub const MAX_ENTRIES: usize = 24;

/// The historical battle an entry replayed.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct HistoryRecord {
    pub name: String,
    pub date_fr: String,
    /// The historical winner won again.
    pub repeated: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PlayedBattle {
    pub title: String,
    pub abandoned: bool,
    /// `attacker` / `defender`, none when the battle did not end.
    pub winner: Option<&'static str>,
    pub player_won: Option<bool>,
    pub aftermath_lines: Vec<String>,
    pub history: Option<HistoryRecord>,
}

#[derive(Resource, Debug, Default, Clone)]
pub struct PlayedBattles(pub Vec<PlayedBattle>);

impl PlayedBattles {
    pub fn push(&mut self, entry: PlayedBattle) {
        self.0.push(entry);
        let excess = self.0.len().saturating_sub(MAX_ENTRIES);
        self.0.drain(..excess);
    }
}

fn side_key(side: SideId) -> &'static str {
    match side {
        SideId::Attacker => "attacker",
        SideId::Defender => "defender",
    }
}

/// The entry of a battle left (pure).
pub fn entry(
    title: &str,
    abandoned: bool,
    outcome: Option<&BattleOutcome>,
    player: Option<SideId>,
    extras: &ResultExtras,
) -> PlayedBattle {
    let winner = outcome.map(|o| o.winner);
    PlayedBattle {
        title: title.to_owned(),
        abandoned,
        winner: winner.map(side_key),
        player_won: winner.zip(player).map(|(w, p)| w == p),
        aftermath_lines: extras
            .aftermath
            .as_ref()
            .map(notice::aftermath_lines)
            .unwrap_or_default(),
        history: extras.history.as_ref().map(|h| HistoryRecord {
            name: h.name.clone(),
            date_fr: h.date_fr.clone(),
            repeated: winner == Some(h.historical_winner),
        }),
    }
}

/// Runs before `battles::leave_battle` (which forgets the campaign battle).
pub fn record_played_battle(
    mut leaves: MessageReader<LeaveBattle>,
    mut session: Option<ResMut<BattleSession>>,
    extras: Res<ResultExtras>,
    fader: Res<crate::ui::fader::Fader>,
    mut log: ResMut<PlayedBattles>,
) {
    let Some(leave) = leaves.read().last().copied() else {
        return;
    };
    if fader.busy() {
        return; // second click during the fade-out: already recorded
    }
    let Some(session) = session.as_mut() else {
        return;
    };
    if session.replaying {
        return;
    }
    let player = session.player;
    let title = session.title.clone();
    let battle = session.sim_mut();
    let outcome = if battle.is_finished() {
        battle.outcome()
    } else {
        None
    };
    log.push(entry(
        &title,
        leave.abandoned,
        outcome.as_ref(),
        player,
        &extras,
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::battle::hud::aftermath::HistoryInfo;

    fn played(n: usize) -> PlayedBattle {
        PlayedBattle {
            title: format!("b{n}"),
            abandoned: false,
            winner: None,
            player_won: None,
            aftermath_lines: Vec::new(),
            history: None,
        }
    }

    #[test]
    fn bounded_newest_last() {
        let mut log = PlayedBattles::default();
        for n in 0..MAX_ENTRIES + 3 {
            log.push(played(n));
        }
        assert_eq!(log.0.len(), MAX_ENTRIES);
        assert_eq!(log.0[0].title, "b3");
        assert_eq!(log.0.last().unwrap().title, format!("b{}", MAX_ENTRIES + 2));
    }

    #[test]
    fn history_of_an_unfinished_battle_does_not_repeat() {
        let extras = ResultExtras {
            aftermath: None,
            history: Some(HistoryInfo {
                name: "Crécy".into(),
                date_fr: "26 août 1346".into(),
                historical_winner: SideId::Attacker,
                faction_names: ["Angleterre".into(), "France".into()],
            }),
        };
        let entry = entry("Crécy", true, None, Some(SideId::Defender), &extras);
        assert!(entry.abandoned && entry.winner.is_none() && entry.player_won.is_none());
        let history = entry.history.expect("history");
        assert_eq!(history.name, "Crécy");
        assert!(!history.repeated);
    }
}
