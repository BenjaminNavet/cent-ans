//! Help panel (F1) and siege status line. Port of battle_hud.gd `_build_help`, `toggle_help`,
//! `set_siege_status` and battle_scene.gd `siege_status`.
//!
//! The help lists the controls; the keys come from the tables of the orders lot
//! (`battle::orders::hotkeys`) and of the orders bar ([`super::orders_bar::HOTKEYS`]); labels are
//! QWERTY (Godot read the keyboard layout). The siege line (walls, breaches, houses burning, gate,
//! hold of the central square, sortie) is read from [`sim_battle::SiegeWorks`]; hidden in a field
//! battle.

use bevy::prelude::*;
use serde_json::{Value, json};
use sim_battle::{CaptureRules, FireState, PieceKind, SiegeWorks};

use super::BattleHudRoot;
use crate::app::AppState;
use crate::battle::scene::BattleSession;
use crate::ui::kit::UiTheme;
use crate::ui::kit::richtext::{RichStyle, rich_text};
use crate::ui::kit::style::{self, UiType};
use crate::ui::kit::widgets::{panel_note, panel_sized};

/// The help text (`HELP_TEXT`), BBCode.
pub const HELP_TEXT: &str = "[b]Bataille — commandes[/b] (F1 : fermer)\n\
• Espace : pause (ordres possibles en pause) · + / − : vitesse ×0,5, ×1, ×2, ×4 (boutons en bas à droite).\n\
• Clic gauche : sélection (glisser : rectangle, Maj : ajouter) · clic sur une carte : sélectionner · double clic sur une carte : centrer la caméra dessus.\n\
• Clic droit : déplacer ou attaquer · double clic droit : au pas de course · glisser-droit : orienter la ligne, sa longueur donne la largeur du front.\n\
• Bannières au-dessus des troupes : clic = sélection, clic droit sur l’ennemi = attaque ; pastilles : déroute, hésite, sous le feu, charge, mode.\n\
• Caméra : W A S D, molette, Q / E ; bouton du milieu : rotation et inclinaison, Maj + bouton du milieu : déplacer la vue ; clic sur la minicarte : y aller.\n\
• Alertes (colonne en haut à gauche) : déroute, général tombé, flanc, renforts, munitions, mur ou porte ; clic : y aller.\n\
• C : verrouille la caméra sur la sélection (ou le général) ; suivi doux, bouton du milieu = orbite ; W A S D ou glisser libèrent la caméra.\n\
• Ordres : [b]F[/b] tir à volonté ou tir retenu · [b]T[/b] formation suivante · [b]H[/b] halte · [b]P[/b] poursuivre · [b]{orders}[/b] ordres du chef · [b]I[/b] incendier (siège) · [b]Maj + clic droit[/b] ordre en file · [b]clic droit minicarte[/b] déplacer la sélection vers ce point.\n\
• Modes : [b]R[/b] course · [b]G[/b] garde · [b]K[/b] escarmouche · [b]M[/b] mêlée.\n\
• Capacités : [b]Alt+1…4[/b] capacités des unités sélectionnées (boutons sous les cartes).\n\
• Sélection : [b]Ctrl+A[/b] tout · [b]Ctrl+Maj+A[/b] tireurs · [b]Ctrl+Maj+C[/b] cavalerie · [b].[/b] / [b],[/b] unité au repos suivante / précédente · [b]Ctrl+1…9[/b] enregistrer un groupe, [b]1…9[/b] le rappeler · [b]Ctrl+G[/b] verrouiller le groupe.\n\
• Affichage : [b]J[/b] replier ou déplier le journal · [b]F1[/b] cette aide.";

#[derive(Component)]
struct HelpPanel;

#[derive(Component)]
struct SiegeLine;

#[derive(Component)]
struct SiegeText;

/// What the panels show, for the dump.
#[derive(Resource, Default, Debug)]
pub struct InfoState {
    pub help_open: bool,
    pub siege_text: String,
    pub toggles: u32,
}

pub struct InfoPlugin;

impl Plugin for InfoPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<InfoState>()
            .add_systems(OnEnter(AppState::Battle), reset)
            .add_systems(
                Update,
                (spawn, toggle_help, update_siege)
                    .chain()
                    .run_if(in_state(AppState::Battle))
                    .run_if(resource_exists::<BattleSession>),
            );
    }
}

fn reset(mut state: ResMut<InfoState>) {
    *state = InfoState::default();
}

/// « Murailles 80 % · 1 brèche(s) · porte tenue · place centrale tenue 12 / 60 s ».
pub fn siege_status(works: &SiegeWorks, hold_to_win: f64) -> String {
    let mut breaches = 0;
    let mut gate_open = false;
    for piece in &works.pieces {
        if !piece.intact() {
            if piece.kind == PieceKind::Gate {
                gate_open = true;
            } else {
                breaches += 1;
            }
        }
    }
    let mut text = format!("Murailles {} %", (works.integrity() * 100.0).round() as i64);
    text += &format!(" · {breaches} brèche(s)");
    let burning = works.burning_houses();
    if burning > 0 {
        text += &format!(" · {burning} maison(s) en feu");
    }
    let gate = if gate_open {
        "enfoncée"
    } else if works.gate_fire.state == FireState::Burning {
        "en feu"
    } else {
        "tenue"
    };
    text += &format!(" · porte {gate}");
    if works.hold_time > 0.0 {
        text += &format!(
            " · place centrale tenue {} / {} s",
            works.hold_time as i64, hold_to_win as i64
        );
    }
    if works.sortie {
        text += " · sortie de la garnison !";
    }
    text
}

/// The help text with the keys of the leader's orders for the keyboard layout.
pub fn help_text(settings: Option<&crate::settings::Settings>) -> String {
    let azerty = super::orders_bar::is_azerty(settings);
    HELP_TEXT.replace("{orders}", &super::orders_bar::orders_keys_label(azerty))
}

fn spawn(
    mut commands: Commands,
    roots: Query<Entity, With<BattleHudRoot>>,
    existing: Query<(), With<HelpPanel>>,
    theme: Res<UiTheme>,
    assets: Res<AssetServer>,
    settings: Option<Res<crate::settings::Settings>>,
) {
    let Ok(root) = roots.single() else { return };
    if !existing.is_empty() {
        return;
    }
    let wrapper = |top: f32| Node {
        position_type: PositionType::Absolute,
        top: Val::Px(top),
        width: Val::Percent(100.0),
        justify_content: JustifyContent::Center,
        display: Display::None,
        ..default()
    };
    // Siege line, under the top bar.
    let siege = commands
        .spawn((
            Name::new("SiegeStatus"),
            SiegeLine,
            wrapper(62.0),
            Pickable::IGNORE,
            ChildOf(root),
        ))
        .id();
    let frame = commands
        .spawn((Name::new("SiegeFrame"), panel_note(&theme), ChildOf(siege)))
        .id();
    commands.spawn((
        SiegeText,
        Text::new(""),
        theme.text_font(false, false, UiType::Body.size()),
        TextColor(style::INK),
        TextLayout::justify(Justify::Center),
        ChildOf(frame),
    ));
    // Help, hidden until F1.
    let help = commands
        .spawn((
            Name::new("BattleHelp"),
            HelpPanel,
            wrapper(110.0),
            Pickable::IGNORE,
            ChildOf(root),
        ))
        .id();
    let panel = commands
        .spawn((panel_sized(&theme, 760.0), ChildOf(help)))
        .id();
    let mut body = RichStyle::body(&theme);
    body.size = UiType::Body.size();
    commands.spawn((
        rich_text(&theme, &assets, &help_text(settings.as_deref()), &body),
        ChildOf(panel),
    ));
}

fn toggle_help(
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<InfoState>,
    mut panels: Query<&mut Node, With<HelpPanel>>,
) {
    if keys.just_pressed(KeyCode::F1) {
        state.help_open = !state.help_open;
        state.toggles += 1;
    }
    for mut node in &mut panels {
        let display = if state.help_open {
            Display::Flex
        } else {
            Display::None
        };
        if node.display != display {
            node.display = display;
        }
    }
}

fn update_siege(
    mut session: ResMut<BattleSession>,
    mut state: ResMut<InfoState>,
    lines: Query<Entity, With<SiegeLine>>,
    mut nodes: Query<&mut Node>,
    mut texts: Query<&mut Text, With<SiegeText>>,
    mut hold: Local<Option<f64>>,
) {
    let hold_to_win = *hold.get_or_insert_with(|| CaptureRules::bundled().square.hold_s);
    let text = match session.sim_mut().siege() {
        Some(works) => siege_status(works, hold_to_win),
        None => String::new(),
    };
    if text == state.siege_text {
        return;
    }
    state.siege_text = text.clone();
    for entity in &lines {
        if let Ok(mut node) = nodes.get_mut(entity) {
            node.display = if text.is_empty() {
                Display::None
            } else {
                Display::Flex
            };
        }
    }
    for mut label in &mut texts {
        label.0 = text.clone();
    }
}

pub fn dump(world: &World) -> Value {
    let state = world.resource::<InfoState>();
    json!({
        "help_open": state.help_open,
        "help_lines": HELP_TEXT.lines().count(),
        "siege": state.siege_text,
        "toggles": state.toggles,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn help_names_the_order_keys_of_the_layout() {
        assert!(help_text(None).contains("[b]W X V B[/b]"));
        let mut settings = crate::settings::Settings::default();
        settings.keyboard_layout = "qwerty".into();
        assert!(help_text(Some(&settings)).contains("[b]Z X V B[/b]"));
        assert!(!help_text(None).contains("{orders}"));
    }

    #[test]
    fn help_lists_every_shortcut_family() {
        for needle in ["{orders}", "Alt+1…4", "Ctrl+A", "minicarte", "F1", "J"] {
            assert!(HELP_TEXT.contains(needle), "{needle}");
        }
    }
}
