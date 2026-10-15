//! Layout of the skill tree (`skill_tree_view.gd::_columns_of` and `_draw`): one band per branch,
//! ranks as columns from left to right, each skill linked to its prerequisites by an ink
//! stroke (gold once both ends are learned). Pure: the view spawns the nodes and the strokes.

use std::collections::BTreeMap;

use game_api::court::{SkillNode, SkillState};

use super::family_layout::Segment;

pub const NODE_W: f32 = 156.0;
pub const NODE_H: f32 = 52.0;
const COLUMN_STEP: f32 = 186.0;
const ROW_STEP: f32 = 60.0;
pub const BAND_HEADER: f32 = 28.0;
const BAND_GAP: f32 = 12.0;
const LEFT: f32 = 8.0;
const BRANCHES: [&str; 3] = ["command", "governance", "court"];

/// Colour class of a prerequisite stroke.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkKind {
    /// Both ends learned (gold).
    Both,
    /// Prerequisite learned (ink).
    Source,
    /// Faded.
    Idle,
}

#[derive(Debug, Clone, Copy)]
pub struct Link {
    pub segment: Segment,
    pub kind: LinkKind,
}

#[derive(Debug, Clone)]
pub struct Band {
    pub branch: &'static str,
    pub top: f32,
    pub height: f32,
}

#[derive(Debug, Default, Clone)]
pub struct SkillLayout {
    pub positions: BTreeMap<String, (f32, f32)>,
    pub bands: Vec<Band>,
    pub links: Vec<Link>,
    pub width: f32,
    pub height: f32,
}

/// Columns of a branch: depth = max(tier - 1, depth of the prerequisites + 1), then a
/// barycentric sort (mean row of the prerequisites) to limit the crossings.
fn columns_of<'a>(nodes: &'a [SkillNode], branch: &str) -> Vec<Vec<&'a SkillNode>> {
    let mut ids: Vec<&SkillNode> = nodes.iter().filter(|n| n.branch == branch).collect();
    ids.sort_by(|a, b| a.id.cmp(&b.id));
    let mut depth: BTreeMap<&str, usize> = ids
        .iter()
        .map(|n| (n.id.as_str(), n.tier.saturating_sub(1) as usize))
        .collect();
    let mut changed = true;
    let mut guard = 0;
    while changed && guard < 64 {
        changed = false;
        guard += 1;
        for node in &ids {
            for prerequisite in &node.prerequisites {
                if let Some(&parent) = depth.get(prerequisite.as_str())
                    && depth[node.id.as_str()] <= parent
                {
                    depth.insert(node.id.as_str(), parent + 1);
                    changed = true;
                }
            }
        }
    }
    let mut columns: Vec<Vec<&SkillNode>> = Vec::new();
    for node in &ids {
        let d = depth[node.id.as_str()];
        while columns.len() <= d {
            columns.push(Vec::new());
        }
        columns[d].push(node);
    }
    let mut row_of: BTreeMap<&str, usize> = BTreeMap::new();
    for (c, column) in columns.iter_mut().enumerate() {
        if c > 0 {
            let weight = |n: &SkillNode| {
                let rows: Vec<f32> = n
                    .prerequisites
                    .iter()
                    .filter_map(|p| row_of.get(p.as_str()).map(|r| *r as f32))
                    .collect();
                if rows.is_empty() {
                    99.0
                } else {
                    rows.iter().sum::<f32>() / rows.len() as f32
                }
            };
            column.sort_by(|a, b| {
                weight(a)
                    .partial_cmp(&weight(b))
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| a.id.cmp(&b.id))
            });
        }
        for (r, node) in column.iter().enumerate() {
            row_of.insert(node.id.as_str(), r);
        }
    }
    columns
}

/// Lays the tree out.
pub fn layout(nodes: &[SkillNode]) -> SkillLayout {
    let mut out = SkillLayout::default();
    let mut top = 0.0;
    for branch in BRANCHES {
        let columns = columns_of(nodes, branch);
        if columns.is_empty() {
            continue;
        }
        let rows = columns.iter().map(Vec::len).max().unwrap_or(0);
        for (c, column) in columns.iter().enumerate() {
            let offset = (rows - column.len()) as f32 * ROW_STEP * 0.5;
            for (r, node) in column.iter().enumerate() {
                out.positions.insert(
                    node.id.clone(),
                    (
                        LEFT + c as f32 * COLUMN_STEP,
                        top + BAND_HEADER + offset + r as f32 * ROW_STEP,
                    ),
                );
            }
        }
        let height = BAND_HEADER + rows as f32 * ROW_STEP;
        out.bands.push(Band {
            branch,
            top,
            height,
        });
        out.width = out
            .width
            .max(LEFT + (columns.len() - 1) as f32 * COLUMN_STEP + NODE_W + LEFT);
        top += height + BAND_GAP;
    }
    out.height = (top - BAND_GAP).max(0.0);
    let state_of: BTreeMap<&str, SkillState> =
        nodes.iter().map(|n| (n.id.as_str(), n.state)).collect();
    for node in nodes {
        let Some(&(tx, ty)) = out.positions.get(&node.id) else {
            continue;
        };
        for prerequisite in &node.prerequisites {
            let Some(&(sx, sy)) = out.positions.get(prerequisite) else {
                continue;
            };
            let source_learned = state_of.get(prerequisite.as_str()) == Some(&SkillState::Learned);
            let kind = if source_learned && node.state == SkillState::Learned {
                LinkKind::Both
            } else if source_learned {
                LinkKind::Source
            } else {
                LinkKind::Idle
            };
            let thick = if kind == LinkKind::Both { 2.5 } else { 1.6 };
            // Elbow: out of the right edge, vertical at mid-gap, into the left edge.
            let (x1, y1) = (sx + NODE_W, sy + NODE_H * 0.5);
            let (x2, y2) = (tx, ty + NODE_H * 0.5);
            let mid = x1 + (x2 - x1) * 0.5;
            let seg = |x: f32, y: f32, w: f32, h: f32| Link {
                segment: Segment { x, y, w, h },
                kind,
            };
            out.links
                .push(seg(x1, y1 - thick * 0.5, (mid - x1).max(1.0), thick));
            out.links.push(seg(
                mid - thick * 0.5,
                y1.min(y2),
                thick,
                (y2 - y1).abs().max(thick),
            ));
            out.links
                .push(seg(mid, y2 - thick * 0.5, (x2 - mid).max(1.0), thick));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn skill(
        id: &str,
        branch: &str,
        tier: u32,
        prerequisites: &[&str],
        state: SkillState,
    ) -> SkillNode {
        SkillNode {
            id: id.to_owned(),
            name: id.to_owned(),
            branch: branch.to_owned(),
            tier,
            cost: 1,
            description: String::new(),
            prerequisites: prerequisites.iter().map(|p| (*p).to_owned()).collect(),
            effects: Vec::new(),
            state,
        }
    }

    #[test]
    fn depth_follows_prerequisites_and_bands_stack() {
        let nodes = vec![
            skill("a", "command", 1, &[], SkillState::Learned),
            skill("b", "command", 1, &["a"], SkillState::Available),
            skill("c", "court", 1, &[], SkillState::Locked),
        ];
        let layout = layout(&nodes);
        assert!(
            layout.positions["b"].0 > layout.positions["a"].0,
            "b is a rank after a"
        );
        assert_eq!(layout.bands.len(), 2);
        assert!(layout.positions["c"].1 > layout.positions["a"].1);
        // One prerequisite link = 3 strokes, ink because only the source is learned.
        assert_eq!(layout.links.len(), 3);
        assert!(layout.links.iter().all(|l| l.kind == LinkKind::Source));
    }

    #[test]
    fn learned_pair_is_gold() {
        let nodes = vec![
            skill("a", "court", 1, &[], SkillState::Learned),
            skill("b", "court", 2, &["a"], SkillState::Learned),
        ];
        assert!(
            layout(&nodes)
                .links
                .iter()
                .all(|l| l.kind == LinkKind::Both)
        );
    }
}
