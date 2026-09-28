# RS-D — Commerce et tests de campagne (fichier de reprise)

Branche `feat/rs-d-trade` (depuis `main` c0be4da7). Orchestration : `docs/wip/restes.md`.
Cible cargo privée : `CARGO_TARGET_DIR=core/target-rs-d` (à supprimer à la fin).
Hors périmètre (RS-B en parallèle) : `economy.rs`, `settlements.rs`, `population.rs`, `medicine.rs`, `economy.json`.

## État

1. Chemins commerciaux précalculés (revue-code 18c) : FAIT.
   - `data-model/src/trade_paths.rs` : `shortest_path` (Dijkstra identique à l'ancien de `trade.rs`),
     `GameData::build_trade_paths` (appelé à la fin de `build_movement_graph`, donc au chargement
     et à chaque reconstruction du graphe), `GameData::trade_path` (cache, sinon recherche).
   - `sim-campaign/src/trade.rs` lit `data.trade_path`.
   - Test `review_tests::precomputed_trade_paths_match_the_old_search`.
   - Gain (release, `review_tests::trade_paths_timing`, ignoré) : `trade_routes` 28,3 ms → 0,73 ms
     par appel ; précalcul 8,9 ms une fois au chargement.
2. Tests de campagne n° 4, 13 (deux tests), 14, 15 : FAITS dans `sim-campaign/src/review_tests.rs`.
   Vérifiés par mutation : chacun échoue quand on retire son correctif. `naval::own_ships_lost` passé `pub(crate)`.
3. Pavois / cible cachée : toujours ouvert après CB4. Test ignoré
   `sim-battle/tests/review_fixes.rs::pavised_crossbowmen_close_in_on_a_hidden_target` (échoue : z reste 380).
   Correction de règle (`start_attack` : tester aussi la visibilité) laissée à un lot bataille.
4. Sonde release `fifty_turns_on_eight_seeds_stay_in_the_c7a_band` : échoue sur le trésor moyen français
   38 297 < 40 000 ; identique sans le cache (dérive préexistante). Toutes les assertions par graine passent.
5. `revue-code.md` coché.

## Prochaine étape

Résultat de la sonde, fmt/clippy/test complets, `git merge main`, retests, suppression de `core/target-rs-d`.
