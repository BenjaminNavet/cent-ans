//! Battles played this session: the consumer of the result screen's [`ResultExtras`].
//!
//! Each battle left ([`LeaveBattle`]) adds an entry with its title, outcome, the « suites » of the
//! campaign ([`ResultExtras::aftermath`], as the lines of the return notice) and, for a historical
//! battle, whether History repeated ([`ResultExtras::history`]). Bounded to [`MAX_ENTRIES`], newest
//! last; read by the dump (`flow.played_battles`) and listed by the « Batailles jouées » overlay
//! (pause menu). Saved with the game as a sidecar `<save>.played` (the core's persistent
//! `CampaignState::battle_history` has no field for the client's extras): [`write_sidecar`] /
//! [`read_sidecar`] are called by `menu::saves::{save_now, load_into}` through a process-wide
//! mirror ([`MIRROR`]) that [`PlayedBattles::publish`] keeps current; [`sync_after_load`] copies it
//! back into the resource, a new game ([`clear_for_new_game`]) empties both.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use sim_battle::{BattleOutcome, SideId};

use super::LeaveBattle;
use super::notice;
use crate::battle::hud::aftermath::ResultExtras;
use crate::battle::scene::BattleSession;

pub const MAX_ENTRIES: usize = 24;

/// The historical battle an entry replayed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HistoryRecord {
    pub name: String,
    pub date_fr: String,
    /// The historical winner won again.
    pub repeated: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayedBattle {
    pub title: String,
    pub abandoned: bool,
    /// `attacker` / `defender`, none when the battle did not end.
    pub winner: Option<String>,
    pub player_won: Option<bool>,
    pub aftermath_lines: Vec<String>,
    pub history: Option<HistoryRecord>,
}

#[derive(Resource, Debug, Default, Clone)]
pub struct PlayedBattles(pub Vec<PlayedBattle>);

/// What `save_now` writes: the session's log, whatever system calls it (quick save, autosave).
static MIRROR: std::sync::Mutex<Vec<PlayedBattle>> = std::sync::Mutex::new(Vec::new());

fn mirror() -> std::sync::MutexGuard<'static, Vec<PlayedBattle>> {
    MIRROR
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Sidecar of the save file `save_path` (`x.json` → `x.played`; not a `.json`, so the slot list
/// ignores it).
pub fn sidecar_path(save_path: &std::path::Path) -> std::path::PathBuf {
    save_path.with_extension("played")
}

/// Writes the log next to the save (best effort: a failure only loses the history).
pub fn write_sidecar(save_path: &std::path::Path) {
    let log = mirror().clone();
    let path = sidecar_path(save_path);
    match serde_json::to_string(&log) {
        Ok(text) if !log.is_empty() => {
            if let Err(error) = std::fs::write(&path, text) {
                warn!("played battles: cannot write {}: {error}", path.display());
            }
        }
        Ok(_) => {
            let _ = std::fs::remove_file(&path); // no stale log from an older save of the slot
        }
        Err(error) => warn!("played battles: {error}"),
    }
}

/// Reads the sidecar of the save just loaded into the mirror (empty when absent or unreadable).
pub fn read_sidecar(save_path: &std::path::Path) {
    let log: Vec<PlayedBattle> = std::fs::read_to_string(sidecar_path(save_path))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default();
    *mirror() = log;
}

/// Empties the log (new game).
pub fn clear_for_new_game(mut log: ResMut<PlayedBattles>) {
    log.0.clear();
    mirror().clear();
}

/// The save loaded replaced the log: the resource follows the mirror.
pub fn sync_after_load(
    mut loaded: MessageReader<crate::menu::saves::GameLoaded>,
    mut log: ResMut<PlayedBattles>,
) {
    if loaded.read().last().is_some() {
        log.0.clone_from(&mirror());
    }
}

impl PlayedBattles {
    pub fn push(&mut self, entry: PlayedBattle) {
        self.0.push(entry);
        let excess = self.0.len().saturating_sub(MAX_ENTRIES);
        self.0.drain(..excess);
    }

    /// Makes the next `save_now` write this log.
    pub fn publish(&self) {
        mirror().clone_from(&self.0);
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
        winner: winner.map(|w| side_key(w).to_owned()),
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
    log.publish();
}

/// Asks to open the « Batailles jouées » overlay (pause menu button).
#[derive(Message, Debug, Clone, Copy)]
pub struct OpenPlayedBattles;

/// How many times the overlay was opened (dump).
#[derive(Resource, Default, Debug)]
pub struct OverlayStats {
    pub opened: u32,
}

/// The overlay root.
#[derive(Component)]
pub struct PlayedBattlesRoot;

/// One list line per battle: title and outcome (pure).
pub fn headline(entry: &PlayedBattle) -> String {
    let outcome = if entry.abandoned {
        "abandonnée"
    } else {
        match entry.player_won {
            Some(true) => "victoire",
            Some(false) => "défaite",
            None => "issue inconnue",
        }
    };
    format!("{} — {outcome}", entry.title)
}

/// Detail lines of an entry: History repeated or reversed, then the campaign's « suites » (pure).
pub fn details(entry: &PlayedBattle) -> Vec<String> {
    let mut lines = Vec::new();
    if let Some(history) = &entry.history {
        lines.push(format!(
            "{} ({}) : {}.",
            history.name,
            history.date_fr,
            if history.repeated {
                "l’Histoire se répète"
            } else {
                "l’Histoire est renversée"
            }
        ));
    }
    lines.extend(entry.aftermath_lines.iter().cloned());
    lines
}

pub fn open_overlay(
    mut commands: Commands,
    mut requests: MessageReader<OpenPlayedBattles>,
    theme: Res<crate::ui::kit::UiTheme>,
    log: Res<PlayedBattles>,
    open: Query<(), With<PlayedBattlesRoot>>,
    mut stats: ResMut<OverlayStats>,
) {
    use crate::ui::kit::style::UiType;
    use crate::ui::kit::{button_small, scroll_list, typed};
    if requests.read().count() == 0 || !open.is_empty() {
        return;
    }
    stats.opened += 1;
    let (root, panel) = crate::menu::spawn_overlay(
        &mut commands,
        &theme,
        "PlayedBattles",
        "Batailles jouées",
        560.0,
        1,
    );
    commands.entity(root).insert(PlayedBattlesRoot);
    if log.0.is_empty() {
        commands.spawn((
            typed(
                &theme,
                "Aucune bataille jouée dans cette partie.",
                UiType::Body,
            ),
            ChildOf(panel),
        ));
    } else {
        let list = commands
            .spawn((scroll_list(&theme, "played:list", 320.0), ChildOf(panel)))
            .id();
        for entry in log.0.iter().rev() {
            commands.spawn((
                typed(&theme, &headline(entry), UiType::Heading),
                ChildOf(list),
            ));
            for line in details(entry) {
                commands.spawn((typed(&theme, &line, UiType::Caption), ChildOf(list)));
            }
        }
    }
    commands.spawn((
        button_small(&theme, "played:close", "Fermer"),
        Node {
            align_self: AlignSelf::End,
            ..default()
        },
        ChildOf(panel),
    ));
}

pub fn close_overlay(
    mut commands: Commands,
    mut pressed: MessageReader<crate::ui::kit::ButtonPressed>,
    roots: Query<Entity, With<PlayedBattlesRoot>>,
) {
    if pressed.read().any(|press| press.name == "played:close") {
        for root in &roots {
            commands.entity(root).despawn();
        }
    }
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
    fn headline_and_details_read_in_french() {
        let mut entry = played(1);
        entry.title = "Crécy".into();
        entry.player_won = Some(false);
        entry.aftermath_lines = vec!["Général X : +3 expérience.".into()];
        entry.history = Some(HistoryRecord {
            name: "Crécy".into(),
            date_fr: "26 août 1346".into(),
            repeated: false,
        });
        assert_eq!(headline(&entry), "Crécy — défaite");
        let lines = details(&entry);
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("renversée"));
        entry.abandoned = true;
        assert!(headline(&entry).ends_with("abandonnée"));
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

#[cfg(test)]
mod sidecar_tests {
    use super::*;

    #[test]
    fn the_log_travels_with_the_save() {
        let dir = std::env::temp_dir().join(format!("cent-ans-played-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let save = dir.join("slot.json");
        let mut log = PlayedBattles::default();
        log.push(PlayedBattle {
            title: "Crécy".into(),
            abandoned: false,
            winner: Some("attacker".into()),
            player_won: Some(true),
            aftermath_lines: vec!["+1 expérience".into()],
            history: Some(HistoryRecord {
                name: "Crécy".into(),
                date_fr: "26 août 1346".into(),
                repeated: true,
            }),
        });
        log.publish();
        write_sidecar(&save);
        assert!(sidecar_path(&save).ends_with("slot.played"));
        mirror().clear();
        read_sidecar(&save);
        assert_eq!(*mirror(), log.0);
        // Overwriting the slot with an empty log removes the sidecar.
        mirror().clear();
        write_sidecar(&save);
        assert!(!sidecar_path(&save).exists());
        read_sidecar(&save);
        assert!(mirror().is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
