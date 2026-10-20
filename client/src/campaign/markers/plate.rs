//! Town plates in the grand-strategy manner (lot TS-T6): the name label of a place is a
//! cartouche coloured by the holder's standing towards the player (own / allied / neutral /
//! enemy, same partition as the borders of ADR 0155 and the Diplomacy mode), the holder's shield
//! on the left and, on the right, one to three round pills (siege, garrison, buildings), each
//! with a tooltip.
//!
//! No rule here: the standing comes from the core through the overlay's `ProvinceView`, the
//! figures from `SettlementState`; colours, sizes and which pills a rank shows are read from
//! `data/map/stance_cues.json` (`town.plate`).

use bevy::prelude::*;
use serde_json::{Value, json};

use super::{MarkerStats, PlaceLabel, Pins, placement, places::Places};
use crate::campaign::provinces::modes::{self, Cue, MapMode, MapModeState, Tuning};
use crate::campaign::provinces::overlay::OverlayState;
use crate::campaign::terrain::surface::GroundHeight;
use crate::sim::Sim;
use crate::ui::kit::UiTheme;
use crate::ui::kit::tooltip::Tooltip;

/// Seconds between two refreshes of the plates (the core moves once per turn).
const REFRESH_SECS: f32 = 0.25;

/// Standing of a place's holder towards the player.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlateStatus {
    Own,
    Ally,
    Neutral,
    Enemy,
}

impl PlateStatus {
    pub const ALL: [PlateStatus; 4] = [
        PlateStatus::Own,
        PlateStatus::Ally,
        PlateStatus::Neutral,
        PlateStatus::Enemy,
    ];

    /// Key in `town.plate.status`.
    pub fn key(self) -> &'static str {
        match self {
            PlateStatus::Own => "self",
            PlateStatus::Ally => "ally",
            PlateStatus::Neutral => "neutral",
            PlateStatus::Enemy => "enemy",
        }
    }

    pub fn index(self) -> usize {
        Self::ALL.iter().position(|s| *s == self).unwrap_or(0)
    }
}

/// Status of a stance key (`war`, `ally`, `self`, ...) through the `categories` of the cue file.
pub fn status_of(stance_key: &str, cues: &Value) -> PlateStatus {
    match modes::cue_of_stance(stance_key, cues) {
        Cue::SelfFaction => PlateStatus::Own,
        Cue::Friend => PlateStatus::Ally,
        Cue::Enemy => PlateStatus::Enemy,
        Cue::Other => PlateStatus::Neutral,
    }
}

/// Fill, border and name ink of a status.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StatusLook {
    pub fill: Srgba,
    pub border: Srgba,
    pub ink: Srgba,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PastilleKind {
    Siege,
    Garrison,
    Buildings,
}

impl PastilleKind {
    pub const ALL: [PastilleKind; 3] = [
        PastilleKind::Siege,
        PastilleKind::Garrison,
        PastilleKind::Buildings,
    ];

    pub fn key(self) -> &'static str {
        match self {
            PastilleKind::Siege => "siege",
            PastilleKind::Garrison => "garrison",
            PastilleKind::Buildings => "buildings",
        }
    }

    pub fn index(self) -> usize {
        Self::ALL.iter().position(|k| *k == self).unwrap_or(0)
    }
}

/// Look of a pill kind: colour and minimal place rank.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PastilleLook {
    pub color: Srgba,
    pub min_rank: u8,
}

/// `town.plate` of `stance_cues.json`.
#[derive(Debug, Clone, PartialEq)]
pub struct PlateStyle {
    pub looks: [StatusLook; 4],
    pub fill_alpha: f32,
    pub border_px: f32,
    pub max_pastilles: usize,
    pub size_px: f32,
    pub pastilles: [PastilleLook; 3],
}

impl PlateStyle {
    /// Reads the `town.plate` block of the cue file (`None` or a missing key: neutral fallbacks,
    /// so a broken data file never hides a name).
    pub fn from_cues(cues: &Value) -> Self {
        let plate = &cues["town"]["plate"];
        let colour = |node: &Value, fallback: Srgba| {
            node.as_str().and_then(modes::hex).unwrap_or(fallback)
        };
        let fallback = StatusLook {
            fill: Srgba::new(0.26, 0.21, 0.16, 1.0),
            border: Srgba::new(0.54, 0.48, 0.36, 1.0),
            ink: Srgba::new(0.94, 0.89, 0.77, 1.0),
        };
        let look = |status: PlateStatus| {
            let node = &plate["status"][status.key()];
            StatusLook {
                fill: colour(&node["fill"], fallback.fill),
                border: colour(&node["border"], fallback.border),
                ink: colour(&node["ink"], fallback.ink),
            }
        };
        let pill = |kind: PastilleKind| {
            let node = &plate["pastilles"][kind.key()];
            PastilleLook {
                color: colour(&node["color"], fallback.fill),
                min_rank: node["min_rank"].as_u64().unwrap_or(0).min(4) as u8,
            }
        };
        Self {
            looks: PlateStatus::ALL.map(look),
            fill_alpha: plate["fill_alpha"].as_f64().unwrap_or(0.84) as f32,
            border_px: plate["border_px"].as_f64().unwrap_or(1.5) as f32,
            max_pastilles: plate["pastilles"]["max"].as_u64().unwrap_or(3).min(3) as usize,
            size_px: plate["pastilles"]["size_px"].as_f64().unwrap_or(16.0) as f32,
            pastilles: PastilleKind::ALL.map(pill),
        }
    }

    pub fn look(&self, status: PlateStatus) -> StatusLook {
        self.looks[status.index()]
    }

    /// Border colour: the status colour, or in the Diplomacy mode the colour of the exact
    /// stance (same palette as the borders).
    pub fn border(&self, status: PlateStatus, stance_key: &str, diplomacy: bool) -> Srgba {
        if diplomacy && let Some(colour) = modes::diplomacy_color(stance_key) {
            return colour;
        }
        self.look(status).border
    }
}

/// Figures of a place the pills draw on (read from the core's `SettlementState`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PlateFacts {
    pub garrison: u32,
    pub buildings: usize,
    pub fortification: u8,
    pub siege: Option<SiegeFacts>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SiegeFacts {
    pub attacker: String,
    pub turns_left: u32,
    pub breach: u8,
}

/// A pill to draw.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pastille {
    pub kind: PastilleKind,
    pub text: String,
    /// BBCode of the tooltip.
    pub tip: String,
}

/// Short form of a count: `850`, `1,2k`, `12k`.
pub fn compact_count(n: u32) -> String {
    match n {
        0..=999 => n.to_string(),
        1000..=9999 => {
            let tenths = (n + 50) / 100;
            if tenths % 10 == 0 {
                format!("{}k", tenths / 10)
            } else {
                format!("{},{}k", tenths / 10, tenths % 10)
            }
        }
        _ => format!("{}k", (n + 500) / 1000),
    }
}

/// The pills of a place of `rank`: siege first (threat), then garrison, then buildings, at most
/// `style.max_pastilles`, each only from its minimal rank.
pub fn pastilles(facts: &PlateFacts, rank: u8, style: &PlateStyle) -> Vec<Pastille> {
    let allowed = |kind: PastilleKind| rank >= style.pastilles[kind.index()].min_rank;
    let mut out = Vec::new();
    if let Some(siege) = &facts.siege
        && allowed(PastilleKind::Siege)
    {
        out.push(Pastille {
            kind: PastilleKind::Siege,
            text: siege.turns_left.to_string(),
            tip: format!(
                "[b]Siège en cours[/b]\nAssiégeant : {}\nCapitulation estimée dans {} tour(s)\nBrèche : {} %",
                siege.attacker, siege.turns_left, siege.breach
            ),
        });
    }
    if facts.garrison > 0 && allowed(PastilleKind::Garrison) {
        out.push(Pastille {
            kind: PastilleKind::Garrison,
            text: compact_count(facts.garrison),
            tip: format!(
                "[b]Garnison[/b]\n{} hommes\nFortification : niveau {}",
                facts.garrison, facts.fortification
            ),
        });
    }
    if facts.buildings > 0 && allowed(PastilleKind::Buildings) {
        out.push(Pastille {
            kind: PastilleKind::Buildings,
            text: facts.buildings.to_string(),
            tip: format!("[b]Bâtiments[/b]\n{} bâtiment(s) construits", facts.buildings),
        });
    }
    out.truncate(style.max_pastilles);
    out
}

// ---- ECS ---------------------------------------------------------------------------------

/// Children of a plate root that this module drives.
#[derive(Component)]
pub struct PlateParts {
    pub name: Entity,
    pub pastilles: Entity,
}

/// What the plate shows now (change detection and dump).
#[derive(Component, Default)]
pub struct PlateState {
    pub status: Option<PlateStatus>,
    pub signature: String,
    pub pastilles: Vec<PastilleKind>,
}

#[derive(Component)]
struct PastilleNode;

/// Plates in the `campaign_markers.plates` dump section.
#[derive(Debug, Default)]
pub struct PlateStats {
    pub by_status: [usize; 4],
    pub pastilles_by_kind: [usize; 3],
    pub style_loaded: bool,
    pub samples: Vec<Value>,
}

impl PlateStats {
    pub fn dump(&self) -> Value {
        let statuses: serde_json::Map<String, Value> = PlateStatus::ALL
            .iter()
            .map(|s| (s.key().to_owned(), json!(self.by_status[s.index()])))
            .collect();
        let kinds: serde_json::Map<String, Value> = PastilleKind::ALL
            .iter()
            .map(|k| (k.key().to_owned(), json!(self.pastilles_by_kind[k.index()])))
            .collect();
        json!({
            "style_from_data": self.style_loaded,
            "by_status": statuses,
            "pastilles": kinds,
            "samples": self.samples,
        })
    }
}

/// Facts of the place `id` from the core.
fn facts_of(sim: &Sim, id: &str) -> Option<PlateFacts> {
    let state = sim.state.as_ref()?;
    let settlement = state.settlement_state(&data_model::SettlementId::new(id).ok()?)?;
    let siege = settlement.siege.as_ref().map(|s| SiegeFacts {
        attacker: sim.data.as_ref().map_or_else(
            || s.attacker.to_string(),
            |data| data.faction_name(&s.attacker),
        ),
        turns_left: s.turns_left,
        breach: s.breach,
    });
    Some(PlateFacts {
        garrison: settlement.garrison_strength(),
        buildings: settlement.buildings.len(),
        fortification: settlement.fortification_level,
        siege,
    })
}

#[allow(clippy::too_many_arguments)]
pub(super) fn update_plates(
    mut commands: Commands,
    time: Res<Time>,
    mut timer: Local<f32>,
    mut style: Local<Option<PlateStyle>>,
    (tuning, overlay, mode): (
        Option<Res<Tuning>>,
        Option<Res<OverlayState>>,
        Option<Res<MapModeState>>,
    ),
    sim: Res<Sim>,
    theme: Res<UiTheme>,
    (places, pins, ground, camera): (
        Res<Places>,
        Res<Pins>,
        GroundHeight,
        Query<&GlobalTransform, With<Camera3d>>,
    ),
    mut roots: Query<(
        &PlaceLabel,
        &PlateParts,
        &mut PlateState,
        &mut BackgroundColor,
        &mut BorderColor,
    )>,
    mut names: Query<&mut TextColor>,
    mut boxes: Query<(&mut Node, Option<&Children>)>,
    mut stats: ResMut<MarkerStats>,
) {
    *timer += time.delta_secs();
    if *timer < REFRESH_SECS {
        return;
    }
    *timer = 0.0;
    let (Some(overlay), Some(eye)) = (overlay, camera.iter().next().map(|t| t.translation()))
    else {
        return;
    };
    if style.is_none() || tuning.as_ref().is_some_and(|t| t.is_changed()) {
        *style = Some(PlateStyle::from_cues(
            tuning.as_ref().map_or(&Value::Null, |t| &t.cues),
        ));
        stats.plates.style_loaded = tuning
            .as_ref()
            .is_some_and(|t| t.cues["town"]["plate"].is_object());
    }
    let Some(style) = style.as_ref() else { return };
    let cues = tuning.as_ref().map_or(&Value::Null, |t| &t.cues);
    let diplomacy = mode.as_ref().is_some_and(|m| m.mode == MapMode::Diplomacy);
    let mut by_status = [0usize; 4];
    let mut by_kind = [0usize; 3];
    let mut samples: Vec<(u8, Value)> = Vec::new();
    for (label, parts, mut state, mut background, mut border) in &mut roots {
        let Some(place) = places.places.get(label.index) else {
            continue;
        };
        let stance = if place.controller.is_empty() {
            ""
        } else {
            overlay.view.stance_of(&place.controller)
        };
        let status = status_of(stance, cues);
        by_status[status.index()] += 1;
        let facts = facts_of(&sim, &place.id).unwrap_or_default();
        let list = pastilles(&facts, place.rank, style);
        for p in &list {
            by_kind[p.kind.index()] += 1;
        }
        let world = placement::px_to_world(place.px, super::ground_y(&ground, place.px));
        let near = pins.indices.contains(&label.index)
            || eye.distance(world) < places.catalog.shield_until(place.rank);
        if samples.len() < 24 && !list.is_empty() {
            samples.push((
                place.rank,
                json!({
                    "id": place.id, "status": status.key(),
                    "pastilles": list.iter().map(|p| json!({"kind": p.kind.key(), "text": p.text})).collect::<Vec<_>>(),
                }),
            ));
        }
        let signature = format!(
            "{}|{stance}|{diplomacy}|{near}|{}",
            status.key(),
            list.iter()
                .map(|p| format!("{}:{}", p.kind.key(), p.text))
                .collect::<Vec<_>>()
                .join(",")
        );
        if state.signature == signature {
            continue;
        }
        state.signature = signature;
        state.status = Some(status);
        let look = style.look(status);
        *background = BackgroundColor(look.fill.with_alpha(style.fill_alpha).into());
        *border = BorderColor::all(Color::from(style.border(status, stance, diplomacy)));
        if let Ok(mut ink) = names.get_mut(parts.name) {
            ink.0 = look.ink.into();
        }
        let Ok((mut node, children)) = boxes.get_mut(parts.pastilles) else {
            continue;
        };
        node.display = if near && !list.is_empty() {
            Display::Flex
        } else {
            Display::None
        };
        if let Some(children) = children {
            for child in children.iter() {
                commands.entity(child).despawn();
            }
        }
        state.pastilles = list.iter().map(|p| p.kind).collect();
        for pill in &list {
            let colour = style.pastilles[pill.kind.index()].color;
            let child = commands
                .spawn((
                    Name::new(format!("pastille:{}", pill.kind.key())),
                    PastilleNode,
                    Node {
                        min_width: Val::Px(style.size_px),
                        height: Val::Px(style.size_px),
                        padding: UiRect::horizontal(Val::Px(3.0)),
                        border: UiRect::all(Val::Px(1.0)),
                        border_radius: BorderRadius::MAX,
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(colour.into()),
                    BorderColor::all(Color::from(look.ink.with_alpha(0.8))),
                    Tooltip::new(&pill.tip),
                    ChildOf(parts.pastilles),
                ))
                .id();
            commands.spawn((
                Text::new(pill.text.clone()),
                theme.text_font(true, false, style.size_px * 0.62),
                TextColor(Color::srgb(0.97, 0.94, 0.85)),
                ChildOf(child),
            ));
        }
    }
    samples.sort_by_key(|(rank, _)| std::cmp::Reverse(*rank));
    stats.plates.by_status = by_status;
    stats.plates.pastilles_by_kind = by_kind;
    stats.plates.samples = samples.into_iter().take(6).map(|(_, v)| v).collect();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cues() -> Value {
        serde_json::from_str(include_str!("../../../../data/map/stance_cues.json")).unwrap()
    }

    #[test]
    fn status_follows_the_stance_categories() {
        let cues = cues();
        assert_eq!(status_of("self", &cues), PlateStatus::Own);
        assert_eq!(status_of("war", &cues), PlateStatus::Enemy);
        assert_eq!(status_of("ally", &cues), PlateStatus::Ally);
        assert_eq!(status_of("vassal", &cues), PlateStatus::Ally);
        assert_eq!(status_of("neutral", &cues), PlateStatus::Neutral);
        assert_eq!(status_of("", &cues), PlateStatus::Neutral);
    }

    #[test]
    fn data_gives_distinct_status_colours_and_red_only_for_enemies() {
        let style = PlateStyle::from_cues(&cues());
        let borders: Vec<_> = PlateStatus::ALL
            .iter()
            .map(|s| style.look(*s).border)
            .collect();
        for (i, a) in borders.iter().enumerate() {
            for b in &borders[i + 1..] {
                assert_ne!(a, b);
            }
        }
        let red = |c: Srgba| c.red > 0.6 && c.green < 0.3 && c.blue < 0.3;
        for status in PlateStatus::ALL {
            assert_eq!(
                red(style.look(status).border),
                status == PlateStatus::Enemy,
                "{status:?}"
            );
        }
        // The border palette is the one of the borders (ADR 0155).
        let cues = cues();
        assert_eq!(
            style.look(PlateStatus::Own).border,
            modes::hex(cues["border"]["self"].as_str().unwrap()).unwrap()
        );
        assert_eq!(
            style.look(PlateStatus::Enemy).border,
            modes::hex(cues["border"]["enemy"].as_str().unwrap()).unwrap()
        );
        assert_eq!(
            style.look(PlateStatus::Ally).border,
            modes::hex(cues["border"]["friend"].as_str().unwrap()).unwrap()
        );
    }

    #[test]
    fn diplomacy_mode_uses_the_stance_colour_for_the_border() {
        let style = PlateStyle::from_cues(&cues());
        assert_eq!(
            style.border(PlateStatus::Neutral, "tension", true),
            modes::diplomacy_color("tension").unwrap()
        );
        assert_eq!(
            style.border(PlateStatus::Neutral, "tension", false),
            style.look(PlateStatus::Neutral).border
        );
    }

    #[test]
    fn missing_data_falls_back_to_visible_colours() {
        let style = PlateStyle::from_cues(&Value::Null);
        assert!(style.fill_alpha > 0.0);
        assert!(style.look(PlateStatus::Own).ink.alpha > 0.0);
        assert_eq!(style.max_pastilles, 3);
    }

    #[test]
    fn counts_are_compact() {
        assert_eq!(compact_count(0), "0");
        assert_eq!(compact_count(850), "850");
        assert_eq!(compact_count(1000), "1k");
        assert_eq!(compact_count(1240), "1,2k");
        assert_eq!(compact_count(12_400), "12k");
    }

    fn facts() -> PlateFacts {
        PlateFacts {
            garrison: 420,
            buildings: 5,
            fortification: 3,
            siege: Some(SiegeFacts {
                attacker: "Angleterre".into(),
                turns_left: 4,
                breach: 20,
            }),
        }
    }

    #[test]
    fn pastilles_put_the_siege_first_and_respect_rank_and_max() {
        let style = PlateStyle::from_cues(&cues());
        let all = pastilles(&facts(), 4, &style);
        let kinds: Vec<_> = all.iter().map(|p| p.kind).collect();
        assert_eq!(
            kinds,
            [
                PastilleKind::Siege,
                PastilleKind::Garrison,
                PastilleKind::Buildings
            ]
        );
        assert!(all[0].tip.contains("Angleterre"));
        assert_eq!(all[1].text, "420");
        // A hamlet shows the siege only (garrison from rank 1, buildings from rank 2).
        let hamlet = pastilles(&facts(), 0, &style);
        assert_eq!(hamlet.len(), 1);
        assert_eq!(hamlet[0].kind, PastilleKind::Siege);
        let mut capped = style.clone();
        capped.max_pastilles = 2;
        assert_eq!(pastilles(&facts(), 4, &capped).len(), 2);
        // Nothing to say: no pill.
        assert!(pastilles(&PlateFacts::default(), 4, &style).is_empty());
    }
}
