//! Layout and ink links of the family tree (`family_tree_view.gd::_layout` and `draw_links`).
//!
//! Current generation in the middle, ancestors above, descendants below, a spouse next to each
//! blood relative (double line). Parents to children: a drop from the couple (or the lone
//! parent), a bar, then one stroke per child. Pure functions over `game_api::court::FamilyTree`,
//! no UI: the view spawns one card per position and one thin node per segment.

use std::collections::{BTreeMap, BTreeSet};

use game_api::court::{FamilyNode, FamilyTree};

pub const NODE_W: f32 = 124.0;
pub const NODE_H: f32 = 134.0;
/// Y of the centre of the portrait inside a card (where the couple line runs).
pub const PORTRAIT_Y: f32 = 44.0;
pub const PORTRAIT_HALF: f32 = 32.0;
const SIBLING_GAP: f32 = 18.0;
const SPOUSE_GAP: f32 = 30.0;
const ROW_STEP: f32 = 176.0;
const MARGIN_X: f32 = 24.0;
const MARGIN_Y: f32 = 36.0;
pub const LINE: f32 = 2.0;

/// An axis-aligned ink stroke.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Segment {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// Result of the layout.
#[derive(Debug, Default, Clone)]
pub struct Layout {
    /// Top-left corner of the card of each person.
    pub positions: BTreeMap<String, (f32, f32)>,
    pub segments: Vec<Segment>,
    /// Pairs drawn side by side.
    pub couples: Vec<(String, String)>,
    pub width: f32,
    pub height: f32,
}

struct Tree<'a> {
    nodes: BTreeMap<&'a str, &'a FamilyNode>,
}

impl<'a> Tree<'a> {
    fn older_first(&self, a: &str, b: &str) -> std::cmp::Ordering {
        let year = |id: &str| self.nodes[id].birth_year;
        year(a).cmp(&year(b)).then_with(|| a.cmp(b))
    }

    /// Displayed father, else displayed mother.
    fn primary_parent(&self, id: &str) -> Option<&'a str> {
        let node = self.nodes[id];
        [&node.father, &node.mother]
            .into_iter()
            .find(|p| !p.is_empty() && self.nodes.contains_key(p.as_str()))
            .map(String::as_str)
    }

    /// Spouse drawn next to `id` (without displayed parents of his own).
    fn attached_spouse(&self, id: &str) -> Option<&'a str> {
        let spouse = self.nodes[id].spouse.as_str();
        if spouse.is_empty() || !self.nodes.contains_key(spouse) {
            return None;
        }
        if self.primary_parent(spouse).is_some() {
            return None;
        }
        Some(spouse)
    }
}

/// Lays `tree` out (zoom 1).
pub fn layout(tree: &FamilyTree) -> Layout {
    let t = Tree {
        nodes: tree.nodes.iter().map(|n| (n.id.as_str(), n)).collect(),
    };
    let mut out = Layout::default();
    let mut ids: Vec<&str> = t.nodes.keys().copied().collect();
    ids.sort_by(|a, b| t.older_first(a, b));
    // Units: a head node (+ attached spouse).
    let mut attached: BTreeMap<&str, &str> = BTreeMap::new(); // spouse -> head
    for &id in &ids {
        if attached.contains_key(id) {
            continue;
        }
        let Some(spouse) = t.attached_spouse(id) else {
            continue;
        };
        if attached.contains_key(spouse) {
            continue;
        }
        if t.primary_parent(id).is_none()
            && t.nodes[id].sex == "female"
            && t.nodes[spouse].sex == "male"
        {
            continue; // the husband will be the head
        }
        attached.insert(spouse, id);
    }
    let mut children_of: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    let mut heads = Vec::new();
    for &id in &ids {
        if attached.contains_key(id) {
            continue;
        }
        heads.push(id);
        children_of.insert(id, Vec::new());
    }
    let mut roots = Vec::new();
    for &id in &heads {
        let parent = t
            .primary_parent(id)
            .map(|p| attached.get(p).copied().unwrap_or(p));
        match parent.filter(|p| children_of.contains_key(p)) {
            Some(parent) => children_of.get_mut(parent).expect("head").push(id),
            None => roots.push(id),
        }
    }
    let mut spouse_of: BTreeMap<&str, &str> = BTreeMap::new();
    for (spouse, head) in &attached {
        spouse_of.insert(head, spouse);
        out.couples.push(((*head).to_owned(), (*spouse).to_owned()));
    }
    let unit_width = |id: &str| {
        if spouse_of.contains_key(id) {
            NODE_W * 2.0 + SPOUSE_GAP
        } else {
            NODE_W
        }
    };
    // Subtree widths (children sorted oldest first).
    for kids in children_of.values_mut() {
        kids.sort_by(|a, b| t.older_first(a, b));
    }
    fn width_of<'a>(
        id: &'a str,
        children_of: &BTreeMap<&'a str, Vec<&'a str>>,
        unit_width: &dyn Fn(&str) -> f32,
        widths: &mut BTreeMap<&'a str, f32>,
    ) -> f32 {
        let kids = &children_of[id];
        let mut total: f32 = kids
            .iter()
            .map(|k| width_of(k, children_of, unit_width, widths))
            .sum();
        if !kids.is_empty() {
            total += SIBLING_GAP * (kids.len() - 1) as f32;
        }
        let width = unit_width(id).max(total);
        widths.insert(id, width);
        width
    }
    let mut widths: BTreeMap<&str, f32> = BTreeMap::new();
    for &id in &roots {
        width_of(id, &children_of, &unit_width, &mut widths);
    }
    let min_generation = t.nodes.values().map(|n| n.generation).min().unwrap_or(0);
    roots.sort_by(|a, b| {
        t.nodes[a]
            .generation
            .cmp(&t.nodes[b].generation)
            .then_with(|| t.older_first(a, b))
    });
    #[allow(clippy::too_many_arguments)]
    fn place<'a>(
        id: &'a str,
        left: f32,
        t: &Tree<'a>,
        children_of: &BTreeMap<&'a str, Vec<&'a str>>,
        spouse_of: &BTreeMap<&'a str, &'a str>,
        widths: &BTreeMap<&'a str, f32>,
        unit_width: &dyn Fn(&str) -> f32,
        min_generation: i32,
        positions: &mut BTreeMap<String, (f32, f32)>,
    ) {
        let width = widths[id];
        let unit = unit_width(id);
        let y = MARGIN_Y + (t.nodes[id].generation - min_generation) as f32 * ROW_STEP;
        let unit_left = left + (width - unit) * 0.5;
        positions.insert(id.to_owned(), (unit_left, y));
        if let Some(spouse) = spouse_of.get(id) {
            positions.insert((*spouse).to_owned(), (unit_left + NODE_W + SPOUSE_GAP, y));
        }
        let kids = &children_of[id];
        let mut total: f32 = kids.iter().map(|k| widths[k]).sum();
        total += SIBLING_GAP * kids.len().saturating_sub(1) as f32;
        let mut x = left + (width - total) * 0.5;
        for kid in kids {
            place(
                kid,
                x,
                t,
                children_of,
                spouse_of,
                widths,
                unit_width,
                min_generation,
                positions,
            );
            x += widths[kid] + SIBLING_GAP;
        }
    }
    let mut x = MARGIN_X;
    for &id in &roots {
        place(
            id,
            x,
            &t,
            &children_of,
            &spouse_of,
            &widths,
            &unit_width,
            min_generation,
            &mut out.positions,
        );
        x += widths[id] + SIBLING_GAP * 2.0;
    }
    // Married branches that are not side by side.
    for &id in &ids {
        let spouse = t.nodes[id].spouse.as_str();
        if !spouse.is_empty()
            && t.nodes.contains_key(spouse)
            && !attached.contains_key(id)
            && !attached.contains_key(spouse)
            && id < spouse
        {
            out.couples.push((id.to_owned(), spouse.to_owned()));
        }
    }
    for &(x, y) in out.positions.values() {
        out.width = out.width.max(x + NODE_W);
        out.height = out.height.max(y + NODE_H);
    }
    out.width += MARGIN_X;
    out.height += MARGIN_Y;
    out.segments = links(&t, &out, &tree.root);
    out
}

fn hline(x1: f32, x2: f32, y: f32) -> Segment {
    Segment {
        x: x1.min(x2),
        y: y - LINE * 0.5,
        w: (x2 - x1).abs().max(LINE),
        h: LINE,
    }
}

fn vline(x: f32, y1: f32, y2: f32) -> Segment {
    Segment {
        x: x - LINE * 0.5,
        y: y1.min(y2),
        w: LINE,
        h: (y2 - y1).abs().max(LINE),
    }
}

/// Ink strokes: double line between spouses, parents to children, arc between siblings whose
/// parents are not shown.
fn links(t: &Tree, layout: &Layout, root: &str) -> Vec<Segment> {
    let mut out = Vec::new();
    let pos = |id: &str| layout.positions[id];
    // Spouses: double horizontal line at the height of the portraits.
    for (a, b) in &layout.couples {
        let (pa, pb) = (pos(a), pos(b));
        let (left, right) = if pa.0 < pb.0 { (pa, pb) } else { (pb, pa) };
        let from = left.0 + NODE_W * 0.5 + PORTRAIT_HALF + 4.0;
        let to = right.0 + NODE_W * 0.5 - PORTRAIT_HALF - 4.0;
        if to > from {
            let y = left.1 + PORTRAIT_Y;
            out.push(hline(from, to, y - 2.5));
            out.push(hline(from, to, y + 2.5));
        }
    }
    // Siblings without displayed parents: a lifted arc above the cards.
    if let Some(&(root_x, root_y)) = layout.positions.get(root)
        && t.primary_parent(root).is_none()
    {
        let root_generation = t.nodes[root].generation;
        let lift = root_y - 16.0;
        for (&id, node) in &t.nodes {
            if id == root
                || !node.blood
                || node.generation != root_generation
                || t.primary_parent(id).is_some()
            {
                continue;
            }
            let (x, _) = layout.positions[id];
            let (a, b) = (root_x + NODE_W * 0.5, x + NODE_W * 0.5);
            out.push(vline(a, lift, root_y + 2.0));
            out.push(hline(a, b, lift));
            out.push(vline(b, lift, root_y + 2.0));
        }
    }
    // Parents -> children, grouped by parent set.
    let mut groups: BTreeMap<Vec<&str>, Vec<&str>> = BTreeMap::new();
    for (&id, node) in &t.nodes {
        if !layout.positions.contains_key(id) {
            continue;
        }
        let parents: Vec<&str> = [&node.father, &node.mother]
            .into_iter()
            .filter(|p| !p.is_empty() && layout.positions.contains_key(p.as_str()))
            .map(String::as_str)
            .collect();
        if !parents.is_empty() {
            groups.entry(parents).or_default().push(id);
        }
    }
    for (parents, kids) in groups {
        let mut anchor_x = 0.0;
        let mut bottom: f32 = 0.0;
        for p in &parents {
            let (x, y) = pos(p);
            anchor_x += x + NODE_W * 0.5;
            bottom = bottom.max(y + NODE_H);
        }
        anchor_x /= parents.len() as f32;
        let anchor_y = if parents.len() == 2 {
            pos(parents[0]).1 + PORTRAIT_Y
        } else {
            bottom - 4.0
        };
        let child_top = kids.iter().map(|k| pos(k).1).fold(f32::INFINITY, f32::min);
        let bar_y = child_top - 18.0;
        let mut xs = vec![anchor_x];
        xs.extend(kids.iter().map(|k| pos(k).0 + NODE_W * 0.5));
        let min = xs.iter().copied().fold(f32::INFINITY, f32::min);
        let max = xs.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        out.push(vline(anchor_x, anchor_y, bar_y));
        out.push(hline(min, max, bar_y));
        for kid in kids {
            let (x, y) = pos(kid);
            out.push(vline(x + NODE_W * 0.5, bar_y, y + 4.0));
        }
    }
    out
}

/// People shown, by generation (sorted), for the dump.
pub fn generations(tree: &FamilyTree) -> BTreeSet<i32> {
    tree.nodes.iter().map(|n| n.generation).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use game_api::court::PortraitContext;

    fn node(
        id: &str,
        generation: i32,
        father: &str,
        mother: &str,
        spouse: &str,
        sex: &str,
    ) -> FamilyNode {
        FamilyNode {
            id: id.to_owned(),
            name: id.to_owned(),
            epithet: String::new(),
            sex: sex.to_owned(),
            alive: true,
            birth_year: 1300 + generation * 25,
            death_year: None,
            age: 30,
            house: String::new(),
            faction: "f".to_owned(),
            title: String::new(),
            generation,
            blood: spouse.is_empty() || !father.is_empty() || !mother.is_empty(),
            father: father.to_owned(),
            mother: mother.to_owned(),
            spouse: spouse.to_owned(),
            children: Vec::new(),
            portrait: PortraitContext::default(),
        }
    }

    fn family() -> FamilyTree {
        FamilyTree {
            root: "kid1".to_owned(),
            ruler: "dad".to_owned(),
            heir: "kid1".to_owned(),
            nodes: vec![
                node("dad", -1, "", "", "mom", "male"),
                node("mom", -1, "", "", "dad", "female"),
                node("kid1", 0, "dad", "mom", "", "male"),
                node("kid2", 0, "dad", "mom", "", "female"),
            ],
        }
    }

    #[test]
    fn couple_side_by_side_children_below_centred() {
        let layout = layout(&family());
        let dad = layout.positions["dad"];
        let mom = layout.positions["mom"];
        assert_eq!(dad.1, mom.1);
        assert!((mom.0 - dad.0 - (NODE_W + SPOUSE_GAP)).abs() < 0.01);
        let kid1 = layout.positions["kid1"];
        let kid2 = layout.positions["kid2"];
        assert!(kid1.1 > dad.1);
        assert!(kid2.0 > kid1.0);
        let couple_mid = (dad.0 + mom.0 + NODE_W) * 0.5;
        let kids_mid = (kid1.0 + kid2.0 + NODE_W) * 0.5;
        assert!(
            (couple_mid - kids_mid).abs() < 0.01,
            "{couple_mid} {kids_mid}"
        );
        assert_eq!(layout.couples, [("dad".to_owned(), "mom".to_owned())]);
    }

    #[test]
    fn links_join_every_child_to_the_parents() {
        let layout = layout(&family());
        // 2 spouse lines + (drop, bar, 2 strokes).
        assert_eq!(layout.segments.len(), 2 + 4, "{:?}", layout.segments);
        assert!(layout.width > 2.0 * NODE_W && layout.height > NODE_H);
    }
}
