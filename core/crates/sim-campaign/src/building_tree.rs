//! RG11 (ADR 0345): the construction tree of a settlement. Every building of
//! `data/buildings/` as a node with its state (built, under construction,
//! queued, available, locked), the plain-French reason of a lock and its
//! requirements. Read-only: it reuses the validations of
//! [`CampaignState::buildable`] (`structural_blockers` for the locks, the
//! option's own reason for money, queue and slots); building is still
//! `Order::Build`.

use data_model::{BuildingId, GameData, SettlementId, SettlementKind};

use crate::buildings::Blocker;
use crate::state::CampaignState;

/// State of a node of the tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeState {
    /// Standing (or replaced by a later step of its chain).
    Built,
    /// The construction under way.
    Building,
    /// Waiting in the build queue.
    Queued,
    /// Structurally possible here (money, queue or slots may still refuse).
    Available,
    /// Cannot be built here as things are: see `reason`.
    Locked,
}

impl NodeState {
    /// Stable key for the UI.
    pub fn key(self) -> &'static str {
        match self {
            NodeState::Built => "built",
            NodeState::Building => "building",
            NodeState::Queued => "queued",
            NodeState::Available => "available",
            NodeState::Locked => "locked",
        }
    }
}

/// One prerequisite of a node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeRequirement {
    /// `rank`, `chain`, `building`, `technology`, `resource`, `coast`,
    /// `river` or `unique`.
    pub kind: &'static str,
    /// Id of the thing required (building, technology, resource), or "".
    pub id: String,
    /// French sentence, « Requiert la technologie X ».
    pub text: String,
    pub met: bool,
}

/// One building of the tree.
#[derive(Debug, Clone, PartialEq)]
pub struct TreeNode {
    pub building: BuildingId,
    pub name: String,
    pub category: &'static str,
    pub tier: u8,
    /// Previous step of the upgrade chain.
    pub parent: Option<BuildingId>,
    /// First step of the chain this node belongs to.
    pub root: BuildingId,
    /// Position in the chain (1 = root).
    pub depth: u8,
    pub state: NodeState,
    /// `true` when this very building stands (not only a later step).
    pub current: bool,
    /// `requirements` kind of the first unmet one (locked nodes), else "".
    pub lock: &'static str,
    /// Plain French reason of the lock, "" when not locked.
    pub reason: String,
    /// Why an available node cannot start now (treasury, queue, slots).
    pub blocked_now: String,
    /// `true` when `Order::Build` would be accepted now.
    pub can_build: bool,
    pub cost: u32,
    pub turns: u32,
    pub upkeep: u32,
    /// `true` when building it takes a new slot (it starts a chain).
    pub takes_slot: bool,
    /// For `Building` and `Queued` nodes.
    pub turns_left: u32,
    pub total_turns: u32,
    /// 0 for the construction under way, 1.. for the queue; -1 otherwise.
    pub queue_index: i64,
    /// Whether the settlement's rank allows this building at all.
    pub allowed_here: bool,
    pub requirements: Vec<TreeRequirement>,
}

/// A construction of the queue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeQueueRow {
    pub building: BuildingId,
    pub name: String,
    pub turns_left: u32,
    pub total_turns: u32,
}

/// The construction tree of a settlement.
#[derive(Debug, Clone, PartialEq)]
pub struct BuildingTree {
    pub settlement: SettlementId,
    pub kind: SettlementKind,
    pub kind_label: &'static str,
    pub slots_used: usize,
    pub slots_max: Option<usize>,
    /// Chantiers allowed at once (under way plus queued).
    pub queue_size: usize,
    /// Under way first, then the queue.
    pub queue: Vec<TreeQueueRow>,
    /// By category, chain root, then depth.
    pub nodes: Vec<TreeNode>,
}

/// French name of a settlement rank.
pub fn kind_label(kind: SettlementKind) -> &'static str {
    match kind {
        SettlementKind::City => "Cité",
        SettlementKind::Town => "Ville",
        SettlementKind::Castle => "Château",
        SettlementKind::Abbey => "Abbaye",
        SettlementKind::Village => "Village",
    }
}

const ALL_KINDS: [SettlementKind; 5] = [
    SettlementKind::City,
    SettlementKind::Town,
    SettlementKind::Castle,
    SettlementKind::Abbey,
    SettlementKind::Village,
];

impl CampaignState {
    /// The construction tree of `settlement`; `None` for an unknown id.
    pub fn building_tree(
        &self,
        data: &GameData,
        settlement: &SettlementId,
    ) -> Option<BuildingTree> {
        let state = self.settlement_state(settlement)?;
        let province_data = data.provinces.get(&state.province)?;
        let faction = self.factions.get(&state.controller)?;
        let options = self.buildable(data, settlement);
        let usage = self.slot_usage(data, settlement);
        let queue_size = data.economy_rules.construction_queue_size.max(1) as usize;

        let pending: Vec<&crate::state::Construction> = state
            .construction
            .iter()
            .chain(state.build_queue.iter())
            .collect();
        let queue: Vec<TreeQueueRow> = pending
            .iter()
            .map(|c| TreeQueueRow {
                building: c.building.clone(),
                name: data.building_name(&c.building),
                turns_left: c.turns_left,
                total_turns: options
                    .iter()
                    .find(|o| o.building == c.building)
                    .map_or(c.turns_left, |o| o.turns.max(c.turns_left)),
            })
            .collect();

        let mut nodes: Vec<TreeNode> = data
            .buildings
            .values()
            .map(|building| {
                let id = &building.id;
                let allowed_here = building.allowed_in(state.kind);
                let option = options.iter().find(|o| &o.building == id);
                let blockers = self.structural_blockers(
                    data,
                    settlement,
                    state,
                    province_data,
                    faction,
                    building,
                    false,
                );
                let in_pending = pending.iter().position(|c| &c.building == id);
                let built = data.has_building(&state.buildings, id);
                let node_state = if built {
                    NodeState::Built
                } else if in_pending == Some(0) && state.construction.is_some() {
                    NodeState::Building
                } else if in_pending.is_some() {
                    NodeState::Queued
                } else if !allowed_here || !blockers.is_empty() {
                    NodeState::Locked
                } else {
                    NodeState::Available
                };
                let requirements =
                    requirements_of(data, building, allowed_here, &blockers, &pending);
                let first_unmet = requirements.iter().find(|r| !r.met);
                let locked = node_state == NodeState::Locked;
                let blocked_now = if node_state == NodeState::Available {
                    option.and_then(|o| o.reason.clone()).unwrap_or_default()
                } else {
                    String::new()
                };
                let (cost, turns) = option
                    .map_or((building.cost.money, building.build_time_turns), |o| {
                        (o.cost, o.turns)
                    });
                let (turns_left, total_turns) =
                    in_pending.map_or((0, 0), |i| (queue[i].turns_left, queue[i].total_turns));
                let (root, depth) = chain_root_depth(data, id);
                TreeNode {
                    building: id.clone(),
                    name: building.name.display.clone(),
                    category: building.category.key(),
                    tier: building.tier,
                    parent: building.upgrades_from.clone(),
                    root,
                    depth,
                    state: node_state,
                    current: state.buildings.contains(id),
                    lock: if locked {
                        first_unmet.map_or("", |r| r.kind)
                    } else {
                        ""
                    },
                    reason: if locked {
                        first_unmet.map_or_else(String::new, |r| r.text.clone())
                    } else {
                        String::new()
                    },
                    can_build: node_state == NodeState::Available && blocked_now.is_empty(),
                    blocked_now,
                    cost,
                    turns,
                    upkeep: building.upkeep.unwrap_or(0),
                    takes_slot: building
                        .upgrades_from
                        .as_ref()
                        .is_none_or(|parent| !state.buildings.contains(parent)),
                    turns_left,
                    total_turns,
                    queue_index: in_pending.map_or(-1, |i| i as i64),
                    allowed_here,
                    requirements,
                }
            })
            .collect();
        nodes.sort_by(|a, b| {
            (a.category, &a.root, a.depth, &a.building).cmp(&(
                b.category,
                &b.root,
                b.depth,
                &b.building,
            ))
        });
        Some(BuildingTree {
            settlement: settlement.clone(),
            kind: state.kind,
            kind_label: kind_label(state.kind),
            slots_used: usage.used,
            slots_max: usage.max,
            queue_size,
            queue,
            nodes,
        })
    }
}

/// First step of the chain of `id` and its position in it (1 = root).
fn chain_root_depth(data: &GameData, id: &BuildingId) -> (BuildingId, u8) {
    let mut current = id.clone();
    let mut depth = 1u8;
    for _ in 0..16 {
        match data
            .buildings
            .get(&current)
            .and_then(|b| b.upgrades_from.clone())
        {
            Some(parent) => {
                current = parent;
                depth += 1;
            }
            None => break,
        }
    }
    (current, depth)
}

/// The prerequisites a building declares, with whether each is met. Rank
/// first, then chain, building, technology, resource, coast, river, unique.
fn requirements_of(
    data: &GameData,
    building: &data_model::Building,
    allowed_here: bool,
    blockers: &[Blocker],
    pending: &[&crate::state::Construction],
) -> Vec<TreeRequirement> {
    let mut out = Vec::new();
    let ranks: Vec<&'static str> = ALL_KINDS
        .iter()
        .copied()
        .filter(|k| building.allowed_in(*k))
        .map(kind_label)
        .collect();
    out.push(TreeRequirement {
        kind: "rank",
        id: String::new(),
        text: format!("Requiert le rang {}", join_or(&ranks)),
        met: allowed_here,
    });
    let in_works = |id: &BuildingId| pending.iter().any(|c| &c.building == id);
    let works = |id: &BuildingId| if in_works(id) { " (en chantier)" } else { "" };
    if let Some(parent) = &building.upgrades_from {
        out.push(TreeRequirement {
            kind: "chain",
            id: parent.to_string(),
            text: format!("Requiert {}{}", data.building_name(parent), works(parent)),
            met: !blockers.contains(&Blocker::Parent(parent.clone())),
        });
    }
    if let Some(required) = &building.required_building {
        out.push(TreeRequirement {
            kind: "building",
            id: required.to_string(),
            text: format!(
                "Requiert le bâtiment {}{}",
                data.building_name(required),
                works(required)
            ),
            met: !blockers.contains(&Blocker::Building(required.clone())),
        });
    }
    if let Some(tech) = &building.required_technology {
        out.push(TreeRequirement {
            kind: "technology",
            id: tech.to_string(),
            text: format!("Requiert la technologie {}", data.tech_name(tech)),
            met: !blockers.contains(&Blocker::Technology(tech.clone())),
        });
    }
    if let Some(resource) = &building.required_resource {
        let name = data
            .resources
            .get(resource)
            .map_or_else(|| resource.to_string(), |r| r.name.display.clone());
        out.push(TreeRequirement {
            kind: "resource",
            id: resource.to_string(),
            text: format!("Requiert la ressource {name} dans la province"),
            met: !blockers.contains(&Blocker::Resource(resource.clone())),
        });
    }
    if building.requires_coastal {
        out.push(TreeRequirement {
            kind: "coast",
            id: String::new(),
            text: "Requiert un accès à la mer (port)".to_owned(),
            met: !blockers.contains(&Blocker::Coast),
        });
    }
    if building.requires_river {
        out.push(TreeRequirement {
            kind: "river",
            id: String::new(),
            text: "Requiert une rivière dans la province".to_owned(),
            met: !blockers.contains(&Blocker::River),
        });
    }
    if building.unique_per_faction {
        out.push(TreeRequirement {
            kind: "unique",
            id: String::new(),
            text: "Un seul exemplaire par faction".to_owned(),
            met: !blockers.contains(&Blocker::Unique),
        });
    }
    out
}

fn join_or(labels: &[&str]) -> String {
    match labels {
        [] => "—".to_owned(),
        [only] => (*only).to_owned(),
        [init @ .., last] => format!("{} ou {last}", init.join(", ")),
    }
}
