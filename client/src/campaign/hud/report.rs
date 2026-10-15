//! Season report (port of `season_report.gd`): sorts the events of the turn that just resolved
//! into the five rubrics and formats them as BBCode for a modal. No game calculation.

use sim_campaign::GameEvent;

use super::text::hex;
use crate::ui::art_plates::{ArtPlates, Vignette};
use crate::ui::kit::fr;

/// Rubrics in order of urgency: (id, title).
pub const SECTIONS: [(&str, &str); 5] = [
    ("lands", "Vos terres"),
    ("treasury", "Trésor"),
    ("armies", "Armées"),
    ("works", "Constructions et recherches"),
    ("world", "Le monde"),
];

const MAX_PER_GROUP: usize = 8;

const LANDS: &[&str] = &[
    "province_captured",
    "siege_started",
    "siege_lifted",
    "raid",
    "revolt",
    "plague",
    "famine",
    "death",
    "succession",
    "no_heir",
    "birth",
    "marriage",
    "regency",
    "table",
    "medicine",
    "chivalry",
    "agent",
    "excommunication",
    "heresy",
    "vassal_rebellion",
    "victory",
    "defeat",
    "campaign_ended",
    "edict",
    "mission",
    "crusade",
    "royal_act",
    "level_up",
    "captain",
    "loyalty",
];
const TREASURY: &[&str] = &["bankruptcy", "coinage", "ransom", "trade"];
const ARMIES: &[&str] = &[
    "battle",
    "army_destroyed",
    "general_captured",
    "recruited",
    "attrition",
];
const WORKS: &[&str] = &["building_completed", "technology_researched"];
/// Kinds reported for any faction.
const WORLD_KINDS: &[&str] = &[
    "war_declared",
    "peace_signed",
    "faction_destroyed",
    "schism",
    "chronicle",
    "victory",
    "defeat",
    "campaign_ended",
    "succession",
    "excommunication",
];
/// Kinds reported for a faction of interest (at war with or allied to the player).
const WORLD_NEWS_KINDS: &[&str] = &[
    "war_declared",
    "peace_signed",
    "alliance_formed",
    "alliance_broken",
    "vassalage",
    "vassal_rebellion",
    "faction_destroyed",
    "schism",
    "excommunication",
    "succession",
    "death",
    "province_captured",
    "battle",
    "revolt",
    "plague",
    "chronicle",
    "victory",
    "defeat",
    "campaign_ended",
    "general_captured",
];

/// Snake-case key of an event kind.
pub fn kind_key(event: &GameEvent) -> String {
    serde_json::to_value(event.kind)
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap_or_default()
}

/// One report line.
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub text: String,
    pub loss: bool,
    pub gain: bool,
    /// A battle line (`battle` event).
    pub battle: bool,
    /// Snake-case event kind (picks the vignette).
    pub kind: String,
}

/// Reach of the world news (`interface/news_filter`, `NewsInterest.MODES`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NewsScope {
    /// Neighbours, allies, enemies and great powers (default).
    #[default]
    Interest,
    /// All of Europe.
    All,
    /// Only the player's own realm.
    Own,
}

impl NewsScope {
    /// Reads the setting value (`interest`, `all`, `own`); anything else is the default.
    pub fn from_setting(value: &str) -> Self {
        match value {
            "all" => Self::All,
            "own" => Self::Own,
            _ => Self::Interest,
        }
    }
}

/// Display of the season report (`interface/season_report`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReportMode {
    Always,
    /// Window only for a loss, a capture or one of the player's battles; else one toast line.
    #[default]
    Auto,
    Off,
}

impl ReportMode {
    pub fn from_setting(value: &str) -> Self {
        match value {
            "always" => Self::Always,
            "off" => Self::Off,
            _ => Self::Auto,
        }
    }
}

/// What the end of a season shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReportPlan {
    Modal,
    Toast(String),
    Nothing,
}

/// Events by rubric (same order as [`SECTIONS`]).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Report {
    pub groups: [Vec<Entry>; 5],
}

impl Report {
    /// `SeasonReport.needs_modal`: a loss, a capture or a battle that is not world news.
    pub fn needs_modal(&self) -> bool {
        self.groups[..4]
            .iter()
            .flatten()
            .any(|e| e.loss || e.gain || e.battle)
    }

    /// The setting decides: `always` opens the window, `off` nothing, `auto` a toast when calm.
    pub fn plan(&self, mode: ReportMode, date_label: &str) -> ReportPlan {
        match mode {
            ReportMode::Always => ReportPlan::Modal,
            ReportMode::Off => ReportPlan::Nothing,
            ReportMode::Auto if self.needs_modal() => ReportPlan::Modal,
            ReportMode::Auto => {
                let count = self.total();
                ReportPlan::Toast(if count == 0 {
                    format!("{date_label} : rien à signaler.")
                } else {
                    format!(
                        "{date_label} : {count} nouvelle{} dans le journal (rien d'urgent).",
                        fr::plural_s(count as i64)
                    )
                })
            }
        }
    }

    pub fn total(&self) -> usize {
        self.groups.iter().map(Vec::len).sum()
    }
}

/// Sorts `events` for `player`; `interest` = factions at war with or allied to the player.
pub fn build(events: &[GameEvent], player: &str, interest: &[String]) -> Report {
    build_scoped(events, player, interest, NewsScope::Interest)
}

/// [`build`] with the reach of the world news chosen by the player.
pub fn build_scoped(
    events: &[GameEvent],
    player: &str,
    interest: &[String],
    scope: NewsScope,
) -> Report {
    let mut report = Report::default();
    for event in events {
        let kind = kind_key(event);
        let mine = event.faction.as_ref().is_some_and(|f| f.as_str() == player);
        let entry = |gain: bool| Entry {
            text: event.text_fr.clone(),
            loss: event.loss && mine,
            gain,
            battle: kind == "battle",
            kind: kind.clone(),
        };
        let group = if mine {
            if LANDS.contains(&kind.as_str()) {
                Some(0)
            } else if TREASURY.contains(&kind.as_str()) {
                Some(1)
            } else if ARMIES.contains(&kind.as_str()) {
                Some(2)
            } else if WORKS.contains(&kind.as_str()) {
                Some(3)
            } else {
                None
            }
        } else {
            let of_interest = event
                .faction
                .as_ref()
                .is_some_and(|f| interest.iter().any(|i| i == f.as_str()));
            let news = WORLD_NEWS_KINDS.contains(&kind.as_str());
            let world = scope != NewsScope::Own
                && (WORLD_KINDS.contains(&kind.as_str())
                    || (event.public && news)
                    || (of_interest && news)
                    || (scope == NewsScope::All && news));
            world.then_some(4)
        };
        if let Some(index) = group {
            let gain = mine && kind == "province_captured" && !event.loss;
            report.groups[index].push(entry(gain));
        }
    }
    // Losses first, then gains, then the rest (stable).
    for group in &mut report.groups {
        group.sort_by_key(|e| {
            if e.loss {
                0
            } else if e.gain {
                1
            } else {
                2
            }
        });
    }
    report
}

/// Width and height budget of the report vignette (Godot: 536x170 inside a wider window).
pub const VIGNETTE_MAX: (f32, f32) = (400.0, 170.0);

impl Report {
    /// Event kinds shown in the report (`season_report._update_vignette`).
    pub fn kinds(&self) -> impl Iterator<Item = &str> {
        self.groups.iter().flatten().map(|e| e.kind.as_str())
    }

    /// The vignette of the highest-priority kind present (`ArtPlates.vignette_for_events`).
    pub fn vignette<'a>(&self, plates: &'a ArtPlates) -> Option<&'a Vignette> {
        plates.vignette_for_events(self.kinds())
    }
}

/// Pixel size of a JPEG file from its start-of-frame marker.
pub fn jpeg_size(bytes: &[u8]) -> Option<(u32, u32)> {
    if bytes.get(..2)? != [0xFF, 0xD8] {
        return None;
    }
    let mut at = 2;
    while at + 9 < bytes.len() {
        if bytes[at] != 0xFF {
            at += 1;
            continue;
        }
        let marker = bytes[at + 1];
        if marker == 0xFF || marker == 0x00 || (0xD0..=0xD9).contains(&marker) {
            at += if marker == 0xFF { 1 } else { 2 };
            continue;
        }
        let length = usize::from(u16::from_be_bytes([bytes[at + 2], bytes[at + 3]]));
        if (0xC0..=0xCF).contains(&marker) && ![0xC4, 0xC8, 0xCC].contains(&marker) {
            let height = u32::from(u16::from_be_bytes([bytes[at + 5], bytes[at + 6]]));
            let width = u32::from(u16::from_be_bytes([bytes[at + 7], bytes[at + 8]]));
            return Some((width, height));
        }
        at += 2 + length;
    }
    None
}

/// `[img]` size keeping the picture's aspect inside [`VIGNETTE_MAX`].
pub fn fit_vignette(size: Option<(u32, u32)>) -> (u32, u32) {
    let (max_w, max_h) = VIGNETTE_MAX;
    let ratio = size.map_or(2.4, |(w, h)| w as f32 / h.max(1) as f32);
    let (mut width, mut height) = (max_w, max_w / ratio);
    if height > max_h {
        height = max_h;
        width = max_h * ratio;
    }
    (width.round() as u32, height.round() as u32)
}

/// Vignette block of the report: the plate and its title (BBCode), "" when none applies.
/// `assets_root` is `game/assets`, used to read the picture's real proportions.
pub fn vignette_block(vignette: &Vignette, assets_root: Option<&std::path::Path>) -> String {
    if vignette.image.is_empty() {
        return String::new();
    }
    let size = assets_root
        .and_then(|root| std::fs::read(root.join(&vignette.image)).ok())
        .and_then(|bytes| jpeg_size(&bytes));
    let (width, height) = fit_vignette(size);
    format!(
        "[center][img={width}x{height}]{}[/img][/center]\n[center][i]{}[/i][/center]\n",
        vignette.image, vignette.title
    )
}

/// BBCode body of the report modal.
pub fn bbcode(report: &Report) -> String {
    if report.total() == 0 {
        return "Une saison sans événement notable.".to_owned();
    }
    let mut out = Vec::new();
    for ((_, title), group) in SECTIONS.iter().zip(&report.groups) {
        if group.is_empty() {
            continue;
        }
        out.push(format!("[b]{title}[/b]"));
        for entry in group.iter().take(MAX_PER_GROUP) {
            let color = if entry.loss {
                Some(fr::LOSS_COLOR)
            } else if entry.gain {
                Some(fr::GAIN_COLOR)
            } else {
                None
            };
            out.push(match color {
                Some(c) => format!("[color={}]• {}[/color]", hex(c), entry.text),
                None => format!("• {}", entry.text),
            });
        }
        if group.len() > MAX_PER_GROUP {
            out.push(format!(
                "[i]… et {} autre{}[/i]",
                group.len() - MAX_PER_GROUP,
                fr::plural_s((group.len() - MAX_PER_GROUP) as i64)
            ));
        }
    }
    out.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use data_model::FactionId;
    use sim_campaign::EventKind;

    fn ev(kind: EventKind, faction: &str, loss: bool) -> GameEvent {
        let mut e = GameEvent::new(kind, "texte");
        e.faction = Some(FactionId::new(faction).unwrap());
        e.loss = loss;
        e
    }

    #[test]
    fn sorts_into_rubrics_and_orders_losses_first() {
        let events = vec![
            ev(EventKind::ProvinceCaptured, "fac_france", false),
            ev(EventKind::Raid, "fac_france", true),
            ev(EventKind::Bankruptcy, "fac_france", true),
            ev(EventKind::Battle, "fac_england", false),
            ev(EventKind::WarDeclared, "fac_england", false),
        ];
        let report = build(&events, "fac_france", &[]);
        assert_eq!(report.groups[0].len(), 2);
        assert!(report.groups[0][0].loss);
        assert!(report.groups[0][1].gain);
        assert_eq!(report.groups[1].len(), 1);
        assert_eq!(
            report.groups[4].len(),
            1,
            "war news only; foreign battle dropped"
        );
        assert!(bbcode(&report).contains("Vos terres"));
    }

    #[test]
    fn news_filter_scopes_the_world_rubric() {
        // A foreign revolt: not a public event, from a faction that is not of interest.
        let events = vec![
            ev(EventKind::Revolt, "fac_england", false),
            ev(EventKind::WarDeclared, "fac_england", false),
        ];
        let world = |scope| build_scoped(&events, "fac_france", &[], scope).groups[4].len();
        assert_eq!(world(NewsScope::Interest), 1);
        assert_eq!(world(NewsScope::All), 2);
        assert_eq!(world(NewsScope::Own), 0);
        assert_eq!(NewsScope::from_setting("own"), NewsScope::Own);
        assert_eq!(NewsScope::from_setting("?"), NewsScope::Interest);
    }

    #[test]
    fn season_report_mode_decides_modal_toast_or_nothing() {
        let calm = build(&[], "fac_france", &[]);
        let loss = build(
            &[ev(EventKind::Raid, "fac_france", true)],
            "fac_france",
            &[],
        );
        let foreign_battle = build(
            &[ev(EventKind::Battle, "fac_england", false)],
            "fac_france",
            &["fac_england".to_owned()],
        );
        assert!(loss.needs_modal());
        assert!(!foreign_battle.needs_modal(), "world battles stay quiet");
        assert_eq!(calm.plan(ReportMode::Always, "x"), ReportPlan::Modal);
        assert_eq!(calm.plan(ReportMode::Off, "x"), ReportPlan::Nothing);
        assert_eq!(loss.plan(ReportMode::Auto, "x"), ReportPlan::Modal);
        assert_eq!(
            calm.plan(ReportMode::Auto, "Été 1337"),
            ReportPlan::Toast("Été 1337 : rien à signaler.".to_owned())
        );
        assert_eq!(
            foreign_battle.plan(ReportMode::Auto, "Été 1337"),
            ReportPlan::Toast("Été 1337 : 1 nouvelle dans le journal (rien d'urgent).".to_owned())
        );
        assert_eq!(ReportMode::from_setting("off"), ReportMode::Off);
    }

    #[test]
    fn jpeg_size_reads_the_frame_header() {
        // SOI, APP0 (length 4), SOF0: precision 8, height 400, width 960.
        let bytes = [
            0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x04, 0x00, 0x00, 0xFF, 0xC0, 0x00, 0x0B, 0x08, 0x01,
            0x90, 0x03, 0xC0, 0x01, 0x01, 0x11, 0x00,
        ];
        assert_eq!(jpeg_size(&bytes), Some((960, 400)));
        assert_eq!(jpeg_size(b"PNG"), None);
    }

    #[test]
    fn vignette_fits_its_budget_and_keeps_the_aspect() {
        assert_eq!(fit_vignette(Some((960, 400))), (400, 167));
        assert_eq!(fit_vignette(Some((960, 640))), (255, 170));
        assert_eq!(fit_vignette(None), (400, 167));
    }

    #[test]
    fn report_picks_the_vignette_of_its_most_urgent_kind() {
        let plates = ArtPlates::parse(&serde_json::json!({"vignettes": [
            {"id": "vg_war", "title": "La guerre", "kinds": ["battle"], "image": "res://assets/art/vignettes/w.jpg"},
            {"id": "vg_raid", "kinds": ["raid"], "image": "res://assets/art/vignettes/r.jpg"}]}));
        let events = vec![
            ev(EventKind::Raid, "fac_france", true),
            ev(EventKind::Battle, "fac_france", false),
        ];
        let report = build(&events, "fac_france", &[]);
        let chosen = report.vignette(&plates).unwrap();
        assert_eq!(chosen.id, "vg_war");
        let block = vignette_block(chosen, None);
        assert!(
            block.contains("[img=400x167]art/vignettes/w.jpg[/img]") && block.contains("La guerre")
        );
        assert!(build(&[], "fac_france", &[]).vignette(&plates).is_none());
    }

    #[test]
    fn empty_season() {
        assert_eq!(
            bbcode(&Report::default()),
            "Une saison sans événement notable."
        );
    }
}
