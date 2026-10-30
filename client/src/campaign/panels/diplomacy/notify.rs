//! Offers received at the end of a season: counted for the HUD pill.
//!
//! Ports the end-of-turn half of `map/diplomacy_controller.gd` (`after_end_turn`). Since
//! XP-AUDIENCE the screen no longer opens by itself and no toast announces an offer: the
//! Audience (`hud/audience.rs`) carries the `diplomacy_offer` Affair. The « Diplomatie » button
//! of the HUD shows how many offers wait (the « pastille »).

use std::collections::HashSet;

use bevy::prelude::*;
use game_api::diplomacy::{OfferView, offers};
use serde_json::{Value, json};

use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::campaign::hud::flow::TurnAdvanced;
use crate::sim::Sim;

/// Offer kinds that never open the screen on their own (`ROUTINE_OFFER_KINDS`).
pub const ROUTINE_OFFER_KINDS: [&str; 1] = ["treaty"];

/// Name of the HUD button carrying the pill.
const HUD_BUTTON: &str = "hud:open_diplomacy";
const HUD_CAPTION: &str = "Diplomatie";

/// Offers already shown to the player and what the last season brought (dump).
#[derive(Resource, Default)]
pub struct OfferWatch {
    seen: HashSet<u32>,
    pub last_fresh: usize,
    pub last_weighty: usize,
    pub waiting: usize,
    pub pill: String,
}

pub struct NotifyPlugin;

impl Plugin for NotifyPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<OfferWatch>()
            .register_dump(dump_watch)
            .add_systems(OnEnter(AppState::Campaign), reset)
            .add_systems(
                Update,
                (after_turn, update_pill)
                    .chain()
                    .run_if(in_state(AppState::Campaign)),
            );
    }
}

fn reset(mut watch: ResMut<OfferWatch>) {
    *watch = OfferWatch::default();
}

/// `(fresh, weighty)`: offers not yet shown, and among them those that are not routine. Marks
/// every offer as seen.
pub fn classify(seen: &mut HashSet<u32>, waiting: &[OfferView]) -> (usize, usize) {
    let (mut fresh, mut weighty) = (0, 0);
    for offer in waiting {
        if seen.insert(offer.id) {
            fresh += 1;
            if !ROUTINE_OFFER_KINDS.contains(&offer.kind.as_str()) {
                weighty += 1;
            }
        }
    }
    (fresh, weighty)
}

/// « 1 proposition diplomatique » / « 3 propositions diplomatiques ».
pub fn count_text(count: usize) -> String {
    if count == 1 {
        "1 proposition diplomatique".to_owned()
    } else {
        format!("{count} propositions diplomatiques")
    }
}

/// Counts the offers the season brought. XP-AUDIENCE: nothing opens or pops up by itself any more;
/// the Audience carries the offers (`diplomacy_offer` Affair) and the HUD pill counts them.
fn after_turn(
    mut turns: MessageReader<TurnAdvanced>,
    sim: Res<Sim>,
    mut watch: ResMut<OfferWatch>,
) {
    if turns.read().count() == 0 {
        return;
    }
    let (Some(data), Some(state)) = (&sim.data, &sim.state) else {
        return;
    };
    let waiting = offers(data, state, &sim.player);
    let (fresh, weighty) = classify(&mut watch.seen, &waiting);
    watch.last_fresh = fresh;
    watch.last_weighty = weighty;
}

/// Caption of the HUD button: « Diplomatie (n) » while offers wait.
fn update_pill(
    sim: Res<Sim>,
    mut watch: ResMut<OfferWatch>,
    buttons: Query<(&Name, &Children)>,
    mut texts: Query<&mut Text>,
) {
    if !sim.is_changed() && !watch.is_added() {
        return;
    }
    let count = match (&sim.data, &sim.state) {
        (Some(data), Some(state)) => offers(data, state, &sim.player).len(),
        _ => 0,
    };
    watch.waiting = count;
    let caption = if count == 0 {
        HUD_CAPTION.to_owned()
    } else {
        format!("{HUD_CAPTION} ({count})")
    };
    for (name, children) in &buttons {
        if name.as_str() != HUD_BUTTON {
            continue;
        }
        for child in children.iter() {
            if let Ok(mut text) = texts.get_mut(child)
                && text.0 != caption
            {
                text.0 = caption.clone();
            }
        }
    }
    watch.pill = caption;
}

fn dump_watch(world: &World) -> (String, Value) {
    let watch = world.resource::<OfferWatch>();
    (
        "offer_watch".to_owned(),
        json!({
            "seen": watch.seen.len(),
            "last_fresh": watch.last_fresh,
            "last_weighty": watch.last_weighty,
            "waiting": watch.waiting,
            "pill": watch.pill,
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn offer(id: u32, kind: &str) -> OfferView {
        OfferView {
            id,
            from: "fac_x".into(),
            from_name: "X".into(),
            kind: kind.into(),
            ultimatum: false,
            text: String::new(),
            expires_in: 2,
        }
    }

    #[test]
    fn routine_offers_never_count_as_weighty() {
        let mut seen = HashSet::new();
        let list = [offer(1, "treaty"), offer(2, "peace")];
        assert_eq!(classify(&mut seen, &list), (2, 1));
        // Already shown: nothing fresh.
        assert_eq!(classify(&mut seen, &list), (0, 0));
        assert_eq!(classify(&mut seen, &[offer(3, "treaty")]), (1, 0));
    }

    #[test]
    fn count_text_agrees() {
        assert_eq!(count_text(1), "1 proposition diplomatique");
        assert_eq!(count_text(3), "3 propositions diplomatiques");
    }
}
