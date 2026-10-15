//! Pure layout and wording of the technology trees (no Bevy): columns by tier, barycentric row
//! order, elbow links between prerequisites, French labels. Port of the maths of
//! `game/scripts/ui/tech_tree_view.gd` and `tech_panel.gd`.

use std::collections::{HashMap, HashSet};

use game_api::tech::{EffectView, TechNode};

pub const NODE_W: f32 = 212.0;
pub const NODE_H: f32 = 92.0;
pub const COLUMN_STEP: f32 = 252.0;
pub const ROW_STEP: f32 = 106.0;
pub const MARGIN_X: f32 = 12.0;
pub const MARGIN_Y: f32 = 34.0;
pub const DIM_ALPHA: f32 = 0.38;

/// A node placed in its branch canvas (top-left corner, in pixels).
#[derive(Debug, Clone, PartialEq)]
pub struct Placed {
    pub id: String,
    pub column: usize,
    pub row: usize,
    pub x: f32,
    pub y: f32,
}

/// Column header ("Rang n") above a column.
#[derive(Debug, Clone, PartialEq)]
pub struct Header {
    pub tier: u32,
    pub x: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Canvas {
    pub placed: Vec<Placed>,
    pub headers: Vec<Header>,
    pub width: f32,
    pub height: f32,
}

/// One thin rectangle of a prerequisite link.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Segment {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// Link from `from` (the prerequisite) to `to`, as horizontal / vertical rectangles.
#[derive(Debug, Clone, PartialEq)]
pub struct Link {
    pub from: String,
    pub to: String,
    pub segments: Vec<Segment>,
}

/// Columns by tier, rows sorted by the mean row of the prerequisites (fewer crossings).
pub fn layout(nodes: &[&TechNode]) -> Canvas {
    let mut tiers: Vec<u32> = nodes.iter().map(|n| n.tier).collect();
    tiers.sort_unstable();
    tiers.dedup();
    let mut rows_of: HashMap<&str, usize> = HashMap::new();
    let mut placed = Vec::new();
    let mut headers = Vec::new();
    let mut max_rows = 0;
    for (column, tier) in tiers.iter().enumerate() {
        let mut entries: Vec<&&TechNode> = nodes.iter().filter(|n| n.tier == *tier).collect();
        let key = |n: &TechNode| -> f32 {
            let rows: Vec<f32> = n
                .prerequisites
                .iter()
                .filter_map(|p| rows_of.get(p.id.as_str()).map(|r| *r as f32))
                .collect();
            if rows.is_empty() {
                100.0
            } else {
                rows.iter().sum::<f32>() / rows.len() as f32
            }
        };
        entries.sort_by(|a, b| {
            key(a)
                .partial_cmp(&key(b))
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.id.cmp(&b.id))
        });
        let x = MARGIN_X + column as f32 * COLUMN_STEP;
        headers.push(Header { tier: *tier, x });
        let ordered: Vec<(usize, &TechNode)> = entries
            .iter()
            .enumerate()
            .map(|(row, n)| (row, **n))
            .collect();
        for (row, n) in &ordered {
            placed.push(Placed {
                id: n.id.clone(),
                column,
                row: *row,
                x,
                y: MARGIN_Y + *row as f32 * ROW_STEP,
            });
        }
        for (row, n) in ordered {
            rows_of.insert(n.id.as_str(), row);
        }
        max_rows = max_rows.max(entries.len());
    }
    Canvas {
        placed,
        headers,
        width: MARGIN_X * 2.0 + tiers.len().saturating_sub(1) as f32 * COLUMN_STEP + NODE_W,
        height: MARGIN_Y + max_rows as f32 * ROW_STEP,
    }
}

fn hline(x1: f32, x2: f32, y: f32) -> Segment {
    Segment {
        x: x1.min(x2),
        y: y - 1.0,
        w: (x2 - x1).abs(),
        h: 2.0,
    }
}

fn vline(x: f32, y1: f32, y2: f32) -> Segment {
    Segment {
        x: x - 1.0,
        y: y1.min(y2),
        w: 2.0,
        h: (y2 - y1).abs(),
    }
}

/// Elbow links between the placed nodes of one branch (prerequisites outside it are skipped).
pub fn links(nodes: &[&TechNode], canvas: &Canvas) -> Vec<Link> {
    let at: HashMap<&str, &Placed> = canvas.placed.iter().map(|p| (p.id.as_str(), p)).collect();
    let gap = (COLUMN_STEP - NODE_W) * 0.5;
    let mut out = Vec::new();
    for node in nodes {
        let Some(target) = at.get(node.id.as_str()) else {
            continue;
        };
        let to = (target.x, target.y + NODE_H * 0.5);
        for prereq in &node.prerequisites {
            let Some(source) = at.get(prereq.id.as_str()) else {
                continue;
            };
            let from = (source.x + NODE_W, source.y + NODE_H * 0.5);
            let mut segments = Vec::new();
            if to.0 - from.0 <= COLUMN_STEP {
                segments.push(hline(from.0, from.0 + gap, from.1));
                segments.push(vline(from.0 + gap, from.1, to.1));
                segments.push(hline(from.0 + gap, to.0, to.1));
            } else {
                // Rank jump: through the gutter above the target, not across the nodes between.
                let lane_y = target.y - (ROW_STEP - NODE_H) * 0.5;
                segments.push(hline(from.0, from.0 + gap, from.1));
                segments.push(vline(from.0 + gap, from.1, lane_y));
                segments.push(hline(from.0 + gap, to.0 - gap, lane_y));
                segments.push(vline(to.0 - gap, lane_y, to.1));
                segments.push(hline(to.0 - gap, to.0, to.1));
            }
            out.push(Link {
                from: prereq.id.clone(),
                to: node.id.clone(),
                segments,
            });
        }
    }
    out
}

/// Unmet ancestors (transitively) and all descendants of `id`, plus `id`: the highlighted path.
pub fn path_of(nodes: &[&TechNode], id: &str) -> HashSet<String> {
    let by_id: HashMap<&str, &TechNode> = nodes.iter().map(|n| (n.id.as_str(), *n)).collect();
    let mut children: HashMap<&str, Vec<&str>> = HashMap::new();
    for n in nodes {
        for p in &n.prerequisites {
            children
                .entry(p.id.as_str())
                .or_default()
                .push(n.id.as_str());
        }
    }
    let mut path = HashSet::new();
    if !by_id.contains_key(id) {
        return path;
    }
    path.insert(id.to_owned());
    let mut stack = vec![id];
    while let Some(current) = stack.pop() {
        for p in &by_id[current].prerequisites {
            if let Some(parent) = by_id.get(p.id.as_str())
                && parent.state != "known"
                && path.insert(p.id.clone())
            {
                stack.push(p.id.as_str());
            }
        }
    }
    let mut stack = vec![id];
    let mut seen = HashSet::new();
    while let Some(current) = stack.pop() {
        for child in children.get(current).into_iter().flatten() {
            if seen.insert(*child) {
                path.insert((*child).to_owned());
                stack.push(child);
            }
        }
    }
    path
}

/// UX5-T3 filters (the search box is not ported: Bevy UI has no text field yet).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Filter {
    pub reach: bool,
    pub unlock: bool,
    pub fade_known: bool,
}

impl Filter {
    pub fn active(&self) -> bool {
        self.reach || self.unlock || self.fade_known
    }

    pub fn matches(&self, node: &TechNode) -> bool {
        if self.fade_known && node.state == "known" {
            return false;
        }
        if self.reach && node.state != "available" {
            return false;
        }
        !(self.unlock && node.unit_unlocks.is_empty() && node.building_unlocks.is_empty())
    }
}

pub fn state_label(state: &str) -> &'static str {
    match state {
        "known" => "Acquise",
        "researching" => "En cours",
        "available" => "Disponible",
        "queued" => "En file",
        _ => "Verrouillée",
    }
}

/// State shown on the node: an available technology waiting in the queue reads "queued".
pub fn shown_state(node: &TechNode) -> &'static str {
    if node.queue_position > 0 && node.state == "available" {
        "queued"
    } else {
        match node.state.as_str() {
            "known" => "known",
            "researching" => "researching",
            "available" => "available",
            _ => "locked",
        }
    }
}

/// State glyph (never colour alone, UX5-T7); `""` when none.
pub fn glyph(shown: &str, queue_position: usize) -> String {
    match shown {
        "known" => "✓".into(),
        "researching" => "*".into(),
        "locked" => "×".into(),
        "queued" => format!("n°{queue_position}"),
        _ => String::new(),
    }
}

/// Second line of a node: cost, progress and state.
pub fn second_line(node: &TechNode) -> String {
    let shown = shown_state(node);
    if shown == "queued" {
        return format!(
            "{} pts — En file (n° {})",
            node.effective_cost, node.queue_position
        );
    }
    if node.state == "researching" || (node.state == "available" && node.progress > 0) {
        format!(
            "{} / {} pts — {}",
            node.progress,
            node.effective_cost,
            state_label(&node.state)
        )
    } else {
        format!("{} pts — {}", node.effective_cost, state_label(shown))
    }
}

pub fn effect_name(kind: &str) -> &str {
    match kind {
        "tax_income" => "Impôts",
        "trade_income" => "Commerce",
        "health" => "Santé",
        "growth" => "Croissance",
        "unrest" => "Mécontentement",
        "wealth" => "Richesse",
        "production" => "Production",
        "army_armor" => "Armure",
        "army_ranged" => "Tir",
        "army_melee" => "Mêlée",
        "army_morale" => "Moral",
        "army_upkeep" => "Entretien des armées",
        "army_experience" => "Expérience des troupes",
        "recruit_cost" => "Coût de recrutement",
        "movement" => "Mouvement",
        "siege_resistance" => "Résistance aux sièges",
        "fortification_level" => "Fortifications",
        "research_points" => "Points de recherche",
        "prestige" => "Prestige",
        "piety" => "Piété",
        "plague_resistance" => "Résistance à la peste",
        "wound_recovery" => "Soin des blessés",
        "diet_health" => "Santé tirée des régimes",
        other => other,
    }
}

fn category_name(category: &str) -> &str {
    match category {
        "infantry" => "infanterie",
        "ranged" => "tireurs",
        "cavalry" => "cavalerie",
        "siege" => "siège",
        other => other,
    }
}

fn number(value: f64) -> String {
    if (value - value.round()).abs() < 1e-9 {
        format!("{}", value.round() as i64)
    } else {
        format!("{value}")
    }
}

/// "Impôts +10 %" (`effect_label`).
pub fn effect_label(effect: &EffectView) -> String {
    let mut label = effect_name(&effect.kind).to_owned();
    if !effect.unit_category.is_empty() {
        label.push_str(&format!(" ({})", category_name(&effect.unit_category)));
    }
    let sign = if effect.value >= 0.0 { "+" } else { "" };
    let suffix = if effect.mode == "percent" { " %" } else { "" };
    format!("{label} {sign}{}{suffix}", number(effect.value))
}

/// Short name for a foot pill.
pub fn short(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        text.to_owned()
    } else {
        let head: String = text.chars().take(max - 1).collect();
        format!("{head}…")
    }
}

pub fn turns_text(turns: Option<u32>) -> String {
    match turns {
        None => "jamais".into(),
        Some(1) | Some(0) => format!("{} tour", turns.unwrap_or(0)),
        Some(n) => format!("{n} tours"),
    }
}

/// Turns to pay `remaining` points at `points` per turn (`None`: never).
pub fn turns_for(remaining: u32, points: u32) -> Option<u32> {
    (points > 0).then(|| remaining.div_ceil(points))
}

/// One card of the queue ribbon: `(caption, gauge ratio, turns line)`.
#[derive(Debug, Clone, PartialEq)]
pub struct RibbonCard {
    pub caption: String,
    pub ratio: f32,
    pub turns: String,
}

/// Ribbon of 3 slots and the end-of-queue line (`_fill_ribbon`).
pub fn ribbon(
    research: Option<&game_api::tech::ResearchView>,
    queue: &[game_api::tech::QueueEntry],
    points_per_turn: u32,
    current_turn: u32,
) -> (Vec<RibbonCard>, Option<String>) {
    struct Entry {
        name: String,
        max: u32,
        value: u32,
        turns: Option<u32>,
    }
    let mut entries = Vec::new();
    let mut never = false;
    let mut elapsed = 0;
    let points = research.map_or(points_per_turn, |r| r.points_per_turn);
    if let Some(r) = research {
        never = r.turns_left.is_none();
        elapsed = r.turns_left.unwrap_or(0);
        entries.push(Entry {
            name: r.name.clone(),
            max: r.cost.max(1),
            value: r.progress,
            turns: r.turns_left,
        });
    }
    for q in queue {
        match turns_for(q.cost, points) {
            Some(t) if !never => elapsed += t,
            _ => {
                never = true;
                elapsed = 0;
            }
        }
        entries.push(Entry {
            name: q.name.clone(),
            max: q.cost.max(1),
            value: 0,
            turns: if never { None } else { Some(elapsed) },
        });
    }
    let numerals = ["I", "II", "III"];
    let mut cards = Vec::new();
    #[allow(clippy::needless_range_loop)]
    for index in 0..3 {
        match entries.get(index) {
            Some(e) => cards.push(RibbonCard {
                caption: format!("{}. {}", numerals[index], e.name),
                ratio: e.value as f32 / e.max as f32,
                turns: if index == 0 {
                    turns_text(e.turns)
                } else {
                    format!("prêt dans {}", turns_text(e.turns))
                },
            }),
            None => cards.push(RibbonCard {
                caption: "—".into(),
                ratio: 0.0,
                turns: "emplacement libre".into(),
            }),
        }
    }
    let end = if entries.is_empty() {
        None
    } else if never {
        Some("Fin de la file : jamais (aucun point de recherche)".to_owned())
    } else if current_turn > 0 {
        Some(format!("Fin de la file : tour {}", current_turn + elapsed))
    } else {
        Some(format!(
            "Fin de la file : dans {}",
            turns_text(Some(elapsed))
        ))
    };
    (cards, end)
}

/// Rich tooltip (BBCode): lock note, name, description, effects, unlocks, before → after, year, cost.
pub fn tooltip_bbcode(node: &TechNode) -> String {
    let mut lines: Vec<String> = Vec::new();
    let missing: Vec<&str> = node
        .prerequisites
        .iter()
        .filter(|p| !p.met)
        .map(|p| p.name.as_str())
        .collect();
    if node.state == "locked" && !missing.is_empty() {
        lines.push(format!(
            "[color=#9e2114]Verrouillée : requiert {}[/color]",
            missing.join(", ")
        ));
    }
    lines.push(format!("[b]{}[/b]", node.name));
    if !node.description.is_empty() {
        lines.push(node.description.clone());
    }
    if !node.effects.is_empty() {
        let effects: Vec<String> = node.effects.iter().map(effect_label).collect();
        lines.push(format!("Effets : {}", effects.join(", ")));
    }
    let mut unlocked: Vec<String> = node
        .unit_unlocks
        .iter()
        .map(|u| format!("unité {}", u.name))
        .collect();
    unlocked.extend(
        node.building_unlocks
            .iter()
            .map(|b| format!("bâtiment {}", b.name)),
    );
    if !unlocked.is_empty() {
        lines.push(format!("Débloque : {}", unlocked.join(", ")));
    }
    for (key, before, after) in &node.before_after {
        lines.push(format!(
            "[i]{}[/i] : {} → {}",
            effect_name(key),
            number((before * 100.0).round() / 100.0),
            number((after * 100.0).round() / 100.0)
        ));
    }
    if node.historical_year > 0 {
        lines.push(format!(
            "Année historique : {}{}",
            if node.historical_uncertain {
                "vers "
            } else {
                ""
            },
            node.historical_year
        ));
    }
    if node.effective_cost > node.cost {
        let percent = (node.effective_cost - node.cost) * 100 / node.cost.max(1);
        lines.push(format!(
            "Coût : {} points ({} + {percent} % : trop en avance sur son temps)",
            node.effective_cost, node.cost
        ));
    } else {
        lines.push(format!("Coût : {} points", node.cost));
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use game_api::tech::{PrerequisiteView, UnlockView};

    fn node(id: &str, tier: u32, prereqs: &[&str], state: &str) -> TechNode {
        TechNode {
            id: id.into(),
            name: id.into(),
            branch: "civil".into(),
            tier,
            cost: 100,
            effective_cost: 100,
            prerequisites: prereqs
                .iter()
                .map(|p| PrerequisiteView {
                    id: (*p).into(),
                    name: (*p).into(),
                    met: false,
                })
                .collect(),
            unit_unlocks: vec![],
            building_unlocks: vec![],
            effects: vec![],
            description: String::new(),
            historical_year: 0,
            historical_uncertain: false,
            historical_note: String::new(),
            state: state.into(),
            progress: 0,
            queue_position: 0,
            before_after: vec![],
        }
    }

    #[test]
    fn columns_rows_and_links() {
        let a = node("a", 1, &[], "known");
        let b = node("b", 1, &[], "known");
        let c = node("c", 2, &["b"], "available");
        let d = node("d", 3, &["a", "c"], "locked");
        let all = [&a, &b, &c, &d];
        let canvas = layout(&all);
        let pos = |id: &str| canvas.placed.iter().find(|p| p.id == id).unwrap();
        assert_eq!(pos("a").x, MARGIN_X);
        assert_eq!(pos("c").x, MARGIN_X + COLUMN_STEP);
        assert_eq!(pos("d").column, 2);
        assert_eq!(canvas.headers.len(), 3);
        assert_eq!(canvas.height, MARGIN_Y + 2.0 * ROW_STEP);
        assert_eq!(canvas.width, MARGIN_X * 2.0 + 2.0 * COLUMN_STEP + NODE_W);
        let l = links(&all, &canvas);
        assert_eq!(l.len(), 3);
        // a -> d jumps a rank: 5 segments through the lane; b -> c is a 3-segment elbow.
        assert_eq!(l.iter().find(|k| k.from == "a").unwrap().segments.len(), 5);
        assert_eq!(l.iter().find(|k| k.from == "b").unwrap().segments.len(), 3);
        // Path of d: itself, unmet ancestors (c; a and b are known except c's parent b known).
        let path = path_of(&all, "d");
        assert!(path.contains("d") && path.contains("c") && !path.contains("b"));
        let from_a = path_of(&all, "a");
        assert!(from_a.contains("d") && !from_a.contains("c"));
    }

    #[test]
    fn wording() {
        let mut n = node("x", 1, &[], "available");
        n.progress = 40;
        assert_eq!(second_line(&n), "40 / 100 pts — Disponible");
        n.queue_position = 2;
        assert_eq!(second_line(&n), "100 pts — En file (n° 2)");
        assert_eq!(glyph("queued", 2), "n°2");
        let e = EffectView {
            kind: "army_armor".into(),
            value: 10.0,
            mode: "percent".into(),
            unit_category: "cavalry".into(),
            class: String::new(),
        };
        assert_eq!(effect_label(&e), "Armure (cavalerie) +10 %");
        assert_eq!(turns_text(None), "jamais");
        assert_eq!(turns_text(Some(3)), "3 tours");
        assert_eq!(turns_for(95, 10), Some(10));
        let mut f = Filter::default();
        assert!(f.matches(&n));
        f.unlock = true;
        assert!(!f.matches(&n));
        n.unit_unlocks.push(UnlockView {
            id: "u".into(),
            name: "U".into(),
        });
        assert!(f.matches(&n));
    }

    #[test]
    fn ribbon_counts_turns() {
        let research = game_api::tech::ResearchView {
            technology: "a".into(),
            name: "A".into(),
            progress: 50,
            cost: 100,
            points_per_turn: 10,
            turns_left: Some(5),
        };
        let queue = [game_api::tech::QueueEntry {
            id: "b".into(),
            name: "B".into(),
            cost: 100,
        }];
        let (cards, end) = ribbon(Some(&research), &queue, 10, 3);
        assert_eq!(cards[0].turns, "5 tours");
        assert_eq!(cards[1].turns, "prêt dans 15 tours");
        assert_eq!(cards[2].turns, "emplacement libre");
        assert_eq!(end.unwrap(), "Fin de la file : tour 18");
    }
}
