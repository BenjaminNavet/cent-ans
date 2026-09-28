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
   - Test `review_tests::precomputed_trade_paths_match_the_old_search` (ancienne recherche recopiée
     à l'identique, comparaison route par route + `trade_routes` avec et sans cache).
   - Chrono : `review_tests::trade_paths_timing` (ignoré) — mesure à faire.
2. Tests de campagne n° 4, 13, 14, 15 : à faire.
3. Pavois / cible cachée (sim-battle, CB4) : à revérifier.
4. Sonde release `fifty_turns_on_eight_seeds_stay_in_the_c7a_band` : à lancer.

## Prochaine étape

Mesure du gain (release), puis tests 4/13/14/15 dans `review_tests.rs`.
