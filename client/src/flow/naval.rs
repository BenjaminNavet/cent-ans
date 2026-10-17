//! Pre-battle dialog of a naval interception (`naval/naval_pre_battle_dialog.gd`).
//!
//! Naval battles are never played in 3D: the player sees the fleets and the core's estimate, then
//! « Résolution automatique » or « Rentrer au port » / « Refuser le combat » (the intercepted fleet
//! goes back to port). Rules are the core's (`CampaignState::{pending_naval_views,
//! naval_battle_setup, auto_resolve_naval_battle, withdraw_naval_battle}`); the dialog is the
//! same presentation as the ground one (`dialog`): « Flotte ennemie en vue », chances (five
//! automatic resolutions), ships and crews of both fleets, season and rain.
//!
//! Illustrated banner ("naval" plate, Sluys miniature fallback) and the arms of both fleets come
//! from `dialog::{banner, side_header}`, shared with the ground dialog.

use bevy::prelude::*;
use bevy::state::state_scoped::DespawnOnExit;
use game_api::flow::verdict;
use sim_battle::SideId;
use sim_battle::naval::{NavalSetup, NavalSideSetup};
use sim_campaign::naval::PendingNavalBattle;

use super::battles::autosave_before_battle;
use super::{BattleResolved, FlowState, ResolvedKind, Stage};
use crate::app::AppState;
use crate::battle::scene::setup::side_color;
use crate::campaign::hud::HudData;
use crate::campaign::hud::flow::TurnFlow;
use crate::menu::saves::SaveDir;
use crate::settings::Settings;
use crate::sim::Sim;
use crate::ui::kit::modal::{ModalQueue, ModalSpec, PRIORITY_DECISION, ToastRequest};
use crate::ui::kit::style::{self, UiTier, UiType};
use crate::ui::kit::widgets::{button_primary, button_small, caption, panel_sized, separator};
use crate::ui::kit::{ButtonPressed, UiTheme, fr, typed};

/// Whether the pre-battle dialog takes the pending naval battles (set when the flow plugin is
/// built; `CENT_ANS_NAVAL_AUTO=1` keeps the old « resolve at once » for scripts). The armies'
/// auto-resolution (`campaign::armies::sea::auto_resolve_pending`) stands back while it is on.
static OFFER_DIALOG: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Called by the flow plugin: from now on the player decides.
pub fn enable_dialog() {
    let auto = std::env::var("CENT_ANS_NAVAL_AUTO").is_ok_and(|v| v != "0");
    OFFER_DIALOG.store(!auto, std::sync::atomic::Ordering::Relaxed);
}

/// True when pending naval battles wait for the dialog instead of being resolved at once.
pub fn offers_dialog() -> bool {
    OFFER_DIALOG.load(std::sync::atomic::Ordering::Relaxed)
}

pub const AUTO: &str = "flow:naval:auto";
pub const WITHDRAW: &str = "flow:naval:withdraw";
const RESULT_KEY: &str = "flow:naval_result";

/// The open dialog and what it shows (read by the dump and the button handler).
#[derive(Component, Debug, Clone)]
pub struct NavalDialog {
    pub index: usize,
    pub title: String,
    pub verdict: String,
    pub share: f64,
    pub chance: f64,
    pub ships: [usize; 2],
    pub men: [u32; 2],
    pub withdraw_label: String,
}

fn season_fr(season: sim_battle::BattleSeason) -> &'static str {
    use sim_battle::BattleSeason::*;
    match season {
        Spring => "printemps",
        Summer => "été",
        Autumn => "automne",
        Winter => "hiver",
    }
}

fn tone_color(tone: &str) -> Color {
    match tone {
        "good" => style::GOOD,
        "fair" => Color::srgb(0.55, 0.40, 0.08),
        _ => style::RUBRIC,
    }
}

/// Ships of a class, plural by « s » as the Godot dialog does: « 3 nefs, 1 galère ».
fn fleet_line(side: &NavalSideSetup) -> String {
    let mut classes: Vec<(String, usize)> = Vec::new();
    for ship in &side.ships {
        let name = ship.class.name.display.to_lowercase();
        match classes.iter_mut().find(|(n, _)| *n == name) {
            Some((_, count)) => *count += 1,
            None => classes.push((name, 1)),
        }
    }
    let parts: Vec<String> = classes
        .iter()
        .map(|(name, count)| format!("{count} {name}{}", if *count > 1 { "s" } else { "" }))
        .collect();
    format!("{} navires : {}", side.ships.len(), parts.join(", "))
}

/// Lines of the dialog for the player's side then the enemy's (`[own, foe]`).
fn sides_of(setup: &NavalSetup, player: SideId) -> [(&NavalSideSetup, SideId); 2] {
    match player {
        SideId::Attacker => [
            (&setup.attacker, SideId::Attacker),
            (&setup.defender, SideId::Defender),
        ],
        SideId::Defender => [
            (&setup.defender, SideId::Defender),
            (&setup.attacker, SideId::Attacker),
        ],
    }
}

/// « Flotte ennemie en vue » for the first pending interception when nothing else is on screen.
#[allow(clippy::too_many_arguments)]
pub fn offer_pending(
    mut commands: Commands,
    sim: Res<Sim>,
    theme: Res<UiTheme>,
    queue: Res<ModalQueue>,
    turn: Res<TurnFlow>,
    mut flow: ResMut<FlowState>,
    dialogs: Query<(Entity, &NavalDialog)>,
    ground: Query<(), With<super::dialog::PreBattleDialog>>,
    settings: Res<Settings>,
    save_dir: Option<Res<SaveDir>>,
    assets: Res<AssetServer>,
    plates: Res<crate::ui::art_plates::ArtPlates>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
) {
    let (Some(data), Some(state)) = (sim.data.as_ref(), sim.state.as_ref()) else {
        return;
    };
    let pending = state.pending_naval_views(data);
    for (entity, open) in &dialogs {
        if !pending.iter().any(|p| p.index == open.index) {
            commands.entity(entity).despawn();
        }
    }
    if !dialogs.is_empty()
        || !ground.is_empty()
        || flow.stage != Stage::Idle
        || turn.running()
        || queue.is_busy()
    {
        return;
    }
    // A ground battle waits first.
    if !game_api::flow::pending_battles(data, state).is_empty() {
        return;
    }
    let Some(view) = pending.first() else {
        return;
    };
    match state.naval_battle_setup(data, view.index) {
        Ok(setup) => {
            flow.stats.naval_dialogs += 1;
            autosave_before_battle(&settings, save_dir.as_deref(), &sim, &mut flow);
            let art = super::dialog::DialogArt {
                assets: &assets,
                plates: &plates,
                window_height: super::loading::window_height(&windows),
            };
            spawn(&mut commands, &theme, &art, data, view, &setup);
        }
        Err(error) => {
            let message = error.to_string();
            if flow.errors.last() != Some(&message) {
                warn!("naval dialog: {message}");
                flow.errors.push(message);
            }
        }
    }
}

fn spawn(
    commands: &mut Commands,
    theme: &UiTheme,
    art: &super::dialog::DialogArt,
    game: &data_model::GameData,
    view: &PendingNavalBattle,
    setup: &NavalSetup,
) {
    let player = view.player_side;
    let sides = sides_of(setup, player);
    let (text, tone) = verdict(view.win_chance);
    // One estimate for the verdict, the percentage and the balance bar (the forecast of core).
    let share = view.win_chance;
    let waters = if setup.place_name.is_empty() {
        view.sea_name.clone()
    } else {
        setup.place_name.clone()
    };
    let title = format!("Flotte ennemie en vue : {waters}");
    let subtitle = format!(
        "{} barre la traversée de {} · {}",
        view.interceptor_name,
        view.faction_name,
        season_fr(setup.season)
    );
    let withdraw_label = if player == SideId::Defender {
        "Rentrer au port"
    } else {
        "Refuser le combat"
    };
    let colors: [Color; 2] = [0, 1].map(|slot| {
        let (side, id) = sides[slot];
        let [r, g, b] = side_color(game, &side.faction, id);
        Color::srgb(r, g, b)
    });
    let colors = if colors[0] == colors[1] {
        [colors[0], colors[1].darker(0.2)]
    } else {
        colors
    };
    let dialog = NavalDialog {
        index: view.index,
        title: title.clone(),
        verdict: text.to_owned(),
        share,
        chance: view.win_chance,
        ships: [sides[0].0.ships.len(), sides[1].0.ships.len()],
        men: [sides[0].0.men(), sides[1].0.men()],
        withdraw_label: withdraw_label.to_owned(),
    };
    let chance_text = format!(
        "Chances de victoire estimées : {} % · {} navires contre {}",
        (view.win_chance * 100.0).round() as i64,
        dialog.ships[0],
        dialog.ships[1]
    );
    let conditions = format!(
        "Saison : {}{}",
        season_fr(setup.season),
        if setup.rain {
            "   ·   pluie : cordes d’arc mouillées"
        } else {
            ""
        }
    );
    let withdraw_hint = if player == SideId::Defender {
        "La flotte interceptée regagne son port sans combattre : l’armée ne traverse pas ce tour."
    } else {
        "L’escadre ne s’engage pas ; la flotte ennemie, menacée, regagne son port."
    };
    commands
        .spawn((
            Name::new("flow:naval_pre_battle"),
            dialog,
            DespawnOnExit(AppState::Campaign),
            UiTier::Modal.index(),
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.05, 0.03, 0.02, 0.62)),
            Pickable::default(),
        ))
        .with_children(|root| {
            root.spawn(panel_sized(theme, 1000.0)).with_children(|panel| {
                super::dialog::banner(
                    panel,
                    theme,
                    art,
                    "naval",
                    view.index as u64,
                    &title,
                    &subtitle,
                );
                panel
                    .spawn((
                        Name::new("flow:naval_verdict"),
                        typed(theme, text, UiType::Heading),
                    ))
                    .insert(TextColor(tone_color(tone)));
                panel
                    .spawn((
                        Name::new("flow:naval_balance"),
                        Node {
                            width: Val::Px(620.0),
                            height: Val::Px(18.0),
                            border: UiRect::all(Val::Px(2.0)),
                            ..default()
                        },
                        BorderColor::all(style::INK),
                    ))
                    .with_children(|bar| {
                        bar.spawn((
                            Node {
                                width: Val::Percent((share.clamp(0.0, 1.0) * 100.0) as f32),
                                height: Val::Percent(100.0),
                                ..default()
                            },
                            BackgroundColor(colors[0]),
                        ));
                        bar.spawn((
                            Node {
                                flex_grow: 1.0,
                                height: Val::Percent(100.0),
                                ..default()
                            },
                            BackgroundColor(colors[1]),
                        ));
                    });
                panel.spawn(caption(theme, &chance_text));
                panel.spawn(separator(theme));
                panel
                    .spawn(Node {
                        width: Val::Percent(100.0),
                        column_gap: Val::Px(24.0),
                        align_items: AlignItems::FlexStart,
                        ..default()
                    })
                    .with_children(|columns| {
                        for slot in 0..2 {
                            fleet_column(
                                columns,
                                theme,
                                art.assets,
                                sides[slot].0,
                                slot,
                                colors[slot],
                            );
                        }
                    });
                panel.spawn(separator(theme));
                panel.spawn(caption(theme, &conditions));
                panel.spawn((
                    Name::new("flow:naval_modifiers"),
                    caption(
                        theme,
                        "Les archers tirent des châteaux ; le plus haut bord domine l’abordage ; qui tient le vent choisit l’heure du combat.",
                    ),
                    Node {
                        max_width: Val::Px(940.0),
                        ..default()
                    },
                ));
                panel
                    .spawn(Node {
                        width: Val::Percent(100.0),
                        column_gap: Val::Px(18.0),
                        justify_content: JustifyContent::SpaceBetween,
                        align_items: AlignItems::Center,
                        margin: UiRect::top(Val::Px(8.0)),
                        ..default()
                    })
                    .with_children(|row| {
                        row.spawn(button_small(theme, WITHDRAW, withdraw_label));
                        row.spawn((
                            Name::new("flow:naval_hint"),
                            caption(theme, withdraw_hint),
                            Node {
                                max_width: Val::Px(520.0),
                                ..default()
                            },
                        ));
                        row.spawn(button_primary(theme, AUTO, "Résolution automatique"));
                    });
            });
        });
}

fn fleet_column(
    columns: &mut ChildSpawnerCommands,
    theme: &UiTheme,
    assets: &AssetServer,
    side: &NavalSideSetup,
    slot: usize,
    color: Color,
) {
    columns
        .spawn((
            Name::new(format!("flow:fleet:{slot}")),
            Node {
                flex_direction: FlexDirection::Column,
                flex_grow: 1.0,
                flex_basis: Val::Px(0.0),
                row_gap: Val::Px(4.0),
                align_items: if slot == 0 {
                    AlignItems::FlexStart
                } else {
                    AlignItems::FlexEnd
                },
                ..default()
            },
        ))
        .with_children(|column| {
            let you = if slot == 0 { "  (vous)" } else { "" };
            super::dialog::side_header(
                column,
                theme,
                assets,
                &side.faction,
                &format!("{}{you}", side.faction_name),
                slot,
                color,
            );
            if !side.admiral.is_empty() {
                column.spawn(caption(theme, &format!("Amiral : {}", side.admiral)));
            }
            column.spawn((
                caption(theme, &fleet_line(side)),
                Node {
                    max_width: Val::Px(460.0),
                    ..default()
                },
            ));
            column.spawn(caption(
                theme,
                &format!("{} hommes embarqués", fr::digits(i64::from(side.men()))),
            ));
            for ship in side.ships.iter().take(8) {
                let men: u32 = ship.crew.iter().map(|c| c.men).sum();
                column.spawn(caption(
                    theme,
                    &format!("{} — {}, {men} hommes", ship.name, ship.class.name.display),
                ));
            }
            if side.ships.len() > 8 {
                column.spawn(caption(
                    theme,
                    &format!("… et {} autres", side.ships.len() - 8),
                ));
            }
        });
}

/// Dialog buttons → the core's naval resolution.
#[allow(clippy::too_many_arguments)]
pub fn dialog_buttons(
    mut pressed: MessageReader<ButtonPressed>,
    mut commands: Commands,
    dialogs: Query<(Entity, &NavalDialog)>,
    mut sim: ResMut<Sim>,
    mut flow: ResMut<FlowState>,
    mut queue: ResMut<ModalQueue>,
    mut toasts: MessageWriter<ToastRequest>,
    mut resolved: MessageWriter<BattleResolved>,
    mut hud: ResMut<HudData>,
) {
    let Ok((entity, open)) = dialogs.single() else {
        return;
    };
    for press in pressed.read() {
        let auto = match press.name.as_str() {
            AUTO => true,
            WITHDRAW => false,
            _ => continue,
        };
        commands.entity(entity).despawn();
        let Some(data) = sim.data.clone() else {
            return;
        };
        let Some(state) = sim.state.as_mut() else {
            return;
        };
        let result = if auto {
            state.auto_resolve_naval_battle(&data, open.index)
        } else {
            state.withdraw_naval_battle(&data, open.index)
        };
        match result {
            Ok(events) => {
                if auto {
                    flow.stats.naval_resolved += 1;
                } else {
                    flow.stats.naval_withdrawn += 1;
                }
                flow.log_events(&events);
                let body = events
                    .iter()
                    .map(|e| e.text_fr.clone())
                    .collect::<Vec<_>>()
                    .join("\n");
                flow.last_result_title = open.title.clone();
                if !body.is_empty() {
                    queue.request(
                        RESULT_KEY,
                        PRIORITY_DECISION,
                        ModalSpec::info(
                            if auto {
                                "Bataille navale"
                            } else {
                                "Retour au port"
                            },
                            &body,
                        ),
                    );
                }
                resolved.write(BattleResolved {
                    kind: if auto {
                        ResolvedKind::Auto
                    } else {
                        ResolvedKind::Withdrawn
                    },
                    events,
                });
                hud.force_refresh();
            }
            Err(error) => {
                let message = error.to_string();
                flow.errors.push(message.clone());
                toasts.write(ToastRequest::new(&message));
            }
        }
        return;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seasons_are_french() {
        assert_eq!(season_fr(sim_battle::BattleSeason::Summer), "été");
        assert_eq!(season_fr(sim_battle::BattleSeason::Winter), "hiver");
    }
}
