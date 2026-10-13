//! Lot RE R2 (ADR 0353): an army of an older save standing on a cell that a
//! mountain mass made impassable is moved to the nearest passable cell when
//! the save is loaded.
use data_model::test_support::{fac, game_data};
use data_model::{GameData, MapRasters, NavGrid, IMPASSABLE, PLAIN_COST};
use sim_campaign::march::unstick_field_armies;
use sim_campaign::test_support::main_army;
use sim_campaign::{ArmyPosition, CampaignState};

/// The real data on a 512² plain grid with a wall block in the middle.
fn walled_data() -> GameData {
    let mut data = game_data().clone();
    let mut grid = NavGrid::uniform(512, 512, 2, 1.438, PLAIN_COST);
    for y in 200..260 {
        for x in 200..260 {
            grid.set(x, y, IMPASSABLE);
        }
    }
    grid.refresh_min_cost();
    data.set_map_rasters(MapRasters {
        navgrid: grid,
        provinces: None,
    });
    data
}

#[test]
fn army_inside_a_mass_moves_to_the_nearest_free_cell() {
    let data = walled_data();
    let mut state = CampaignState::new_1337(&data, fac("fac_france"), 5).unwrap();
    let army = main_army(&state, "fac_france");
    let grid = data.navgrid();
    // Cell (203, 230): three cells inside the west face of the wall.
    let inside = grid.cell_center(203, 230);
    state.armies.get_mut(&army).unwrap().position = ArmyPosition::field(inside);
    let moved = unstick_field_armies(&mut state, &data);
    assert!(moved >= 1);
    let ArmyPosition::Field { x, y } = state.armies[&army].position else {
        panic!("still a field army");
    };
    let (cx, cy) = grid.cell_of(x, y);
    assert!(grid.passable(i64::from(cx), i64::from(cy)));
    assert_eq!(
        (cx, cy),
        (199, 230),
        "nearest free cell, just west of the wall"
    );
    // A second pass moves nobody.
    assert_eq!(unstick_field_armies(&mut state, &data), 0);
}
