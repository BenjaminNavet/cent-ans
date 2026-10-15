//! `devtools::shuffle_all` really changes the order of unconstrained systems and keeps the
//! constrained ones in order (dev builds only: `cargo test --features dev`).
#![cfg(feature = "dev")]

use bevy::prelude::*;
use cent_ans_client::agent::devtools::shuffle_all;

#[derive(Resource, Default)]
struct Order(Vec<&'static str>);

fn run(seed: Option<u64>) -> Vec<&'static str> {
    let mut app = App::new();
    app.init_resource::<Order>();
    app.add_systems(
        Update,
        (
            |mut o: ResMut<Order>| o.0.push("a"),
            |mut o: ResMut<Order>| o.0.push("b"),
            |mut o: ResMut<Order>| o.0.push("c"),
            (
                |mut o: ResMut<Order>| o.0.push("x1"),
                |mut o: ResMut<Order>| o.0.push("x2"),
            )
                .chain(),
        ),
    );
    if let Some(seed) = seed {
        shuffle_all(&mut app.world_mut().resource_mut::<Schedules>(), seed);
    }
    app.update();
    app.world().resource::<Order>().0.clone()
}

#[test]
fn shuffle_changes_order_and_keeps_constraints() {
    let mut orders = std::collections::BTreeSet::new();
    for seed in 0..20 {
        let order = run(Some(seed));
        let x1 = order.iter().position(|s| *s == "x1").unwrap();
        let x2 = order.iter().position(|s| *s == "x2").unwrap();
        assert!(x1 < x2, "chain kept for seed {seed}: {order:?}");
        assert_eq!(order.len(), 5);
        orders.insert(order);
    }
    assert!(orders.len() > 1, "20 seeds gave one single order");
    // Same seed, same order.
    assert_eq!(run(Some(7)), run(Some(7)));
}

/// The console toggle keys must not collide with a default action of the `KeyMap` (F12 does:
/// `map_screenshot`, so it is not a console key).
#[test]
fn console_keys_are_free_in_the_default_keymap() {
    use cent_ans_client::settings::keymap::{Chord, KeyMap};
    let map = KeyMap::defaults();
    for key in ["F8", "`"] {
        let chord = Chord::parse(key).expect("known key");
        assert_eq!(map.action_using(&chord, ""), None, "{key} is bound");
    }
    let f12 = Chord::parse("F12").unwrap();
    assert!(map.action_using(&f12, "").is_some());
}
