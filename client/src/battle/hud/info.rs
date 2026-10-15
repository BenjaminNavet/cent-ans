//! Help panel (F1) and siege status line. Port of battle_hud.gd `_build_help`, `toggle_help`,
//! `set_siege_status` and battle_scene.gd `siege_status`.
//!
//! The help lists the controls; the keys come from the tables of the orders lot
//! (`settings::KeyMap`, actions `battle_*`), labelled for the keyboard layout. The siege line (walls, breaches, houses burning, gate,
//! hold of the central square, sortie) is read from [`sim_battle::SiegeWorks`]; hidden in a field
//! battle.

use bevy::prelude::*;
use serde_json::{Value, json};
use sim_battle::{CaptureRules, FireState, PieceKind, SiegeWorks};

use super::BattleHudRoot;
use crate::app::AppState;
use crate::battle::scene::BattleSession;
use crate::settings::KeyMap;
use crate::settings::keymap::layout_label;
use crate::ui::kit::UiTheme;
use crate::ui::kit::richtext::{RichStyle, rich_text};
use crate::ui::kit::style::{self, UiType};
use crate::ui::kit::widgets::{panel_note, panel_sized};

/// The help template (`HELP_TEXT`), BBCode. `{action}` is replaced by the keys of that key-map
/// action for the layout (`{action:1}`: its first key only), so a rebound key shows up here.
pub const HELP_TEXT: &str = "[b]Bataille — commandes[/b] ({help_open} : fermer)\n\
• {battle_pause} : pause (ordres possibles en pause) · {battle_speed_up} / {battle_speed_down} : vitesse ×0,5, ×1, ×2, ×4 (boutons en bas à droite).\n\
• Clic gauche : sélection (glisser : rectangle, Maj : ajouter) · clic sur une carte : sélectionner · double clic sur une carte : centrer la caméra dessus.\n\
• Clic droit : déplacer ou attaquer · double clic droit : au pas de course · glisser-droit : orienter la ligne, sa longueur donne la largeur du front.\n\
• Bannières au-dessus des troupes : clic = sélection, clic droit sur l’ennemi = attaque ; pastilles : déroute, hésite, sous le feu, charge, mode.\n\
• Caméra : {map_pan_up:1} {map_pan_left:1} {map_pan_down:1} {map_pan_right:1}, molette, {map_rotate_left} / {map_rotate_right} ; bouton du milieu : rotation et inclinaison, Maj + bouton du milieu : déplacer la vue ; clic sur la minicarte : y aller.\n\
• Alertes (colonne en haut à gauche) : déroute, général tombé, flanc, renforts, munitions, mur ou porte ; clic : y aller.\n\
• {battle_follow} : verrouille la caméra sur la sélection (ou le général) ; suivi doux, bouton du milieu = orbite ; {map_pan_up:1} {map_pan_left:1} {map_pan_down:1} {map_pan_right:1} ou glisser libèrent la caméra.\n\
• Ordres : [b]{battle_fire_at_will}[/b] tir à volonté ou tir retenu · [b]{battle_formation}[/b] formation suivante · [b]{battle_halt}[/b] halte · [b]{battle_pursue}[/b] poursuivre · [b]{orders}[/b] ordres du chef · [b]{battle_burn}[/b] incendier (siège) · [b]Maj + clic droit[/b] ordre en file · [b]clic droit minicarte[/b] déplacer la sélection vers ce point.\n\
• Modes : [b]{battle_run}[/b] course · [b]{battle_guard}[/b] garde · [b]{battle_skirmish}[/b] escarmouche · [b]{battle_melee}[/b] mêlée.\n\
• Capacités : [b]{battle_ability_1}[/b], [b]{battle_ability_2}[/b], [b]{battle_ability_3}[/b], [b]{battle_ability_4}[/b] capacités des unités sélectionnées (boutons sous les cartes).\n\
• Sélection : [b]{battle_select_all}[/b] tout · [b]{battle_select_shooters}[/b] tireurs · [b]{battle_select_cavalry}[/b] cavalerie · [b]{battle_next_idle}[/b] / [b]{battle_prev_idle}[/b] unité au repos suivante / précédente · [b]Ctrl+1…9[/b] enregistrer un groupe, [b]1…9[/b] le rappeler · [b]{battle_lock_group}[/b] verrouiller le groupe.\n\
• Affichage : [b]{battle_journal}[/b] replier ou déplier le journal · [b]{battle_tactical_view}[/b] vue tactique · [b]{battle_markers}[/b] bannières · [b]{help_open}[/b] cette aide.";

#[derive(Component)]
struct HelpPanel;

/// The rich text of the help (rebuilt when the key map or the layout changes).
#[derive(Component)]
struct HelpBody(Entity);

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
                (
                    spawn,
                    refresh_help,
                    help_key.run_if(crate::ui::kit::text_field::keyboard_free),
                    toggle_help,
                    update_siege,
                )
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

/// The help text with the keys of the key map for the keyboard layout.
pub fn help_text(settings: Option<&crate::settings::Settings>, keymap: &KeyMap) -> String {
    let azerty = super::orders_bar::is_azerty(settings);
    let layout = if azerty { "azerty" } else { "qwerty" };
    let mut text = String::with_capacity(HELP_TEXT.len() + 200);
    let mut rest = HELP_TEXT;
    while let Some(open) = rest.find('{') {
        text.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        let Some(close) = after.find('}') else { break };
        let name = &after[..close];
        if name == "orders" {
            text.push_str(&super::orders_bar::orders_keys_label(keymap, azerty));
        } else if let Some(action) = name.strip_suffix(":1") {
            let first = keymap
                .chords(action)
                .first()
                .map_or(String::new(), |c| layout_label(&c.label(), layout));
            text.push_str(&first);
        } else {
            text.push_str(&keymap.label(name, layout));
        }
        rest = &after[close + 1..];
    }
    text.push_str(rest);
    text
}

fn spawn(
    mut commands: Commands,
    roots: Query<Entity, With<BattleHudRoot>>,
    existing: Query<(), With<HelpPanel>>,
    theme: Res<UiTheme>,
    assets: Res<AssetServer>,
    settings: Option<Res<crate::settings::Settings>>,
    keymap: Res<KeyMap>,
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
        HelpBody(panel),
        rich_text(
            &theme,
            &assets,
            &help_text(settings.as_deref(), &keymap),
            &body,
        ),
        ChildOf(panel),
    ));
}

/// Rebuilds the help text when a key or the layout was changed.
fn refresh_help(
    mut commands: Commands,
    bodies: Query<(Entity, &HelpBody)>,
    theme: Res<UiTheme>,
    assets: Res<AssetServer>,
    settings: Option<Res<crate::settings::Settings>>,
    keymap: Res<KeyMap>,
) {
    let settings_changed = settings.as_ref().is_some_and(|s| s.is_changed());
    if !keymap.is_changed() && !settings_changed {
        return;
    }
    for (entity, panel) in &bodies {
        // The first frame spawns the body; a change flagged at the same time rebuilds it once more.
        commands.entity(entity).despawn();
        let mut body = RichStyle::body(&theme);
        body.size = UiType::Body.size();
        commands.spawn((
            HelpBody(panel.0),
            rich_text(
                &theme,
                &assets,
                &help_text(settings.as_deref(), &keymap),
                &body,
            ),
            ChildOf(panel.0),
        ));
    }
}

fn help_key(keys: Res<ButtonInput<KeyCode>>, keymap: Res<KeyMap>, mut state: ResMut<InfoState>) {
    if keymap.just_pressed("help_open", &keys) {
        state.help_open = !state.help_open;
        state.toggles += 1;
    }
}

fn toggle_help(state: Res<InfoState>, mut panels: Query<&mut Node, With<HelpPanel>>) {
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
        let map = KeyMap::defaults();
        assert!(help_text(None, &map).contains("[b]W X V B[/b]"));
        let settings = crate::settings::Settings {
            keyboard_layout: "qwerty".into(),
            ..Default::default()
        };
        assert!(help_text(Some(&settings), &map).contains("[b]Z X V B[/b]"));
        assert!(!help_text(None, &map).contains('{'));
    }

    #[test]
    fn help_follows_rebound_keys() {
        let mut stored = std::collections::BTreeMap::new();
        stored.insert("battle_pause".to_owned(), vec!["P".to_owned()]);
        let map = KeyMap::from_bindings(&stored);
        let text = help_text(None, &map);
        assert!(text.contains("• P : pause"), "{text}");
    }

    #[test]
    fn help_names_every_battle_action_it_mentions() {
        let qwerty = crate::settings::Settings {
            keyboard_layout: "qwerty".into(),
            ..Default::default()
        };
        let text = help_text(Some(&qwerty), &KeyMap::defaults());
        for needle in ["Alt+1", "Ctrl+A", "minicarte", "F1", "[b]J[/b]", "Espace"] {
            assert!(text.contains(needle), "{needle}");
        }
        for open in HELP_TEXT.split('{').skip(1) {
            let name = open.split('}').next().unwrap_or("");
            let name = name.strip_suffix(":1").unwrap_or(name);
            assert!(
                name == "orders" || crate::settings::keymap::definition(name).is_some(),
                "{name}"
            );
        }
    }
}
