//! Helpers shared by the AI integration tests.

use data_model::test_support::game_data;
use data_model::GameData;

/// The real game data, with the feudal AI installed whatever the order of the tests.
pub fn data() -> &'static GameData {
    ai::feudal::install();
    game_data()
}
