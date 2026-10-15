//! Battle HUD: unit cards with their ability buttons, alerts column, speed controls, minimap,
//! leader seal and orders bar, journal, help (F1) and siege status, comparison panel, tooltips and
//! result screen (with its suites cards, history card and painted backdrop).
//!
//! Ports game/scripts/battle/battle_hud.gd (top bar, bottom band, speed buttons), unit_card.gd,
//! roster_card.gd, battle_alerts_column.gd, battle_compare_panel.gd and
//! battle_result_screen.gd. Everything is presentation: the figures come from
//! [`sim_battle::BattleSim`] through [`view::UnitView`] and [`HudData`] (refreshed after every
//! simulation step).
//!
//! Interfaces:
//! - clicking a card sends [`crate::battle::orders::SelectBattleUnits`]; the highlight reads
//!   [`crate::battle::orders::BattleSelection`] (written by the orders lot);
//! - [`HoveredBattleUnits`]: the orders lot (ground, banners) writes the regiments under the
//!   cursor; with exactly one regiment selected and an enemy hovered, the comparison panel opens;
//! - [`BattleAlertRaised`]: every typed alert drained from the simulation, for the audio lot;
//! - the result screen sends [`crate::flow::LeaveBattle`].
//!
//! Wave 4 additions: [`minimap`] (battle_minimap.gd), [`orders_bar`] (leader_orders_bar.gd and
//! the seal of battle_hud.gd), [`journal`], [`info`] (help and siege status), [`abilities`]
//! (display of battle_ability_icons.gd; the click sends [`abilities::AbilityRequested`], the
//! deploy lot applies it) and [`aftermath`] (result screen extras, filled through
//! [`aftermath::ResultExtras`]).
//!
//! Wave 5 additions: [`formation_menu`] (battle_formation_menu.gd and its « Formation » button),
//! state badges on the cards, the « Effets » line of the ability tooltips, the colour-blind
//! minimap, AZERTY key labels, the leader-order fallback, the HUD hidden during the cinematic.
//!
//! Not ported: selection groups (`battle_groups.gd`, done by the orders lot), the « Revoir la
//! bataille » button.
//! Owned by a single lot of agent (ADR 0357); other lots do not edit this folder.

pub mod abilities;
pub mod aftermath;
pub mod alerts;
pub mod cards;
pub mod compare;
pub mod formation_menu;
pub mod info;
pub mod journal;
pub mod minimap;
pub mod orders_bar;
pub mod result;
pub mod speed;
pub mod tips;
pub mod view;

use std::collections::HashMap;

use bevy::prelude::*;
use bevy::state::state_scoped::DespawnOnExit;
use sim_battle::SideId;

use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::battle::scene::{BattleSession, StepSet};
use crate::ui::kit::UiTheme;
use crate::ui::kit::style::UiTier;
use crate::ui::kit::tooltip::TooltipRegistry;
use crate::ui::kit::tooltip_spec::{TooltipTexts, register_rule_texts};
use view::UnitView;

pub use alerts::BattleAlertRaised;

/// Regiments under the cursor (ground, banner), written by the orders lot; read by the
/// comparison panel. The dev hook `CENT_ANS_HUD_HOVER=<unit id>` seeds it for scripted runs.
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub struct HoveredBattleUnits(pub Vec<u32>);

/// What the HUD shows, refreshed after every simulation step.
#[derive(Resource, Default, Debug, Clone)]
pub struct HudData {
    /// Tick of the last refresh (`None`: not yet filled).
    pub tick: Option<u64>,
    /// Bumped by every refresh.
    pub revision: u64,
    pub player: Option<SideId>,
    /// Regiments of the player's side (no synthetic ones), simulation order.
    pub own: Vec<UnitView>,
    pub enemy: Vec<UnitView>,
    /// Display names of the player's regiments (`distinct_names`).
    pub names: HashMap<u32, String>,
    pub elapsed_s: f64,
    pub time_left_s: f64,
    /// `[own, enemy]`.
    pub strength: [u32; 2],
    pub faction_names: [String; 2],
    pub colors: [Color; 2],
    pub finished: bool,
    /// Abilities of the player's regiments, in their Alt+n order.
    pub abilities: HashMap<u32, Vec<abilities::AbilityState>>,
}

impl HudData {
    pub fn unit(&self, id: u32) -> Option<&UnitView> {
        self.own.iter().chain(&self.enemy).find(|u| u.id == id)
    }

    /// Own regiment name (distinct), else the simulation's.
    pub fn name_of(&self, id: u32) -> String {
        self.names
            .get(&id)
            .cloned()
            .or_else(|| self.unit(id).map(|u| u.name.clone()))
            .unwrap_or_default()
    }
}

/// Root of the battle HUD (everything is a descendant).
#[derive(Component)]
pub struct BattleHudRoot;

/// Rule texts of `data/ui/tooltips.json`, loaded once.
#[derive(Resource, Default)]
pub struct HudTexts {
    pub texts: TooltipTexts,
    registered: bool,
    /// The HUD has been spawned (dump).
    built: bool,
    /// The HUD is hidden by the cinematic shot (dump).
    pub cinematic_hidden: bool,
}

/// Shared inputs of the builders.
pub struct BuildCtx<'a> {
    pub theme: &'a UiTheme,
    pub assets: &'a AssetServer,
    pub data: &'a HudData,
}

pub struct BattleHudPlugin;

impl Plugin for BattleHudPlugin {
    fn build(&self, app: &mut App) {
        let hover = std::env::var("CENT_ANS_HUD_HOVER")
            .ok()
            .and_then(|text| text.trim().parse::<u32>().ok())
            .map(|id| vec![id])
            .unwrap_or_default();
        app.init_resource::<HudData>()
            .init_resource::<HudTexts>()
            .insert_resource(HoveredBattleUnits(hover))
            .add_plugins((
                alerts::AlertsPlugin,
                abilities::AbilitiesPlugin,
                cards::CardsPlugin,
                minimap::MinimapPlugin,
                orders_bar::OrdersBarPlugin,
                formation_menu::FormationMenuPlugin,
                journal::JournalPlugin,
                info::InfoPlugin,
                speed::SpeedPlugin,
                compare::ComparePlugin,
                result::ResultPlugin,
            ))
            .add_systems(
                Update,
                (refresh_data, build_hud)
                    .chain()
                    .after(StepSet)
                    .run_if(in_state(AppState::Battle))
                    .run_if(resource_exists::<BattleSession>),
            )
            .add_systems(
                Update,
                hide_during_cinematic.run_if(in_state(AppState::Battle)),
            )
            .add_systems(OnExit(AppState::Battle), reset_data)
            .add_systems(
                Update,
                dev_seed_selection
                    .run_if(in_state(AppState::Battle))
                    .run_if(resource_exists::<BattleSession>),
            )
            .register_dump(dump_hud);
    }
}

/// Dev hook for scripted runs: `CENT_ANS_HUD_SELECT=<unit id>` selects that regiment once, after
/// the HUD is built, when nothing is selected (the orders lot owns the real selection).
fn dev_seed_selection(
    texts: Res<HudTexts>,
    mut selection: ResMut<crate::battle::orders::BattleSelection>,
    mut done: Local<bool>,
) {
    if *done || !texts.built {
        return;
    }
    *done = true;
    if let Some(id) = std::env::var("CENT_ANS_HUD_SELECT")
        .ok()
        .and_then(|text| text.trim().parse::<u32>().ok())
        && selection.units.is_empty()
    {
        selection.units.push(id);
    }
}

/// Reads the simulation into [`HudData`] when a step happened; drains the typed alerts.
fn refresh_data(
    mut session: ResMut<BattleSession>,
    mut data: ResMut<HudData>,
    mut raised: MessageWriter<BattleAlertRaised>,
    mut ability_rules: Local<HashMap<String, (f64, String)>>,
) {
    if data.tick == Some(session.snapshot.tick) {
        return;
    }
    let player = session.player.unwrap_or(SideId::Attacker);
    let colors = session.colors;
    let tick = session.snapshot.tick;
    let finished = session.snapshot.finished;
    let sim = session.sim_mut();
    let (mut own, mut enemy) = (Vec::new(), Vec::new());
    let mut ability_states = HashMap::new();
    if ability_rules.is_empty() {
        for ability in sim.ability_catalog() {
            let effects = serde_json::to_value(&ability.effects).unwrap_or_default();
            ability_rules.insert(
                ability.id.clone(),
                (ability.setup_time, abilities::effects_line(&effects)),
            );
        }
    }
    for unit in sim.units().iter().filter(|u| !u.synthetic) {
        let mut view = view::unit_view(unit);
        view.status = view::status_view(sim, unit);
        if unit.side == player {
            let states: Vec<abilities::AbilityState> = sim
                .ability_views(unit)
                .iter()
                .filter_map(|v| serde_json::to_value(v).ok())
                .map(|v| {
                    let mut state = abilities::AbilityState::from_json(&v);
                    if let Some((setup, effects)) = ability_rules.get(&state.id) {
                        state.setup_time = *setup;
                        state.effects = effects.clone();
                    }
                    state
                })
                .collect();
            if !states.is_empty() {
                ability_states.insert(unit.id, states);
            }
            own.push(view);
        } else {
            enemy.push(view);
        }
    }
    let enemy_side = match player {
        SideId::Attacker => SideId::Defender,
        SideId::Defender => SideId::Attacker,
    };
    let names = view::distinct_names(&own).into_iter().collect();
    let color_of = |side: SideId| {
        let rgb = colors[match side {
            SideId::Attacker => 0,
            SideId::Defender => 1,
        }];
        Color::srgb(rgb[0], rgb[1], rgb[2])
    };
    let faction_names =
        [player, enemy_side].map(|side| sim.setup().side(side).faction_name.clone());
    let new = HudData {
        tick: Some(tick),
        revision: data.revision + 1,
        player: Some(player),
        own,
        enemy,
        names,
        elapsed_s: sim.elapsed(),
        time_left_s: sim.time_left_s(),
        strength: [sim.strength(player), sim.strength(enemy_side)],
        faction_names,
        colors: [color_of(player), color_of(enemy_side)],
        finished,
        abilities: ability_states,
    };
    for alert in sim.take_new_alerts() {
        raised.write(BattleAlertRaised(alert));
    }
    *data = new;
}

/// The HUD disappears while the cinematic shot of the first clash plays (black bars only), and
/// comes back with it (`orders::cinematic::Cinematic`).
fn hide_during_cinematic(
    cinematic: Option<Res<crate::battle::orders::cinematic::Cinematic>>,
    mut roots: Query<&mut Visibility, With<BattleHudRoot>>,
    mut texts: ResMut<HudTexts>,
) {
    let hidden = cinematic.is_some_and(|c| c.active);
    texts.cinematic_hidden = hidden;
    for mut visibility in &mut roots {
        let wanted = if hidden {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
        if *visibility != wanted {
            *visibility = wanted;
        }
    }
}

fn reset_data(mut data: ResMut<HudData>, mut texts: ResMut<HudTexts>) {
    *data = HudData::default();
    texts.built = false;
}

/// Spawns the HUD once the first data is in.
fn build_hud(
    mut commands: Commands,
    roots: Query<(), With<BattleHudRoot>>,
    theme: Res<UiTheme>,
    assets: Res<AssetServer>,
    data: Res<HudData>,
    mut texts: ResMut<HudTexts>,
    mut registry: ResMut<TooltipRegistry>,
) {
    if !roots.is_empty() || data.tick.is_none() {
        return;
    }
    if !texts.registered {
        texts.registered = true;
        if let Ok(loaded) = TooltipTexts::load() {
            register_rule_texts(&mut registry, &loaded, |_| None);
            texts.texts = loaded;
        }
    }
    texts.built = true;
    let root = commands
        .spawn((
            Name::new("BattleHud"),
            BattleHudRoot,
            DespawnOnExit(AppState::Battle),
            UiTier::Hud.index(),
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let ctx = BuildCtx {
        theme: &theme,
        assets: &assets,
        data: &data,
    };
    speed::spawn_top_bar(&mut commands, root, &ctx);
    speed::spawn_speed_panel(&mut commands, root, &ctx);
    cards::spawn_band(&mut commands, root, &ctx, &texts.texts);
    alerts::spawn_column(&mut commands, root, &ctx);
    compare::spawn_panel(&mut commands, root, &ctx);
}

fn dump_hud(world: &World) -> (String, serde_json::Value) {
    let data = world.resource::<HudData>();
    let built = world.resource::<HudTexts>().built;
    (
        "battle_hud".into(),
        serde_json::json!({
            "built": built,
            "hidden_for_cinematic": world.resource::<HudTexts>().cinematic_hidden,
            "revision": data.revision,
            "own_units": data.own.len(),
            "enemy_units": data.enemy.len(),
            "strength": data.strength,
            "factions": data.faction_names,
            "cards": cards::dump(world),
            "alerts": alerts::dump(world),
            "speed": speed::dump(world),
            "compare": compare::dump(world),
            "minimap": minimap::dump(world),
            "orders_bar": orders_bar::dump(world),
            "formation_menu": formation_menu::dump(world),
            "journal": journal::dump(world),
            "info": info::dump(world),
            "abilities": abilities::dump(world),
            "result": result::dump(world),
        }),
    )
}
