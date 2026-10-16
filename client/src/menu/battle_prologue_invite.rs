//! Invitation to the battle tutorial (game/scripts/ui/battle_prologue_invite.gd, NT4).
//!
//! The first time a battle is launched from the menu (custom or historical; demos are not gated,
//! as in Godot) the launch is held and « Jouer le didacticiel de bataille ? » is asked:
//! - « Oui » drops the held launch and sends [`StartBattlePrologue`];
//! - « Non » lets the held launch go (the invitation will come back);
//! - « Ne plus demander » lets it go and remembers the refusal (`never_ask`);
//! - Escape: neither the launch nor a remembered choice.
//!
//! No invitation either once the tutorial is finished (`done`) or when its data
//! (`data/tutorial/battle_prologue.json`) is missing.
//!
//! **Hook for the tutorial screen (lot bprologue)**: read [`StartBattlePrologue`] (a message, sent
//! in `AppState::MainMenu`) and open the guided skirmish; when it ends in victory call
//! [`PrologueInvite::mark_done`] with the prefs path ([`PrologueInvite::path`]) so the invitation
//! stops. Entering the same tutorial from a menu entry « Didacticiel de bataille » can send the
//! same message. Persistence: `battle_prologue.json` (`never_ask`, `done`) in the `Cent Ans`
//! folder (parent of the saves folder, or `CENT_ANS_PREFS`), because the `settings` plugin is not
//! this lot's. The campaign start keeps its own guide invitation (`faction_select`), so the
//! campaign launch is not gated twice (deviation from `start_menu.gd`, which gated both).

use std::path::PathBuf;

use bevy::ecs::message::MessageCursor;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::battles::{LaunchBattle, data_dir, prefs_dir};
use super::saves::SaveDir;
use super::{KeyEdges, spawn_overlay};
use crate::app::AppState;
use crate::ui::kit::style::UiType;
use crate::ui::kit::{ButtonPressed, UiTheme, button_small, typed};

/// The player accepted: the battle tutorial must start (no launch is held any more).
#[derive(Message, Debug, Clone, Copy, Default)]
pub struct StartBattlePrologue;

/// Remembered answers.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrologueFlags {
    #[serde(default)]
    pub never_ask: bool,
    #[serde(default)]
    pub done: bool,
}

impl PrologueFlags {
    /// Should the invitation be shown (tutorial data present, neither finished nor refused)?
    pub fn should_ask(&self, data_present: bool) -> bool {
        data_present && !self.never_ask && !self.done
    }

    fn load(path: &std::path::Path) -> Self {
        std::fs::read(path)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default()
    }

    fn save(&self, path: &std::path::Path) {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(text) = serde_json::to_string_pretty(self)
            && let Err(error) = std::fs::write(path, text)
        {
            warn!("battle_prologue.json: {error}");
        }
    }
}

/// The answer to the invitation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Choice {
    Yes,
    No,
    Never,
}

impl Choice {
    fn key(self) -> &'static str {
        match self {
            Self::Yes => "yes",
            Self::No => "no",
            Self::Never => "never",
        }
    }
}

/// State of the invitation.
#[derive(Resource, Default)]
pub struct PrologueInvite {
    pub flags: PrologueFlags,
    /// Where the flags are stored (`None` until the first launch).
    pub path: Option<PathBuf>,
    /// The question is on screen.
    pub asking: bool,
    /// The launch waiting for the answer.
    held: Option<LaunchBattle>,
    /// The next launch skips the gate (released after « Non » / « Ne plus demander »).
    bypass: bool,
    /// Answers given so far (dump).
    pub answers: Vec<&'static str>,
    /// Number of times `StartBattlePrologue` was sent (dump).
    pub starts_sent: u32,
}

impl PrologueInvite {
    /// Locates the flags file from the saves folder (when a launch did not go through the
    /// invitation, e.g. the menu entry) and reads it.
    pub fn ensure_path(&mut self, save_dir: &std::path::Path) {
        if self.path.is_none() {
            let path = prefs_dir(save_dir).join("battle_prologue.json");
            self.flags = PrologueFlags::load(&path);
            self.path = Some(path);
        }
    }

    /// Marks the tutorial as finished (called by the tutorial when the skirmish is won).
    pub fn mark_done(&mut self) {
        self.flags.done = true;
        if let Some(path) = &self.path {
            self.flags.save(path);
        }
    }

    /// Applies an answer: updates the flags and returns the launch to release, if any.
    pub fn answer(&mut self, choice: Choice) -> Option<LaunchBattle> {
        self.asking = false;
        self.answers.push(choice.key());
        if choice == Choice::Never {
            self.flags.never_ask = true;
            if let Some(path) = &self.path {
                self.flags.save(path);
            }
        }
        let held = self.held.take();
        match choice {
            Choice::Yes => {
                self.starts_sent += 1;
                None
            }
            Choice::No | Choice::Never => {
                self.bypass = held.is_some();
                held
            }
        }
    }
}

#[derive(Component)]
struct InviteRoot;

pub struct PrologueInvitePlugin;

impl Plugin for PrologueInvitePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PrologueInvite>()
            .add_message::<StartBattlePrologue>()
            .add_systems(
                Update,
                (invite_input, gate_launch)
                    .chain()
                    .before(super::battles::launch_battle)
                    .run_if(in_state(AppState::MainMenu)),
            );
    }
}

/// Intercepts [`LaunchBattle`] messages: lets them through, or holds the last one and asks.
/// Held messages are re-sent by [`invite_input`] with `bypass` set.
fn gate_launch(
    mut commands: Commands,
    mut launches: ResMut<Messages<LaunchBattle>>,
    mut cursor: Local<MessageCursor<LaunchBattle>>,
    mut invite: ResMut<PrologueInvite>,
    theme: Res<UiTheme>,
    dir: Option<Res<SaveDir>>,
) {
    let fresh: Vec<bool> = cursor
        .read(&launches)
        .map(|launch| launch.label.starts_with("demo:") || launch.label.starts_with("tutorial:"))
        .collect();
    if fresh.is_empty() {
        return;
    }
    if invite.bypass {
        invite.bypass = false;
        return;
    }
    if fresh.iter().all(|demo| *demo) {
        return;
    }
    if invite.path.is_none()
        && let Some(dir) = dir
    {
        let path = prefs_dir(&dir.0).join("battle_prologue.json");
        invite.flags = PrologueFlags::load(&path);
        invite.path = Some(path);
    }
    let present = data_dir().is_some_and(|dir| dir.join("tutorial/battle_prologue.json").is_file());
    if !invite.flags.should_ask(present) || invite.asking {
        return;
    }
    let held: Vec<LaunchBattle> = launches.drain().collect();
    invite.held = held.into_iter().last();
    invite.asking = true;
    spawn_invite(&mut commands, &theme);
}

fn spawn_invite(commands: &mut Commands, theme: &UiTheme) {
    let (root, panel) = spawn_overlay(
        commands,
        theme,
        "BattlePrologueInvite",
        "Jouer le didacticiel de bataille ?",
        560.0,
        1,
    );
    commands.entity(root).insert(InviteRoot);
    commands.spawn((
        typed(
            theme,
            "Une courte escarmouche guidée de 1337 apprend à mener une bataille : caméra, ordres, \
             formation, charge, tir et pause tactique. On la retrouve dans « Batailles historiques ».",
            UiType::Body,
        ),
        ChildOf(panel),
    ));
    let row = commands
        .spawn((
            Node {
                column_gap: Val::Px(10.0),
                justify_content: JustifyContent::End,
                flex_wrap: FlexWrap::Wrap,
                ..default()
            },
            ChildOf(panel),
        ))
        .id();
    for (name, caption) in [
        ("prologue_invite:never", "Ne plus demander"),
        ("prologue_invite:no", "Non"),
        ("prologue_invite:yes", "Oui"),
    ] {
        commands.spawn((button_small(theme, name, caption), ChildOf(row)));
    }
}

/// Buttons, Enter (= « Oui ») and Escape (the overlay is closed by the menu: neither launch nor
/// remembered choice).
fn invite_input(
    mut commands: Commands,
    mut pressed: MessageReader<ButtonPressed>,
    edges: Res<KeyEdges>,
    mut invite: ResMut<PrologueInvite>,
    roots: Query<Entity, With<InviteRoot>>,
    mut launches: MessageWriter<LaunchBattle>,
    mut starts: MessageWriter<StartBattlePrologue>,
) {
    if !invite.asking {
        pressed.clear();
        return;
    }
    let mut choice = pressed.read().find_map(|press| match press.name.as_str() {
        "prologue_invite:yes" => Some(Choice::Yes),
        "prologue_invite:no" => Some(Choice::No),
        "prologue_invite:never" => Some(Choice::Never),
        _ => None,
    });
    if choice.is_none() && edges.enter {
        choice = Some(Choice::Yes);
    }
    let Some(choice) = choice else {
        // Escape closed the overlay: drop the held launch, remember nothing.
        if roots.is_empty() {
            invite.asking = false;
            invite.held = None;
            invite.answers.push("cancel");
        }
        return;
    };
    for root in &roots {
        commands.entity(root).despawn();
    }
    if let Some(launch) = invite.answer(choice) {
        launches.write(launch);
    }
    if choice == Choice::Yes {
        starts.write(StartBattlePrologue);
    }
}

/// Section `menu.battle_prologue_invite`.
pub fn dump(world: &World) -> Value {
    let Some(invite) = world.get_resource::<PrologueInvite>() else {
        return json!({});
    };
    json!({
        "asking": invite.asking,
        "held": invite.held.as_ref().map(|l| l.label.clone()),
        "never_ask": invite.flags.never_ask,
        "done": invite.flags.done,
        "answers": invite.answers,
        "starts_sent": invite.starts_sent,
        "path": invite.path.as_ref().map(|p| p.display().to_string()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use sim_battle::custom::{CustomBattle, CustomSide};

    fn launch(label: &str) -> LaunchBattle {
        let side = |faction: &str| CustomSide {
            faction: faction.to_owned(),
            budget: 0,
            units: Vec::new(),
            technologies: Vec::new(),
        };
        let battle = CustomBattle {
            attacker: side("fac_france"),
            defender: side("fac_england"),
            terrain: String::new(),
            season: String::new(),
            weather: String::new(),
            hour: String::new(),
            siege: false,
            fortification: None,
            player_side: "attacker".to_owned(),
            year: None,
            engines: None,
        };
        LaunchBattle::new(battle, 1, label)
    }

    #[test]
    fn no_invitation_when_done_refused_or_without_data() {
        let mut flags = PrologueFlags::default();
        assert!(flags.should_ask(true));
        assert!(!flags.should_ask(false));
        flags.never_ask = true;
        assert!(!flags.should_ask(true));
        flags = PrologueFlags {
            never_ask: false,
            done: true,
        };
        assert!(!flags.should_ask(true));
    }

    #[test]
    fn no_proceeds_and_asks_again_never_proceeds_and_remembers_yes_drops() {
        let mut invite = PrologueInvite {
            held: Some(launch("custom")),
            asking: true,
            ..Default::default()
        };
        let released = invite.answer(Choice::No);
        assert_eq!(released.map(|l| l.label), Some("custom".to_owned()));
        assert!(invite.bypass && !invite.flags.never_ask && !invite.asking);

        invite.held = Some(launch("custom"));
        let released = invite.answer(Choice::Never);
        assert!(released.is_some() && invite.flags.never_ask);

        invite.held = Some(launch("custom"));
        let released = invite.answer(Choice::Yes);
        assert!(released.is_none());
        assert_eq!(invite.starts_sent, 1);
        assert_eq!(invite.answers, ["no", "never", "yes"]);
    }

    #[test]
    fn the_flags_are_stored_in_the_prefs_folder() {
        let dir = std::env::temp_dir().join(format!("cent_ans_prologue_{}", std::process::id()));
        let path = dir.join("battle_prologue.json");
        let mut invite = PrologueInvite {
            path: Some(path.clone()),
            ..Default::default()
        };
        invite.mark_done();
        assert!(PrologueFlags::load(&path).done);
        let _ = std::fs::remove_dir_all(dir);
    }
}
