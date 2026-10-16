//! Ability buttons under the unit cards (aimed shot, pavise, banner rally, close ranks, planted
//! pikes). Port of the display part of battle_ability_icons.gd: a round parchment button with the
//! ink icon (`battle_ability_<icon>`, else a letter), a curtain that empties as the cooldown runs
//! out, a gold ring when active, a grey veil when unavailable, a tooltip with name, shortcut,
//! description, timing and state.
//!
//! No rule here: the state of every ability comes from
//! [`sim_battle::BattleSim::ability_views`] (read through its serialised form, some fields being
//! crate-private). Using an ability is the deploy/orders lot's: a click sends
//! [`AbilityRequested`] and nothing else (Alt+1..4 hotkeys land there too; the lot turns it into
//! `Command::UseAbility { units, ability }` through `IssueBattleCommand`).
//!
//! The « Effets » line of the tooltip is built from the ability catalogue of the core
//! ([`effects_line`], the numbers `RuleValues` quotes in Godot).

use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui_widgets::Button;
use serde_json::{Value, json};

use super::HudData;
use crate::app::AppState;
use crate::ui::kit::icons::icon;
use crate::ui::kit::tooltip::Tooltip;
use crate::ui::kit::{ButtonPressed, UiTheme};

/// Abilities shown per card (Alt+1..4).
pub const MAX_SLOTS: usize = 4;
pub const BUTTON_SIZE: f32 = 18.0;
const PAPER: Color = Color::srgba(0.95, 0.9, 0.78, 0.95);
const PAPER_ACTIVE: Color = Color::srgba(1.0, 0.9, 0.55, 0.98);
const INK: Color = Color::srgb(0.22, 0.14, 0.07);
const GOLD: Color = Color::srgb(0.95, 0.75, 0.15);

/// An ability button was clicked (or its Alt+n shortcut pressed): the regiment and the ability id.
#[derive(Message, Debug, Clone, PartialEq, Eq)]
pub struct AbilityRequested {
    pub unit: u32,
    pub ability: String,
    /// 1..=4, the Alt+n number.
    pub slot: u8,
}

/// One ability of a regiment as the card shows it (`AbilityView`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AbilityState {
    pub id: String,
    pub kind: String,
    pub name: String,
    pub description: String,
    pub icon: String,
    pub available: bool,
    pub reason: String,
    pub active: bool,
    pub setup_remaining: f64,
    pub remaining: f64,
    pub duration: f64,
    pub cooldown: f64,
    pub cooldown_remaining: f64,
    pub ended_reason: String,
    /// Seconds before the effect counts (0: immediate), from the catalogue.
    pub setup_time: f64,
    /// « Effets : portée −45 % · … », empty when the ability has no figure to quote.
    pub effects: String,
}

/// Effects quoted by the tooltip: key of the serialised effects, label, `true` for a factor
/// shown as a signed percentage (else a raw bonus). Order of `EFFECTS` of battle_ability_icons.gd.
const EFFECTS: [(&str, &str, bool); 14] = [
    ("range_factor", "portée", true),
    ("accuracy_factor", "précision", true),
    ("vs_mounted_factor", "pertes infligées aux cavaliers", true),
    ("reload_factor", "temps de rechargement", true),
    ("missile_taken_factor", "pertes par le trait", true),
    ("speed_factor", "vitesse", true),
    ("melee_taken_front_factor", "pertes en mêlée de face", true),
    (
        "melee_taken_flank_factor",
        "pertes de flanc et de dos",
        true,
    ),
    (
        "charge_taken_factor",
        "hommes renversés par une charge",
        true,
    ),
    (
        "horse_taken_front_factor",
        "pertes face aux cavaliers de front",
        true,
    ),
    (
        "vs_horse_front_factor",
        "pertes infligées aux cavaliers de front",
        true,
    ),
    ("melee_fatigue_factor", "fatigue en mêlée", true),
    ("morale_bonus", "moral regagné d’un coup", false),
    (
        "morale_recovery",
        "moral regagné par seconde hors contact",
        false,
    ),
];

/// The « Effets » line of an ability from its serialised `BattleAbilityEffects` (absent keys
/// are neutral): factors as a signed change in percent, bonuses as is.
pub fn effects_line(effects: &Value) -> String {
    let mut parts = Vec::new();
    for (key, label, factor) in EFFECTS {
        let Some(value) = effects.get(key).and_then(Value::as_f64) else {
            continue;
        };
        if factor {
            let percent = ((value - 1.0) * 100.0).round();
            if percent == 0.0 {
                continue;
            }
            let sign = if percent > 0.0 { "+" } else { "−" };
            parts.push(format!(
                "{label} {sign}{} %",
                crate::codex::rules::number(percent.abs(), None)
            ));
        } else if value != 0.0 {
            parts.push(format!(
                "{label} +{}",
                crate::codex::rules::number(value, None)
            ));
        }
    }
    if parts.is_empty() {
        String::new()
    } else {
        format!("Effets : {}.", parts.join(" · "))
    }
}

impl AbilityState {
    /// Reads the serialised `AbilityView`.
    pub fn from_json(value: &Value) -> Self {
        let text = |key: &str| {
            value
                .get(key)
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned()
        };
        let number = |key: &str| value.get(key).and_then(Value::as_f64).unwrap_or(0.0);
        let flag = |key: &str| value.get(key).and_then(Value::as_bool).unwrap_or(false);
        Self {
            id: text("id"),
            kind: text("kind"),
            name: text("name"),
            description: text("description"),
            icon: text("icon"),
            available: flag("available"),
            reason: text("reason"),
            active: flag("active"),
            setup_remaining: number("setup_remaining"),
            remaining: number("remaining"),
            duration: number("duration"),
            cooldown: number("cooldown"),
            cooldown_remaining: number("cooldown_remaining"),
            ended_reason: text("ended_reason"),
            setup_time: 0.0,
            effects: String::new(),
        }
    }

    /// Icon id of the ink family.
    pub fn icon_id(&self) -> String {
        let name = if self.icon.is_empty() {
            &self.id
        } else {
            &self.icon
        };
        format!("battle_ability_{name}")
    }

    /// Part of the cooldown that remains (0 ready, 1 just started).
    pub fn cooldown_fraction(&self) -> f32 {
        if self.cooldown <= 0.0 {
            0.0
        } else {
            (self.cooldown_remaining / self.cooldown).clamp(0.0, 1.0) as f32
        }
    }

    /// Readable state (`status_text`).
    pub fn status_text(&self) -> String {
        if self.active {
            let mut text = "[b]Active[/b]".to_owned();
            if self.setup_remaining > 0.0 {
                text += &format!(
                    " (mise en place : {} s)",
                    self.setup_remaining.ceil() as i64
                );
            } else if self.remaining > 0.0 {
                text += &format!(" (encore {} s)", self.remaining.ceil() as i64);
            }
            return text + " — cliquer pour la lever.";
        }
        if self.available {
            return if self.ended_reason.is_empty() {
                "Prête.".to_owned()
            } else {
                format!("Prête. (Dernier arrêt : {}.)", self.ended_reason)
            };
        }
        let mut out = if self.reason.is_empty() {
            "Indisponible.".to_owned()
        } else {
            format!("Indisponible : {}.", self.reason)
        };
        if !self.ended_reason.is_empty() {
            out += &format!(" (Arrêtée : {}.)", self.ended_reason);
        }
        out
    }

    /// Tooltip (`BattleAbilityIcons.tip`, without the numeric effects).
    pub fn tip(&self, slot: usize) -> String {
        let mut title = format!("[b]{}[/b]", self.name);
        if (1..=MAX_SLOTS).contains(&slot) {
            title += &format!(" (Alt+{slot})");
        }
        let number = |value: f64| crate::codex::rules::number(value, None);
        let mut timing = format!("Recharge : {} s", number(self.cooldown));
        if self.duration > 0.0 {
            timing += &format!(" · durée : {} s", number(self.duration));
        } else {
            timing += " · tant que l’unité s’y tient";
        }
        if self.setup_time > 0.0 {
            timing += &format!(" · mise en place : {} s", number(self.setup_time));
        }
        let mut lines = vec![title, self.description.clone(), timing];
        if !self.effects.is_empty() {
            lines.push(self.effects.clone());
        }
        lines.push(self.status_text());
        lines.join("\n")
    }

    /// Letter shown when the ink icon does not exist.
    pub fn letter(&self) -> String {
        self.name
            .chars()
            .next()
            .map(String::from)
            .unwrap_or_default()
    }
}

#[derive(Component)]
struct AbilityButton {
    unit: u32,
    index: usize,
}

#[derive(Component)]
struct AbilityCurtain {
    unit: u32,
    index: usize,
}

/// What the buttons did, for the dump.
#[derive(Resource, Default, Debug)]
pub struct AbilityLog {
    pub requests: Vec<AbilityRequested>,
}

pub struct AbilitiesPlugin;

impl Plugin for AbilitiesPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<AbilityRequested>()
            .init_resource::<AbilityLog>()
            .add_systems(OnEnter(AppState::Battle), clear)
            .add_systems(
                Update,
                (ability_clicks, update_buttons)
                    .chain()
                    .run_if(in_state(AppState::Battle)),
            );
    }
}

fn clear(mut log: ResMut<AbilityLog>) {
    *log = AbilityLog::default();
}

/// The row of ability buttons of the regiment `unit`, under its card. Buttons beyond the
/// regiment's abilities stay hidden.
pub fn spawn_row(commands: &mut Commands, parent: Entity, theme: &UiTheme, unit: u32) {
    let row = commands
        .spawn((
            Name::new(format!("abilities:{unit}")),
            Node {
                width: Val::Px(super::cards::CARD_WIDTH),
                height: Val::Px(BUTTON_SIZE + 2.0),
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                column_gap: Val::Px(2.0),
                ..default()
            },
            ChildOf(parent),
        ))
        .id();
    for index in 0..MAX_SLOTS {
        let button = commands
            .spawn((
                Name::new(format!("ability:{unit}:{index}")),
                AbilityButton { unit, index },
                Button,
                Hovered::default(),
                Tooltip::new(""),
                Node {
                    width: Val::Px(BUTTON_SIZE),
                    height: Val::Px(BUTTON_SIZE),
                    border: UiRect::all(Val::Px(1.0)),
                    overflow: Overflow::clip(),
                    display: Display::None,
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border_radius: BorderRadius::MAX,
                    ..default()
                },
                BackgroundColor(PAPER),
                BorderColor::all(INK),
                ChildOf(row),
            ))
            .id();
        commands.spawn((
            Name::new("AbilityLetter"),
            Text::new(""),
            theme.text_font(true, false, 11.0),
            TextColor(INK),
            Pickable::IGNORE,
            ChildOf(button),
        ));
        commands.spawn((
            AbilityCurtain { unit, index },
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                top: Val::Px(0.0),
                height: Val::Percent(0.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.08, 0.05, 0.02, 0.55)),
            Pickable::IGNORE,
            ChildOf(button),
        ));
    }
}

fn ability_clicks(
    mut pressed: MessageReader<ButtonPressed>,
    data: Res<HudData>,
    mut log: ResMut<AbilityLog>,
    mut out: MessageWriter<AbilityRequested>,
) {
    for press in pressed.read() {
        let mut parts = press.name.split(':');
        if parts.next() != Some("ability") {
            continue;
        }
        let (Some(unit), Some(index)) = (
            parts.next().and_then(|t| t.parse::<u32>().ok()),
            parts.next().and_then(|t| t.parse::<usize>().ok()),
        ) else {
            continue;
        };
        let Some(state) = data.abilities.get(&unit).and_then(|list| list.get(index)) else {
            continue;
        };
        let request = AbilityRequested {
            unit,
            ability: state.id.clone(),
            slot: (index + 1) as u8,
        };
        log.requests.push(request.clone());
        out.write(request);
    }
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn update_buttons(
    data: Res<HudData>,
    theme: Res<UiTheme>,
    assets: Res<AssetServer>,
    mut commands: Commands,
    mut buttons: Query<(
        Entity,
        &AbilityButton,
        &mut Node,
        &mut BackgroundColor,
        &mut BorderColor,
        &mut Tooltip,
        Option<&Children>,
    )>,
    mut curtains: Query<(&AbilityCurtain, &mut Node), Without<AbilityButton>>,
    mut letters: Query<&mut Text>,
    mut iconed: Local<std::collections::HashSet<(u32, usize)>>,
) {
    if !data.is_changed() {
        return;
    }
    for (entity, button, mut node, mut background, mut border, mut tooltip, children) in
        &mut buttons
    {
        let state = data
            .abilities
            .get(&button.unit)
            .and_then(|list| list.get(button.index));
        let Some(state) = state else {
            if node.display != Display::None {
                node.display = Display::None;
            }
            continue;
        };
        if node.display != Display::Flex {
            node.display = Display::Flex;
        }
        let ink_icon = theme.icons.get(&state.icon_id()).is_some();
        if ink_icon && iconed.insert((button.unit, button.index)) {
            commands.spawn((
                icon(&theme, &assets, &state.icon_id(), BUTTON_SIZE - 6.0),
                Pickable::IGNORE,
                ChildOf(entity),
            ));
        }
        *background = BackgroundColor(if state.active { PAPER_ACTIVE } else { PAPER });
        *border = BorderColor::all(if state.active { GOLD } else { INK });
        let tip = state.tip(button.index + 1);
        if tooltip.bbcode != tip {
            tooltip.bbcode = tip;
        }
        // The letter child (first child) is only shown without an ink icon.
        if let Some(first) = children.and_then(|c| c.first().copied())
            && let Ok(mut text) = letters.get_mut(first)
        {
            let shown = if ink_icon {
                String::new()
            } else {
                state.letter()
            };
            if text.0 != shown {
                text.0 = shown;
            }
        }
    }
    for (curtain, mut node) in &mut curtains {
        let Some(state) = data
            .abilities
            .get(&curtain.unit)
            .and_then(|list| list.get(curtain.index))
        else {
            continue;
        };
        let fraction = if state.active {
            0.0
        } else if !state.available && state.cooldown_remaining <= 0.0 {
            1.0
        } else {
            state.cooldown_fraction()
        };
        node.height = Val::Percent(fraction * 100.0);
    }
}

pub fn dump(world: &World) -> Value {
    let data = world.resource::<HudData>();
    let log = world.resource::<AbilityLog>();
    let with: Vec<&u32> = data
        .abilities
        .iter()
        .filter(|(_, l)| !l.is_empty())
        .map(|(id, _)| id)
        .collect();
    let first = data.abilities.values().find_map(|list| list.first());
    json!({
        "units_with_abilities": with.len(),
        "first": first.map(|a| json!({
            "id": a.id, "name": a.name, "icon": a.icon_id(), "available": a.available,
            "reason": a.reason, "tip": a.tip(1),
        })),
        "requests": log.requests.iter().map(|r| json!({"unit": r.unit, "ability": r.ability, "slot": r.slot})).collect::<Vec<_>>(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> AbilityState {
        AbilityState::from_json(&json!({
            "id": "pavise", "kind": "pavise", "name": "Pavois", "description": "Plante le pavois.",
            "icon": "pavise", "available": true, "reason": "", "active": false,
            "cooldown": 20.0, "cooldown_remaining": 5.0, "duration": 0.0,
        }))
    }

    #[test]
    fn reads_the_serialised_view() {
        let s = state();
        assert_eq!(s.icon_id(), "battle_ability_pavise");
        assert!((s.cooldown_fraction() - 0.25).abs() < 1e-6);
        assert_eq!(s.letter(), "P");
    }

    #[test]
    fn effects_line_quotes_signed_percentages_and_bonuses() {
        let line = effects_line(&json!({
            "range_factor": 0.55, "speed_factor": 1.0, "melee_taken_front_factor": 1.25,
            "morale_bonus": 10.0, "frontal_only": true
        }));
        assert_eq!(
            line,
            "Effets : portée −45 % · pertes en mêlée de face +25 % · moral regagné d’un coup +10."
        );
        assert_eq!(effects_line(&json!({"speed_factor": 1.0})), "");
        let mut s = state();
        s.effects = line;
        s.setup_time = 1.5;
        let tip = s.tip(1);
        assert!(tip.contains("mise en place : 1,5 s"));
        assert!(tip.contains("Effets : portée"));
    }

    #[test]
    fn tip_names_the_shortcut_and_the_state() {
        let tip = state().tip(2);
        assert!(tip.contains("Alt+2"));
        assert!(tip.contains("Prête."));
        assert!(tip.contains("tant que l’unité s’y tient"));
    }

    #[test]
    fn status_of_active_and_refused() {
        let mut s = state();
        s.active = true;
        s.remaining = 3.2;
        assert!(s.status_text().contains("encore 4 s"));
        s.active = false;
        s.available = false;
        s.reason = "au contact de l'ennemi".into();
        s.ended_reason = "l'unité s'est mise en mouvement".into();
        let text = s.status_text();
        assert!(text.starts_with("Indisponible : au contact"));
        assert!(text.contains("(Arrêtée :"));
    }
}
