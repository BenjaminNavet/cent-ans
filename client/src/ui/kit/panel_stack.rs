//! Panel stack (port of game/scripts/ui/panel_stack.gd, audit A3 "Empilement des fenêtres").
//!
//! Every registered panel has a [`PanelTier`]:
//! - `Docked`: right-hand panels (province, colony). A central panel sets them aside (hidden,
//!   remembered); they come back when the last central panel closes;
//! - `Central`: big windows (faction, court, techs, diplomacy…), exclusive: opening one closes the
//!   other central panels;
//! - `Companion`: accompanies a central panel (character sheet next to the court). Closes the
//!   central panels it does not accompany, and closes with the ones it does;
//! - `Modal`: pause, settings, confirmations. While one is open, Escape belongs to it.
//!
//! Escape closes the top panel (the most recently opened). Closing = hide, then
//! [`PanelClosed`] so that the owner updates its own state (province deselected…). No game rule
//! here: layout only.
//!
//! Use: put [`StackedPanel`] on the panel's root `Node` (hidden panels have `display: none`;
//! the stack toggles it), then send [`PanelStackRequest`]s (`Open`, `Close`, `Toggle`,
//! `CloseTop`, `Reveal`). [`PanelStack`] is the pure state (unit-tested), usable without Bevy.

use std::collections::HashMap;

use bevy::prelude::*;
use serde_json::{Value, json};

use super::modal::ModalQueue;

/// Kind of a stacked panel (`PanelStack.Kind`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PanelTier {
    Docked,
    Central,
    Companion,
    Modal,
}

impl PanelTier {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Docked => "docked",
            Self::Central => "central",
            Self::Companion => "companion",
            Self::Modal => "modal",
        }
    }
}

/// Marker + identity of a stacked panel. `companion_of`: ids of the central panels a
/// `Companion` accompanies.
#[derive(Component, Debug, Clone, PartialEq)]
pub struct StackedPanel {
    pub tier: PanelTier,
    pub id: String,
    pub companion_of: Vec<String>,
}

impl StackedPanel {
    pub fn new(tier: PanelTier, id: &str) -> Self {
        Self {
            tier,
            id: id.to_owned(),
            companion_of: Vec::new(),
        }
    }

    /// A companion of the central panels `hosts`.
    pub fn companion(id: &str, hosts: &[&str]) -> Self {
        Self {
            tier: PanelTier::Companion,
            id: id.to_owned(),
            companion_of: hosts.iter().map(|host| (*host).to_owned()).collect(),
        }
    }
}

/// What the stack did to a panel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StackEffect {
    /// Became visible.
    Show(String),
    /// Hidden and set aside (docked panel under a central one): no `closed` notification.
    Suspend(String),
    /// Closed (hidden + [`PanelClosed`]).
    Close(String),
}

#[derive(Debug, Clone)]
struct Entry {
    tier: PanelTier,
    companion_of: Vec<String>,
}

/// Pure stack state.
#[derive(Resource, Default, Debug)]
pub struct PanelStack {
    entries: HashMap<String, Entry>,
    /// Visible panels, oldest first (the last one is on top).
    order: Vec<String>,
    /// Docked panels set aside by a central panel.
    suspended: Vec<String>,
}

impl PanelStack {
    /// Registers a panel (idempotent), closed.
    pub fn register(&mut self, id: &str, tier: PanelTier, companion_of: &[String]) {
        self.entries.entry(id.to_owned()).or_insert(Entry {
            tier,
            companion_of: companion_of.to_vec(),
        });
    }

    pub fn unregister(&mut self, id: &str) {
        self.entries.remove(id);
        self.order.retain(|other| other != id);
        self.suspended.retain(|other| other != id);
    }

    pub fn is_registered(&self, id: &str) -> bool {
        self.entries.contains_key(id)
    }

    pub fn tier_of(&self, id: &str) -> Option<PanelTier> {
        self.entries.get(id).map(|entry| entry.tier)
    }

    /// Visible panels, oldest first.
    pub fn visible(&self) -> &[String] {
        &self.order
    }

    pub fn is_open(&self, id: &str) -> bool {
        self.order.iter().any(|other| other == id)
    }

    pub fn top(&self) -> Option<&str> {
        self.order.last().map(String::as_str)
    }

    pub fn is_suspended(&self, id: &str) -> bool {
        self.suspended.iter().any(|other| other == id)
    }

    pub fn has_central_open(&self) -> bool {
        self.order.iter().any(|id| {
            matches!(
                self.tier_of(id),
                Some(PanelTier::Central | PanelTier::Companion)
            )
        })
    }

    pub fn has_modal_open(&self) -> bool {
        self.order
            .iter()
            .any(|id| self.tier_of(id) == Some(PanelTier::Modal))
    }

    /// Opens `id` and applies the tier rules. Opening an open panel only brings it on top.
    pub fn open(&mut self, id: &str) -> Vec<StackEffect> {
        let Some(tier) = self.tier_of(id) else {
            return Vec::new();
        };
        let mut effects = Vec::new();
        let was_open = self.is_open(id);
        self.order.retain(|other| other != id);
        self.order.push(id.to_owned());
        if !was_open {
            effects.push(StackEffect::Show(id.to_owned()));
        }
        match tier {
            PanelTier::Docked => {
                if self.has_central_open() {
                    // Refresh of a set-aside panel: it stays aside.
                    if !self.is_suspended(id) {
                        self.suspended.push(id.to_owned());
                    }
                    self.order.retain(|other| other != id);
                    effects.retain(|effect| *effect != StackEffect::Show(id.to_owned()));
                    effects.push(StackEffect::Suspend(id.to_owned()));
                }
            }
            PanelTier::Central => {
                for other in self.order.clone() {
                    if other == id {
                        continue;
                    }
                    match self.tier_of(&other) {
                        Some(PanelTier::Central) => self.close_quiet(&other, &mut effects),
                        Some(PanelTier::Companion)
                            if !self.hosts_of(&other).contains(&id.to_owned()) =>
                        {
                            self.close_quiet(&other, &mut effects);
                        }
                        _ => {}
                    }
                }
                self.suspend_docked(&mut effects);
            }
            PanelTier::Companion => {
                let hosts = self.hosts_of(id);
                for other in self.order.clone() {
                    if other == id {
                        continue;
                    }
                    match self.tier_of(&other) {
                        Some(PanelTier::Central) if !hosts.contains(&other) => {
                            self.close_quiet(&other, &mut effects);
                        }
                        Some(PanelTier::Companion) => self.close_quiet(&other, &mut effects),
                        _ => {}
                    }
                }
                self.suspend_docked(&mut effects);
            }
            PanelTier::Modal => {}
        }
        effects
    }

    /// Closes `id` (hidden, then `closed`), then applies the follow-ups: a central panel takes
    /// its companions with it, and docked panels come back when no central one is left.
    pub fn close(&mut self, id: &str) -> Vec<StackEffect> {
        let Some(tier) = self.tier_of(id) else {
            return Vec::new();
        };
        let mut effects = Vec::new();
        self.suspended.retain(|other| other != id);
        let was_open = self.is_open(id);
        self.order.retain(|other| other != id);
        effects.push(StackEffect::Close(id.to_owned()));
        if !was_open {
            return effects; // hiding a hidden panel changes nothing else
        }
        if tier == PanelTier::Central {
            for other in self.order.clone() {
                if self.tier_of(&other) == Some(PanelTier::Companion)
                    && self.hosts_of(&other).contains(&id.to_owned())
                {
                    self.close_quiet(&other, &mut effects);
                }
            }
        }
        if matches!(tier, PanelTier::Central | PanelTier::Companion) && !self.has_central_open() {
            for docked in std::mem::take(&mut self.suspended) {
                if self.is_registered(&docked) {
                    self.order.push(docked.clone());
                    effects.push(StackEffect::Show(docked));
                }
            }
        }
        effects
    }

    /// Escape: closes the top panel. Empty when nothing is open or a modal owns the key.
    pub fn close_top(&mut self) -> Vec<StackEffect> {
        if self.has_modal_open() {
            return Vec::new();
        }
        match self.top().map(str::to_owned) {
            Some(id) => self.close(&id),
            None => Vec::new(),
        }
    }

    /// A docked panel is asked for a **new** selection while a central panel is open: the central
    /// panels close and the set-aside docked ones are forgotten.
    pub fn reveal(&mut self, id: &str) -> Vec<StackEffect> {
        if !self.is_suspended(id) && !self.has_central_open() {
            return Vec::new();
        }
        self.suspended.clear();
        let mut effects = Vec::new();
        for other in self.order.clone() {
            if matches!(
                self.tier_of(&other),
                Some(PanelTier::Central | PanelTier::Companion)
            ) {
                self.order.retain(|x| *x != other);
                effects.push(StackEffect::Close(other));
            }
        }
        effects
    }

    /// The owner closed a set-aside docked panel (deselection): it will not come back.
    pub fn forget_suspended(&mut self, id: &str) {
        self.suspended.retain(|other| other != id);
    }

    fn hosts_of(&self, id: &str) -> Vec<String> {
        self.entries
            .get(id)
            .map(|entry| entry.companion_of.clone())
            .unwrap_or_default()
    }

    /// Closed by the exclusivity rules: hidden + `closed`, without re-running follow-ups.
    fn close_quiet(&mut self, id: &str, effects: &mut Vec<StackEffect>) {
        self.order.retain(|other| other != id);
        effects.push(StackEffect::Close(id.to_owned()));
    }

    fn suspend_docked(&mut self, effects: &mut Vec<StackEffect>) {
        for other in self.order.clone() {
            if self.tier_of(&other) == Some(PanelTier::Docked) {
                if !self.is_suspended(&other) {
                    self.suspended.push(other.clone());
                }
                self.order.retain(|x| *x != other);
                effects.push(StackEffect::Suspend(other));
            }
        }
    }

    pub fn dump(&self) -> Value {
        let mut registered: Vec<(&String, &Entry)> = self.entries.iter().collect();
        registered.sort_by(|a, b| a.0.cmp(b.0));
        json!({
            "registered": registered
                .iter()
                .map(|(id, entry)| json!({"id": id, "tier": entry.tier.label()}))
                .collect::<Vec<_>>(),
            "visible": self.order,
            "suspended": self.suspended,
            "top": self.top(),
            "central_open": self.has_central_open(),
            "modal_open": self.has_modal_open(),
        })
    }
}

// ---- Bevy layer ----------------------------------------------------------------------------

/// Request to the stack (by panel id).
#[derive(Message, Debug, Clone, PartialEq, Eq)]
pub enum PanelStackRequest {
    Open(String),
    Close(String),
    /// Opens a closed panel, closes an open one.
    Toggle(String),
    /// Escape.
    CloseTop,
    Reveal(String),
    ForgetSuspended(String),
}

/// The stack closed a panel (by exclusivity, Escape or request): the owner updates its state
/// (Godot `closed` signal).
#[derive(Message, Debug, Clone, PartialEq, Eq)]
pub struct PanelClosed {
    pub id: String,
}

/// The set of visible panels changed (Godot `changed`: the HUD re-places itself).
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct PanelStackChanged;

pub struct PanelStackPlugin;

impl Plugin for PanelStackPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PanelStack>()
            .add_message::<PanelStackRequest>()
            .add_message::<PanelClosed>()
            .add_message::<PanelStackChanged>()
            .add_observer(unregister_removed)
            .add_systems(
                Update,
                escape_closes_top.before(crate::menu::pause::pause_escape),
            )
            .add_systems(Update, (register_new, apply_requests).chain());
    }
}

fn unregister_removed(
    remove: On<Remove<StackedPanel>>,
    panels: Query<&StackedPanel>,
    mut stack: ResMut<PanelStack>,
) {
    if let Ok(panel) = panels.get(remove.entity) {
        stack.unregister(&panel.id);
    }
}

/// Escape closes the top panel unless a kit modal is up (its own Escape handler runs later in
/// the frame, and this system runs first, so the modal still counts as busy).
fn escape_closes_top(
    mut edges: ResMut<crate::menu::KeyEdges>,
    modals: Option<Res<ModalQueue>>,
    stack: Res<PanelStack>,
    focus: Option<Res<super::text_field::TextInputFocus>>,
    mut requests: MessageWriter<PanelStackRequest>,
) {
    let typing = focus.is_some_and(|f| f.is_active());
    if edges.escape
        && !typing
        && stack.top().is_some()
        && !modals.is_some_and(|queue| queue.is_busy())
    {
        requests.write(PanelStackRequest::CloseTop);
        // The panel on top consumed the key: the pause menu must not open as well.
        edges.escape = false;
    }
}

/// Registers new [`StackedPanel`]s; one that is displayed counts as open.
fn register_new(
    mut stack: ResMut<PanelStack>,
    new: Query<(&StackedPanel, Option<&Node>), Added<StackedPanel>>,
    mut requests: MessageWriter<PanelStackRequest>,
) {
    for (panel, node) in &new {
        stack.register(&panel.id, panel.tier, &panel.companion_of);
        if node.is_some_and(|node| node.display != Display::None) {
            requests.write(PanelStackRequest::Open(panel.id.clone()));
        }
    }
}

fn apply_requests(
    mut requests: MessageReader<PanelStackRequest>,
    mut stack: ResMut<PanelStack>,
    mut panels: Query<(&StackedPanel, &mut Node)>,
    mut closed: MessageWriter<PanelClosed>,
    mut changed: MessageWriter<PanelStackChanged>,
) {
    for request in requests.read() {
        let effects = match request {
            PanelStackRequest::Open(id) => stack.open(id),
            PanelStackRequest::Close(id) => stack.close(id),
            PanelStackRequest::Toggle(id) => {
                if stack.is_open(id) {
                    stack.close(id)
                } else {
                    stack.open(id)
                }
            }
            PanelStackRequest::CloseTop => stack.close_top(),
            PanelStackRequest::Reveal(id) => stack.reveal(id),
            PanelStackRequest::ForgetSuspended(id) => {
                stack.forget_suspended(id);
                Vec::new()
            }
        };
        if effects.is_empty() {
            continue;
        }
        for effect in &effects {
            let (id, display) = match effect {
                StackEffect::Show(id) => (id, Display::Flex),
                StackEffect::Suspend(id) | StackEffect::Close(id) => (id, Display::None),
            };
            for (panel, mut node) in &mut panels {
                if panel.id == *id && node.display != display {
                    node.display = display;
                }
            }
            if let StackEffect::Close(id) = effect {
                closed.write(PanelClosed { id: id.clone() });
            }
        }
        changed.write(PanelStackChanged);
    }
}

/// Exclusivity of the campaign panels, driven by [`OpenPanel`] (one central screen at a time, a
/// central screen replaces the docked panels and the other way round, `CloseAll` closes all).
/// Each panel owner calls [`displaces`] with its own panel id (`diplomacy`, `tech`, `chronicle`,
/// `feudal`, `court`, `faction` central; `province`, `army`, `siege` docked) and closes itself
/// when it answers true. Docked panels keep company with each other (their owners decide).
pub fn displaces(message: &crate::campaign::panels::OpenPanel, own_id: &str) -> bool {
    use crate::campaign::panels::OpenPanel as P;
    const CENTRAL: [&str; 7] = [
        "diplomacy",
        "tech",
        "chronicle",
        "feudal",
        "court",
        "faction",
        "economy",
    ];
    let own_central = CENTRAL.contains(&own_id);
    let central = |id: &str| !(own_id == id);
    match message {
        P::CloseAll => true,
        P::Diplomacy => central("diplomacy"),
        P::Tech => central("tech"),
        P::Chronicle => central("chronicle"),
        P::Feudal => central("feudal"),
        P::Council => central("court"),
        P::Faction(_) => central("faction"),
        P::Economy => central("economy"),
        // The character sheet accompanies a central screen (faction, court).
        P::Character(_) => !own_central,
        P::Province(_) | P::Army(_) | P::Recruitment(_) | P::Siege(_) => own_central,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stack() -> PanelStack {
        let mut stack = PanelStack::default();
        stack.register("province", PanelTier::Docked, &[]);
        stack.register("colony", PanelTier::Docked, &[]);
        stack.register("faction", PanelTier::Central, &[]);
        stack.register("tech", PanelTier::Central, &[]);
        stack.register("court", PanelTier::Central, &[]);
        stack.register("character", PanelTier::Companion, &["court".to_owned()]);
        stack.register("pause", PanelTier::Modal, &[]);
        stack
    }

    #[test]
    fn central_closes_other_central() {
        let mut stack = stack();
        stack.open("faction");
        let effects = stack.open("tech");
        assert!(effects.contains(&StackEffect::Close("faction".into())));
        assert_eq!(stack.visible(), ["tech"]);
    }

    #[test]
    fn central_sets_docked_aside_and_restores_them() {
        let mut stack = stack();
        stack.open("province");
        let effects = stack.open("faction");
        assert!(effects.contains(&StackEffect::Suspend("province".into())));
        assert!(stack.is_suspended("province"));
        assert_eq!(stack.visible(), ["faction"]);
        let effects = stack.close("faction");
        assert!(effects.contains(&StackEffect::Show("province".into())));
        assert_eq!(stack.visible(), ["province"]);
    }

    #[test]
    fn docked_opened_under_central_stays_aside() {
        let mut stack = stack();
        stack.open("faction");
        let effects = stack.open("colony");
        assert_eq!(effects, [StackEffect::Suspend("colony".into())]);
        assert!(!stack.is_open("colony") && stack.is_suspended("colony"));
        stack.close("faction");
        assert!(stack.is_open("colony"));
    }

    #[test]
    fn companion_follows_its_host_only() {
        let mut stack = stack();
        stack.open("court");
        stack.open("character");
        assert_eq!(stack.visible(), ["court", "character"]);
        // Another central panel closes the court and its companion.
        let effects = stack.open("tech");
        assert!(effects.contains(&StackEffect::Close("court".into())));
        assert!(effects.contains(&StackEffect::Close("character".into())));
        assert_eq!(stack.visible(), ["tech"]);
        // Closing the host closes its companion.
        stack.open("court");
        stack.open("character");
        let effects = stack.close("court");
        assert!(effects.contains(&StackEffect::Close("character".into())));
        assert!(stack.visible().is_empty());
    }

    #[test]
    fn companion_closes_unrelated_central() {
        let mut stack = stack();
        stack.open("faction");
        let effects = stack.open("character");
        assert!(effects.contains(&StackEffect::Close("faction".into())));
    }

    #[test]
    fn escape_closes_top_unless_modal() {
        let mut stack = stack();
        stack.open("province");
        stack.open("pause");
        assert!(stack.close_top().is_empty());
        stack.close("pause");
        assert_eq!(stack.close_top(), [StackEffect::Close("province".into())]);
        assert!(stack.close_top().is_empty());
    }

    #[test]
    fn reveal_closes_central_and_forgets_aside() {
        let mut stack = stack();
        stack.open("province");
        stack.open("faction");
        let effects = stack.reveal("province");
        assert_eq!(effects, [StackEffect::Close("faction".into())]);
        assert!(!stack.is_suspended("province"));
        // Nothing to reveal when no central panel is open.
        assert!(stack.reveal("province").is_empty());
    }

    #[test]
    fn unregister_forgets_everything() {
        let mut stack = stack();
        stack.open("faction");
        stack.unregister("faction");
        assert!(stack.visible().is_empty() && !stack.is_registered("faction"));
        assert!(stack.open("faction").is_empty());
    }

    #[test]
    fn reopening_brings_on_top() {
        let mut stack = stack();
        stack.open("pause");
        stack.open("province");
        stack.open("pause");
        assert_eq!(stack.top(), Some("pause"));
    }

    #[test]
    fn central_panels_displace_each_other_and_docked() {
        use crate::campaign::panels::OpenPanel as P;
        assert!(displaces(&P::Tech, "diplomacy"));
        assert!(!displaces(&P::Tech, "tech"));
        assert!(displaces(&P::Feudal, "province"));
        assert!(displaces(&P::Province("p".into()), "chronicle"));
        assert!(!displaces(&P::Province("p".into()), "army"));
        assert!(!displaces(&P::Character("c".into()), "faction"));
        assert!(displaces(&P::Character("c".into()), "province"));
        assert!(displaces(&P::CloseAll, "court"));
    }
}
