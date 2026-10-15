//! Structured tooltips (ADR 0109 / IB): port of the section model of
//! game/scripts/ui/tooltip_view.gd and the spec shape of game/scripts/ui/rich_tooltip.gd.
//!
//! A [`TooltipSpec`] is presentation-only data (no game rule): header (title, subtitle, icon),
//! featured figures (`headline`), effects (▲ good / ▼ bad / • neutral), stats grid, strengths and
//! weaknesses, conditions (`requires`, `warnings`), detail lines, flavour and a footer (cost,
//! upkeep, time). Three ways to use it:
//!
//! - [`TooltipSpec::blocks`]: the sections to show, short (hover) or detailed (locked bubble);
//! - [`TooltipSpec::to_bbcode`]: flat BBCode for the kit's [`Tooltip`] / [`TooltipRegistry`]
//!   (effect labels become `ib:rule:<key>` links when `data/ui/tooltips.json` explains them);
//! - [`tooltip_view`]: the sectioned widget (note frame, header band, rules between blocks).
//!
//! Texts come from `data/ui/tooltips.json` ([`TooltipTexts`]: `effects`, `stats`, `gauges`, `hud`,
//! `plain`, … by key, each `{label|title, body}`); style (width, header icon, kind colours) from
//! `data/ui/tooltip_style.json` ([`TooltipStyle`]). [`register_rule_texts`] fills the registry with
//! the rule bubbles (`ib:rule:<key>`). `{rule.<key>}` placeholders are resolved by the caller
//! through [`fill_rules`] (the values live in the core).

use std::collections::HashMap;

use bevy::prelude::*;
use serde::Deserialize;
use serde_json::Value;

use super::UiTheme;
use super::icons::icon;
use super::richtext::{RichStyle, rich_text};
use super::style::{self, UiType};
use super::tooltip::{HINT, TooltipRegistry};
use super::widgets::{KitWidget, MIN_FRAME_HEIGHT};
use crate::paths;

pub const GREEN: &str = "#2a6a2a";
pub const RED: &str = "#8b1a1a";
pub const MUTED_HEX: &str = "#6b5a40";
/// Flavour ink, paler than [`MUTED_HEX`] (`TooltipView.FLAVOUR_COLOR`).
pub const FLAVOUR_HEX: &str = "#7d6a4e";
const CHECK: &str = "✓";
const CROSS: &str = "✗";
const UNKNOWN: &str = "•";
const WARNING: &str = "⚠";
const HEADLINE_ICON_PX: f32 = 28.0;
/// Ids of headline figures without their own icon (`HEADLINE_ICON_FALLBACK`).
const HEADLINE_ICON_FALLBACK: [(&str, &str); 2] = [
    ("stat_ranged", "battle_state_shoot"),
    ("stat_melee", "battle_state_melee"),
];
/// Footer icons (`_footer`).
const FOOTER_ICONS: [(&str, &str, &str); 3] = [
    ("cost", "hud_treasury", "Coût"),
    ("upkeep", "hud_income", "Entretien"),
    ("time", "hud_end_turn", "Durée"),
];

// ---- spec ----------------------------------------------------------------------------------

/// A featured figure ("badge"): icon, big value, caption.
#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct Headline {
    pub key: String,
    pub stat: String,
    pub icon: String,
    pub label: String,
    pub value: String,
    pub sign: i32,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct StatRow {
    pub key: String,
    pub label: String,
    pub value: String,
}

/// An effect line: `text`, or `label` + `before` → `after` (+ `value`), with its sign.
#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct Effect {
    pub key: String,
    pub text: String,
    pub label: String,
    pub value: String,
    pub sign: i32,
    pub before: Option<f64>,
    pub after: Option<f64>,
    pub equilibrium: bool,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct Traits {
    pub strengths: Vec<String>,
    pub weaknesses: Vec<String>,
    pub abilities: Vec<String>,
}

/// A requirement; `met` is `None` when the state is unknown.
#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct Requirement {
    pub text: String,
    pub met: Option<bool>,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct Footer {
    pub cost: String,
    pub upkeep: String,
    pub time: String,
}

/// Tooltip content (`RichTooltip` spec dictionary).
#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct TooltipSpec {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub subtitle: String,
    pub icon: String,
    pub headline: Vec<Headline>,
    pub stats: Vec<StatRow>,
    pub effects: Vec<Effect>,
    pub traits: Traits,
    pub requires: Vec<Requirement>,
    pub requires_label: Option<String>,
    pub warnings: Vec<String>,
    pub flavour: String,
    pub footer: Footer,
    pub detail: Vec<String>,
}

/// A body section, in display order.
#[derive(Debug, Clone, PartialEq)]
pub enum Block {
    Headline(Vec<Headline>),
    /// BBCode lines, one per effect.
    Effects(Vec<String>),
    Stats(Vec<StatRow>),
    Traits(Vec<String>),
    Conditions(Vec<String>),
    Detail(Vec<String>),
    Flavour(String),
}

impl Block {
    /// Node name (`Headline`, `Effects`, `Stats`…), as in Godot.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Headline(_) => "Headline",
            Self::Effects(_) => "Effects",
            Self::Stats(_) => "Stats",
            Self::Traits(_) => "Traits",
            Self::Conditions(_) => "Conditions",
            Self::Detail(_) => "Detail",
            Self::Flavour(_) => "Flavour",
        }
    }

    /// Body lines the block counts for (`short_max_body_lines`).
    pub fn lines(&self) -> usize {
        match self {
            Self::Headline(_) | Self::Flavour(_) => 1,
            Self::Effects(lines)
            | Self::Traits(lines)
            | Self::Conditions(lines)
            | Self::Detail(lines) => lines.len(),
            Self::Stats(rows) => rows.len().div_ceil(2),
        }
    }
}

/// `▲ ` / `▼ ` / nothing (`RichTooltip.mark`).
pub fn mark(sign: i32) -> &'static str {
    match sign.signum() {
        1 => "▲ ",
        -1 => "▼ ",
        _ => "",
    }
}

fn number(value: f64) -> String {
    let rounded = (value * 100.0).round() / 100.0;
    if (rounded - rounded.round()).abs() < 1e-9 {
        format!("{}", rounded as i64)
    } else {
        format!("{rounded}").replace('.', ",")
    }
}

impl Effect {
    /// "Moral 60 → 65 (+5)", "Santé : équilibre 50 → 55 (+5)" or the plain `text`.
    pub fn line(&self) -> String {
        match (self.before, self.after) {
            (Some(before), Some(after)) if !self.label.is_empty() => {
                let label = if self.equilibrium {
                    format!("{} : équilibre", self.label)
                } else {
                    self.label.clone()
                };
                format!(
                    "{label} {} → {} ({})",
                    number(before),
                    number(after),
                    self.value
                )
            }
            _ => self.text.clone(),
        }
    }
}

/// `[url=ib:rule:<key>]label[/url]`.
pub fn rule_link(key: &str, label: &str) -> String {
    format!("[url=ib:rule:{key}]{label}[/url]")
}

impl TooltipSpec {
    pub fn from_json(text: &str) -> Result<Self, String> {
        serde_json::from_str(text).map_err(|e| e.to_string())
    }

    /// Effect line with its label linked to the rule bubble `key` when `texts` knows it
    /// (`_linked_effect`).
    fn linked_effect(effect: &Effect, texts: &TooltipTexts) -> String {
        let line = effect.line();
        let Some(rule) = texts.rule("effects", &effect.key) else {
            return line;
        };
        let label = if effect.label.is_empty() {
            rule.label.clone()
        } else {
            effect.label.clone()
        };
        link_first(&line, &effect.key, &label)
    }

    /// Sections to show (`blocks_for`): short = hover (headline, effects, traits, unmet
    /// conditions, warnings); detailed = locked bubble (adds stats, abilities, met requirements,
    /// detail lines, flavour).
    pub fn blocks(&self, texts: &TooltipTexts, detailed: bool) -> Vec<Block> {
        let mut blocks = Vec::new();
        if !self.headline.is_empty() {
            blocks.push(Block::Headline(self.headline.clone()));
        }
        let effects: Vec<String> = self
            .effects
            .iter()
            .map(|effect| {
                let text = Self::linked_effect(effect, texts);
                match effect.sign.signum() {
                    1 => format!("[color={GREEN}]▲[/color] {text}"),
                    -1 => format!("[color={RED}]▼[/color] {text}"),
                    _ => format!("{UNKNOWN} {text}"),
                }
            })
            .collect();
        if !effects.is_empty() {
            blocks.push(Block::Effects(effects));
        }
        if detailed {
            let featured: Vec<&str> = self.headline.iter().map(|h| h.stat.as_str()).collect();
            let stats: Vec<StatRow> = self
                .stats
                .iter()
                .filter(|stat| !featured.contains(&stat.key.as_str()))
                .cloned()
                .collect();
            if !stats.is_empty() {
                blocks.push(Block::Stats(stats));
            }
        }
        let mut traits = Vec::new();
        if !self.traits.strengths.is_empty() {
            traits.push(format!(
                "[color={GREEN}]▲ Forces : {}[/color]",
                self.traits.strengths.join(", ")
            ));
        }
        if !self.traits.weaknesses.is_empty() {
            traits.push(format!(
                "[color={RED}]▼ Faiblesses : {}[/color]",
                self.traits.weaknesses.join(", ")
            ));
        }
        if detailed && !self.traits.abilities.is_empty() {
            traits.push(format!(
                "{UNKNOWN} Capacités : {}",
                self.traits.abilities.join(", ")
            ));
        }
        if !traits.is_empty() {
            blocks.push(Block::Traits(traits));
        }
        let label = self.requires_label.as_deref().unwrap_or("Requiert");
        let mut conditions = Vec::new();
        for requirement in &self.requires {
            match requirement.met {
                None => {
                    if detailed || !self.warnings.is_empty() {
                        conditions.push(format!(
                            "[color={MUTED_HEX}]{UNKNOWN}[/color] {label} : {}",
                            requirement.text
                        ));
                    }
                }
                Some(true) => {
                    if detailed {
                        conditions.push(format!(
                            "[color={GREEN}]{CHECK}[/color] {label} : {}",
                            requirement.text
                        ));
                    }
                }
                Some(false) => conditions.push(format!(
                    "[color={RED}]{CROSS} {label} : {}[/color]",
                    requirement.text
                )),
            }
        }
        for warning in &self.warnings {
            conditions.push(format!("[color={RED}]{WARNING} {warning}[/color]"));
        }
        if !conditions.is_empty() {
            blocks.push(Block::Conditions(conditions));
        }
        if detailed {
            let detail: Vec<String> = self
                .detail
                .iter()
                .filter(|line| !line.is_empty())
                .cloned()
                .collect();
            if !detail.is_empty() {
                blocks.push(Block::Detail(detail));
            }
            if !self.flavour.is_empty() {
                blocks.push(Block::Flavour(format!(
                    "[color={FLAVOUR_HEX}][i]{}[/i][/color]",
                    self.flavour
                )));
            }
        }
        blocks
    }

    /// Footer parts "icon text" (cost, upkeep, time), skipping `cost` when it is a headline.
    pub fn footer_parts(&self) -> Vec<(&'static str, &'static str, String)> {
        let featured_cost = self.headline.iter().any(|h| h.key == "cost");
        FOOTER_ICONS
            .iter()
            .filter_map(|(key, icon_id, label)| {
                let text = match *key {
                    "cost" => &self.footer.cost,
                    "upkeep" => &self.footer.upkeep,
                    _ => &self.footer.time,
                };
                if text.is_empty() || (*key == "cost" && featured_cost) {
                    None
                } else {
                    Some((*icon_id, *label, text.clone()))
                }
            })
            .collect()
    }

    /// Display name: the data's name for `id` when known, else `title`.
    pub fn display_title(&self, texts: &TooltipTexts) -> String {
        texts
            .any_title(&self.id)
            .unwrap_or_else(|| self.title.clone())
    }

    /// Flat BBCode for [`super::tooltip::Tooltip`]: bold title, italic subtitle, then the blocks
    /// (stats as "label : value" lines, headline as "label value" lines), then the footer.
    pub fn to_bbcode(&self, texts: &TooltipTexts, detailed: bool) -> String {
        let mut out = format!("[b]{}[/b]", self.display_title(texts));
        if !self.subtitle.is_empty() {
            out.push_str(&format!("\n[i]{}[/i]", self.subtitle));
        }
        for block in self.blocks(texts, detailed) {
            match block {
                Block::Headline(items) => {
                    for item in items {
                        let value = if item.sign == 0 {
                            format!("[b]{}[/b]", item.value)
                        } else {
                            let color = if item.sign > 0 { GREEN } else { RED };
                            format!(
                                "[b][color={color}]{}{}[/color][/b]",
                                mark(item.sign),
                                item.value
                            )
                        };
                        out.push_str(&format!("\n{value} {}", item.label));
                    }
                }
                Block::Stats(rows) => {
                    for row in rows {
                        out.push_str(&format!(
                            "\n{} : {}",
                            rule_or_plain(texts, "stats", &row.key, &row.label),
                            row.value
                        ));
                    }
                }
                Block::Effects(lines)
                | Block::Traits(lines)
                | Block::Conditions(lines)
                | Block::Detail(lines) => {
                    for line in lines {
                        out.push('\n');
                        out.push_str(&line);
                    }
                }
                Block::Flavour(line) => {
                    out.push('\n');
                    out.push_str(&line);
                }
            }
        }
        let parts: Vec<String> = self
            .footer_parts()
            .into_iter()
            .map(|(_, label, text)| format!("{label} : {text}"))
            .collect();
        if !parts.is_empty() {
            out.push_str(&format!("\n[right]{}[/right]", parts.join("  ·  ")));
        }
        out
    }
}

/// `text` with the first occurrence of `label` turned into the rule link `key`.
fn link_first(text: &str, key: &str, label: &str) -> String {
    if label.is_empty() || key.is_empty() || text.contains(&format!("[url=ib:rule:{key}]")) {
        return text.to_owned();
    }
    match text.find(label) {
        Some(at) => format!(
            "{}{}{}",
            &text[..at],
            rule_link(key, label),
            &text[at + label.len()..]
        ),
        None => text.to_owned(),
    }
}

fn rule_or_plain(texts: &TooltipTexts, block: &str, key: &str, label: &str) -> String {
    if texts.rule(block, key).is_some() {
        rule_link(key, label)
    } else {
        label.to_owned()
    }
}

/// Replaces `{rule.<key>}` by `lookup(key)`; unknown keys stay as written.
pub fn fill_rules(text: &str, lookup: impl Fn(&str) -> Option<String>) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("{rule.") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 6..];
        match after.find('}') {
            Some(end) => {
                let key = &after[..end];
                match lookup(key) {
                    Some(value) => out.push_str(&value),
                    None => out.push_str(&rest[start..start + 6 + end + 1]),
                }
                rest = &after[end + 1..];
            }
            None => {
                out.push_str(&rest[start..]);
                rest = "";
            }
        }
    }
    out.push_str(rest);
    out
}

// ---- data/ui/tooltips.json -----------------------------------------------------------------

/// One entry of `tooltips.json`: `label` (effects, stats) or `title` (gauges, hud, plain…), `body`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TextEntry {
    pub label: String,
    pub title: String,
    pub body: String,
}

/// Blocks of `tooltips.json` whose entries are rule bubbles (`RULE_BLOCKS`).
pub const RULE_BLOCKS: [&str; 3] = ["effects", "stats", "gauges"];
/// Blocks searched for a display title by id.
const TITLE_BLOCKS: [&str; 7] = [
    "hud",
    "gauges",
    "categories",
    "classes",
    "branches",
    "abilities",
    "plain",
];

/// Parsed `data/ui/tooltips.json`.
#[derive(Debug, Clone, Default)]
pub struct TooltipTexts {
    blocks: HashMap<String, HashMap<String, TextEntry>>,
}

impl TooltipTexts {
    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        let value: Value = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        let root = value.as_object().ok_or("not an object")?;
        let mut blocks = HashMap::new();
        for (block, entries) in root {
            let Some(entries) = entries.as_object() else {
                continue;
            };
            let mut table = HashMap::new();
            for (key, entry) in entries {
                let text = |field: &str| {
                    entry
                        .get(field)
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_owned()
                };
                table.insert(
                    key.clone(),
                    TextEntry {
                        label: text("label"),
                        title: text("title"),
                        body: text("body"),
                    },
                );
            }
            blocks.insert(block.clone(), table);
        }
        Ok(Self { blocks })
    }

    /// Reads `data/ui/tooltips.json` from the repository.
    pub fn load() -> Result<Self, String> {
        let root = paths::find_root()?;
        let path = paths::data_dir(&root).join("ui/tooltips.json");
        let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        Self::parse(&bytes)
    }

    pub fn entry(&self, block: &str, key: &str) -> Option<&TextEntry> {
        self.blocks.get(block)?.get(key)
    }

    /// Rule bubble entry of `key` in a rule block (`effects`, `stats`, `gauges`).
    pub fn rule(&self, block: &str, key: &str) -> Option<&TextEntry> {
        self.entry(block, key)
            .filter(|entry| !entry.label.is_empty() || !entry.title.is_empty())
    }

    /// Title of `id` in the title blocks (`hud`, `gauges`, `plain`…).
    pub fn any_title(&self, id: &str) -> Option<String> {
        if id.is_empty() {
            return None;
        }
        TITLE_BLOCKS
            .iter()
            .find_map(|block| self.entry(block, id))
            .map(|entry| entry.title.clone())
            .filter(|title| !title.is_empty())
    }

    pub fn len(&self) -> usize {
        self.blocks.values().map(HashMap::len).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Registers every rule bubble of `texts` (`ib:rule:<key>` → "[b]label[/b]\nbody") so that the
/// links of [`TooltipSpec::to_bbcode`] open them. `resolve` fills `{rule.…}` placeholders.
pub fn register_rule_texts(
    registry: &mut TooltipRegistry,
    texts: &TooltipTexts,
    resolve: impl Fn(&str) -> Option<String>,
) -> usize {
    let mut count = 0;
    for block in RULE_BLOCKS {
        let Some(entries) = texts.blocks.get(block) else {
            continue;
        };
        for (key, entry) in entries {
            let name = if entry.label.is_empty() {
                &entry.title
            } else {
                &entry.label
            };
            let body = fill_rules(&entry.body, &resolve);
            registry.register(&format!("ib:rule:{key}"), &format!("[b]{name}[/b]\n{body}"));
            count += 1;
        }
    }
    count
}

// ---- data/ui/tooltip_style.json ------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct TooltipStyle {
    pub width_px: f32,
    pub header_icon_px: f32,
    pub short_max_body_lines: usize,
    pub kind_colors: HashMap<String, String>,
}

impl Default for TooltipStyle {
    fn default() -> Self {
        Self {
            width_px: 360.0,
            header_icon_px: 48.0,
            short_max_body_lines: 8,
            kind_colors: HashMap::new(),
        }
    }
}

impl TooltipStyle {
    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        serde_json::from_slice(bytes).map_err(|e| e.to_string())
    }

    pub fn load() -> Result<Self, String> {
        let root = paths::find_root()?;
        let path = paths::data_dir(&root).join("ui/tooltip_style.json");
        let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        Self::parse(&bytes)
    }

    /// Colour of a `kind` (ink when unknown).
    pub fn kind_color(&self, kind: &str) -> Color {
        self.kind_colors
            .get(kind)
            .and_then(|hex| Srgba::hex(hex).ok())
            .map_or(style::INK, Color::from)
    }
}

// ---- widget --------------------------------------------------------------------------------

/// Resolved id of a headline icon (`headline_icon`): own icon, `stat_<stat>`, then the fallback.
pub fn headline_icon(theme: &UiTheme, icon_id: &str, stat: &str) -> Option<String> {
    let first = if icon_id.is_empty() && !stat.is_empty() {
        format!("stat_{stat}")
    } else {
        icon_id.to_owned()
    };
    let fallback = HEADLINE_ICON_FALLBACK
        .iter()
        .find(|(from, _)| *from == first)
        .map(|(_, to)| (*to).to_owned());
    [Some(first), fallback]
        .into_iter()
        .flatten()
        .find(|id| !id.is_empty() && theme.icons.has(id))
}

/// Sectioned tooltip widget: note frame, header band (icon, title in the kind colour, subtitle),
/// then the blocks with a rule between them, then the footer (hint left, costs right). Node names
/// are those of Godot (`Header`, `Title`, `Subtitle`, `Headline`, `Effects`, `Stats`, `Traits`,
/// `Conditions`, `Detail`, `Flavour`, `Footer`, `Hint`, `Costs`, `Rule<Block>`).
pub fn tooltip_view(
    theme: &UiTheme,
    assets: &AssetServer,
    spec: &TooltipSpec,
    texts: &TooltipTexts,
    tooltip_style: &TooltipStyle,
    detailed: bool,
) -> impl Bundle {
    let rich = RichStyle::body(theme);
    let caption = RichStyle {
        size: UiType::Caption.size(),
        color: style::MUTED,
        ..rich.clone()
    };
    let kind_color = tooltip_style.kind_color(&spec.kind);
    let width = tooltip_style.width_px;
    let blocks = spec.blocks(texts, detailed);
    (
        Name::new("TooltipView"),
        KitWidget("tooltip_view"),
        Node {
            width: Val::Px(width),
            max_width: Val::Percent(100.0),
            min_height: Val::Px(MIN_FRAME_HEIGHT),
            padding: theme.inner_padding("tooltip", 4.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(6.0),
            ..default()
        },
        theme.sliced("tooltip"),
        Children::spawn(SpawnWith({
            let theme = theme.clone();
            let assets = assets.clone();
            let spec = spec.clone();
            let texts = texts.clone();
            let icon_px = tooltip_style.header_icon_px;
            move |parent: &mut bevy::ecs::relationship::RelatedSpawner<ChildOf>| {
                // Header band.
                parent
                    .spawn((
                        Name::new("Header"),
                        Node {
                            align_items: AlignItems::Center,
                            column_gap: Val::Px(8.0),
                            padding: UiRect::axes(Val::Px(8.0), Val::Px(4.0)),
                            border: UiRect::left(Val::Px(3.0)),
                            ..default()
                        },
                        BackgroundColor(style::PARCHMENT_DARK.with_alpha(0.35)),
                        BorderColor::all(kind_color),
                    ))
                    .with_children(|header| {
                        if !spec.icon.is_empty() && theme.icons.has(&spec.icon) {
                            header.spawn(icon(&theme, &assets, &spec.icon, icon_px));
                        }
                        header
                            .spawn(Node {
                                flex_direction: FlexDirection::Column,
                                flex_shrink: 1.0,
                                ..default()
                            })
                            .with_children(|column| {
                                column.spawn((
                                    Name::new("Title"),
                                    Text::new(spec.display_title(&texts)),
                                    theme.text_font(true, false, UiType::Heading.size()),
                                    TextColor(kind_color),
                                ));
                                if !spec.subtitle.is_empty() {
                                    column.spawn((
                                        Name::new("Subtitle"),
                                        Text::new(spec.subtitle.clone()),
                                        theme.text_font(false, true, UiType::Caption.size()),
                                        TextColor(style::MUTED),
                                    ));
                                }
                            });
                    });
                let mut first = true;
                for block in blocks {
                    if !first {
                        parent.spawn(rule(&format!("Rule{}", block.name())));
                    } else {
                        parent.spawn(rule("RuleHeader"));
                    }
                    first = false;
                    spawn_block(parent, &theme, &assets, &rich, &texts, &block);
                }
                parent.spawn(rule("RuleFooter"));
                let footer = spec.footer_parts();
                parent
                    .spawn((
                        Name::new("Footer"),
                        Node {
                            justify_content: JustifyContent::SpaceBetween,
                            align_items: AlignItems::End,
                            column_gap: Val::Px(8.0),
                            ..default()
                        },
                    ))
                    .with_children(|row| {
                        row.spawn((
                            Name::new("Hint"),
                            Text::new(HINT),
                            theme.text_font(false, false, UiType::Caption.size()),
                            TextColor(style::MUTED),
                        ));
                        if !footer.is_empty() {
                            row.spawn((
                                Name::new("Costs"),
                                Node {
                                    column_gap: Val::Px(10.0),
                                    align_items: AlignItems::Center,
                                    ..default()
                                },
                            ))
                            .with_children(|costs| {
                                for (icon_id, label, text) in footer {
                                    costs
                                        .spawn(Node {
                                            column_gap: Val::Px(4.0),
                                            align_items: AlignItems::Center,
                                            ..default()
                                        })
                                        .with_children(|part| {
                                            if theme.icons.has(icon_id) {
                                                part.spawn(icon(&theme, &assets, icon_id, 14.0));
                                            } else {
                                                part.spawn((
                                                    Text::new(format!("{label} :")),
                                                    theme.text_font(
                                                        false,
                                                        false,
                                                        UiType::Caption.size(),
                                                    ),
                                                    TextColor(style::MUTED),
                                                ));
                                            }
                                            part.spawn((
                                                Text::new(text),
                                                theme.text_font(
                                                    false,
                                                    false,
                                                    UiType::Caption.size(),
                                                ),
                                                TextColor(caption.color),
                                            ));
                                        });
                                }
                            });
                        }
                    });
            }
        })),
    )
}

fn rule(name: &str) -> impl Bundle {
    (
        Name::new(name.to_owned()),
        Node {
            width: Val::Percent(100.0),
            height: Val::Px(1.0),
            ..default()
        },
        BackgroundColor(style::INK_SOFT.with_alpha(0.45)),
    )
}

fn spawn_block(
    parent: &mut bevy::ecs::relationship::RelatedSpawner<ChildOf>,
    theme: &UiTheme,
    assets: &AssetServer,
    rich: &RichStyle,
    texts: &TooltipTexts,
    block: &Block,
) {
    let name = Name::new(block.name());
    match block {
        Block::Headline(items) => {
            parent
                .spawn((
                    name,
                    Node {
                        column_gap: Val::Px(12.0),
                        ..default()
                    },
                ))
                .with_children(|row| {
                    for item in items {
                        row.spawn((
                            Name::new(format!("Badge_{}", item.key)),
                            Node {
                                column_gap: Val::Px(6.0),
                                align_items: AlignItems::Center,
                                flex_grow: 1.0,
                                ..default()
                            },
                        ))
                        .with_children(|badge| {
                            if let Some(id) = headline_icon(theme, &item.icon, &item.stat) {
                                badge.spawn(icon(theme, assets, &id, HEADLINE_ICON_PX));
                            }
                            badge
                                .spawn(Node {
                                    flex_direction: FlexDirection::Column,
                                    ..default()
                                })
                                .with_children(|column| {
                                    let color = match item.sign.signum() {
                                        1 => style::GOOD,
                                        -1 => style::POOR,
                                        _ => style::INK,
                                    };
                                    column.spawn((
                                        Name::new("Value"),
                                        Text::new(format!("{}{}", mark(item.sign), item.value)),
                                        theme.text_font(true, false, UiType::Title.size()),
                                        TextColor(color),
                                    ));
                                    column.spawn((
                                        Name::new("Caption"),
                                        Text::new(item.label.clone()),
                                        theme.text_font(false, false, UiType::Caption.size()),
                                        TextColor(style::MUTED),
                                    ));
                                });
                        });
                    }
                });
        }
        Block::Stats(rows) => {
            parent
                .spawn((
                    name,
                    Node {
                        display: Display::Grid,
                        grid_template_columns: RepeatedGridTrack::flex(2, 1.0),
                        column_gap: Val::Px(16.0),
                        ..default()
                    },
                ))
                .with_children(|grid| {
                    for row in rows {
                        grid.spawn(Node {
                            justify_content: JustifyContent::SpaceBetween,
                            ..default()
                        })
                        .with_children(|cell| {
                            cell.spawn(rich_text(
                                theme,
                                assets,
                                &rule_or_plain(texts, "stats", &row.key, &row.label),
                                rich,
                            ));
                            cell.spawn((
                                Text::new(row.value.clone()),
                                theme.text_font(false, false, UiType::Body.size()),
                                TextColor(style::INK),
                            ));
                        });
                    }
                });
        }
        Block::Effects(lines)
        | Block::Traits(lines)
        | Block::Conditions(lines)
        | Block::Detail(lines) => {
            parent
                .spawn(rich_text(theme, assets, &lines.join("\n"), rich))
                .insert(name);
        }
        Block::Flavour(line) => {
            parent
                .spawn(rich_text(theme, assets, line, rich))
                .insert(name);
        }
    }
}

// ---- tests ---------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const SPEC: &str = r#"{
        "id": "unit_knights", "kind": "unit", "title": "Chevaliers", "subtitle": "Cavalerie lourde",
        "headline": [{"key": "strength", "label": "Effectif", "value": "120", "sign": 0},
                      {"key": "melee", "stat": "melee", "label": "Mêlée", "value": "9", "sign": 1}],
        "stats": [{"key": "melee", "label": "Mêlée", "value": "9"}, {"key": "armor", "label": "Armure", "value": "7"}],
        "effects": [{"key": "army_morale", "label": "Moral des armées", "before": 60, "after": 65.5, "value": "+5,5", "sign": 1},
                     {"text": "Coût accru", "sign": -1}, {"text": "Neutre", "sign": 0}],
        "traits": {"strengths": ["charge"], "weaknesses": ["pluie"], "abilities": ["lance"]},
        "requires": [{"text": "Écurie", "met": true}, {"text": "Forge", "met": false}, {"text": "Fief"}],
        "warnings": ["Trésor insuffisant"],
        "flavour": "Fleur de la chevalerie.",
        "footer": {"cost": "300 ₶", "time": "2 saisons"},
        "detail": ["Une ligne", ""]
    }"#;

    fn texts() -> TooltipTexts {
        TooltipTexts::parse(
            r#"{"effects": {"army_morale": {"label": "Moral des armées", "body": "Tient la ligne {rule.x}."}},
                 "stats": {"armor": {"label": "Armure", "body": "Protège."}},
                 "hud": {"hud_treasury": {"title": "Trésor", "body": "Livres."}}}"#
                .as_bytes(),
        )
        .unwrap()
    }

    #[test]
    fn short_version_omits_stats_and_flavour() {
        let spec = TooltipSpec::from_json(SPEC).unwrap();
        let names: Vec<_> = spec
            .blocks(&texts(), false)
            .iter()
            .map(Block::name)
            .collect();
        assert_eq!(names, ["Headline", "Effects", "Traits", "Conditions"]);
    }

    #[test]
    fn detailed_version_adds_stats_without_featured_ones() {
        let spec = TooltipSpec::from_json(SPEC).unwrap();
        let blocks = spec.blocks(&texts(), true);
        let names: Vec<_> = blocks.iter().map(Block::name).collect();
        assert_eq!(
            names,
            [
                "Headline",
                "Effects",
                "Stats",
                "Traits",
                "Conditions",
                "Detail",
                "Flavour"
            ]
        );
        // `melee` is a headline stat: only `armor` is left in the grid.
        match &blocks[2] {
            Block::Stats(rows) => assert_eq!(rows.len(), 1),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn conditions_follow_state() {
        let spec = TooltipSpec::from_json(SPEC).unwrap();
        let short = spec.blocks(&texts(), false);
        let Block::Conditions(lines) = short.last().unwrap() else {
            panic!()
        };
        // Unmet requirement + warning + unknown (a warning exists); the met one is hidden.
        assert_eq!(lines.len(), 3);
        assert!(lines[0].contains("✗") && lines.iter().all(|l| !l.contains("Écurie")));
        let detailed = spec.blocks(&texts(), true);
        let Block::Conditions(lines) = &detailed[4] else {
            panic!()
        };
        assert_eq!(lines.len(), 4);
    }

    #[test]
    fn effect_line_and_link() {
        let spec = TooltipSpec::from_json(SPEC).unwrap();
        assert_eq!(spec.effects[0].line(), "Moral des armées 60 → 65,5 (+5,5)");
        let Block::Effects(lines) = &spec.blocks(&texts(), false)[1] else {
            panic!()
        };
        assert!(lines[0].contains("[url=ib:rule:army_morale]Moral des armées[/url]"));
        assert!(lines[0].starts_with("[color=#2a6a2a]▲"));
        assert!(lines[2].starts_with('•'));
    }

    #[test]
    fn bbcode_has_header_blocks_and_footer() {
        let spec = TooltipSpec::from_json(SPEC).unwrap();
        let text = spec.to_bbcode(&texts(), true);
        assert!(text.starts_with("[b]Chevaliers[/b]\n[i]Cavalerie lourde[/i]"));
        assert!(text.contains("[url=ib:rule:armor]Armure[/url] : 7"));
        assert!(text.ends_with("[right]Coût : 300 ₶  ·  Durée : 2 saisons[/right]"));
    }

    #[test]
    fn cost_headline_hides_footer_cost() {
        let mut spec = TooltipSpec::default();
        spec.footer.cost = "10".into();
        spec.headline.push(Headline {
            key: "cost".into(),
            ..default()
        });
        assert!(spec.footer_parts().is_empty());
    }

    #[test]
    fn registers_rule_bubbles_and_fills_placeholders() {
        let mut registry = TooltipRegistry::default();
        let count = register_rule_texts(&mut registry, &texts(), |key| {
            (key == "x").then(|| "12".to_owned())
        });
        assert_eq!(count, 2);
        assert!(
            registry
                .resolve("ib:rule:army_morale")
                .contains("Tient la ligne 12.")
        );
        assert_eq!(fill_rules("a {rule.y} b", |_| None), "a {rule.y} b");
    }

    #[test]
    fn real_data_files_parse() {
        let Ok(texts) = TooltipTexts::load() else {
            return; // no repository next to the test binary
        };
        assert!(texts.rule("effects", "unrest").is_some());
        assert!(texts.any_title("hud_treasury").is_some());
        let style = TooltipStyle::load().unwrap();
        assert_eq!(style.width_px, 360.0);
        assert_ne!(style.kind_color("unit"), style::INK);
    }
}
