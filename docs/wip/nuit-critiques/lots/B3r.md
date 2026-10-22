# Lot B3r — réintégration de B3 (carte) dans main

## État
- [x] `13a834f88` (B3, ADR 0730) rejoué par cherry-pick sur main (`72204be4b`) ; 8 binaires en conflit
      (`colormap_bc1_*.bin`, `colormap_preview.jpg`, `navgrid.png`, `province_border_dist.png`, `splat.png`) pris côté main.
- [x] Chaîne géométrique relancée depuis les données de main (côtes/lacs conservés : `land_mask.png`, `heightmap*.png`,
      `coastline.geojson`, `height/` non modifiés) : `geo provinces`, `settlements`, `roads --computed`, `splat`,
      `rivers-render`, `hamlets`, `navgrid`, `colormap`, `sea-lanes`, `compose authored-px`, `horizon`, `cross_fixture`.
      Pas relancés (écraseraient les retouches de côte) : `geo compose`, `compose-relief`, `compose-layers`.
- [x] ADR 0730 : aucun 0730 sur main, numéro conservé.
- [x] `data/map/real/` (gitignoré) copié depuis le checkout principal pour la chaîne.

## Validation
- pytest : 2910 passed, 3 skipped.
- clippy `-D warnings` : propre.
- `cargo test --workspace` : 4 échecs (`m3_sequential_turn::ai_factions_play_one_after_the_other_in_id_order`,
  `campaign::save_load_round_trip`, `m5::save_round_trip_keeps_diplomacy`, `c5_trade::trade_state_survives_saves`),
  **déjà présents sur main** (vérifié sur m3 : ordre de jeu des factions IA et index du RNG, sans lien avec la carte).
- seed-bots `--quick` : avant (main) aucun constat ; après 1 constat SlowTurn (diplomate England tour 19, 5265 ms pour
  5000), passes lancées en parallèle sur une machine saturée (load > 100) : bruit de charge probable.

## Points ouverts
- Les 4 tests rouges de main sont à traiter dans un lot à part.
