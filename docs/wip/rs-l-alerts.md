# RS-L — Détail du siège dans `get_provinces_snapshot` — fichier de reprise

Branche `feat/rs-l-alerts`, depuis `main` (48f1a5f8). Suite de la note E (`docs/wip/restes.md`,
`docs/wip/rs-e-map.md` point 2) : `game/scripts/ui/alerts.gd::collect` gardait un appel
`get_province_state` par province assiégée pour lire le détail du siège (`attacker`, `supplies`,
`turns_elapsed`), absent de `get_provinces_snapshot`. Cible privée `core/target-rs-l`.

## Plan

1. `core/crates/godot-bridge/src/campaign_sim_provinces.rs` : ajouter à `get_provinces_snapshot`
   trois tableaux groupés `siege_attacker` (PackedStringArray), `siege_supplies`
   (PackedInt32Array), `siege_turns_elapsed` (PackedInt32Array), mêmes noms que dans le dict de
   `get_province_state`'s `siege`. Vides/0 si pas de siège.
2. `game/scripts/map/province_snapshot.gd` : lire ces 3 champs dans `ProvinceSnapshot.read`
   (chemin groupé) ; chemin de secours (`get_province_state`) inchangé pour compat.
3. `game/scripts/ui/alerts.gd` : remplacer le second appel `sim.call("get_province_state", ...)`
   par une lecture directe de `snapshot.siege_attacker/siege_supplies/siege_turns_elapsed`.
4. Test Rust du pont sur les nouveaux champs (`campaign_sim_provinces.rs` ou test existant du
   module). `smoke.gd`, tests d'alertes/carte existants.

## État — TERMINÉ (28/09)

- [x] Rust (`campaign_sim_provinces.rs`) : `get_provinces_snapshot` gagne `siege_attacker`
      (PackedStringArray), `siege_supplies`, `siege_turns_elapsed` (PackedInt32Array), lus via une
      nouvelle fonction pure `province_snapshot_row` (testable sans moteur Godot). `cargo fmt`,
      `clippy --all-targets -D warnings`, `cargo test --workspace` verts. 2 tests dédiés
      (`siege_fields_read_the_ongoing_siege`, `unknown_province_gives_no_row`).
- [x] `province_snapshot.gd` : `ProvinceSnapshot` lit les 3 champs dans le chemin groupé et dans
      le chemin de secours (`get_province_state`, pour les simulations factices des tests).
- [x] `alerts.gd` : le second appel `sim.call("get_province_state", ...)` par province assiégée
      est retiré ; `collect()` lit `snapshot.siege_attacker/siege_supplies/siege_turns_elapsed`.
- [x] Tests Godot : `smoke.gd` (2 suites OK, exit 0), `cb5_alerts_test.gd` (OK, sans rapport avec
      ce lot mais seul test « alert » du dépôt — alertes de bataille, pas alertes de carte). Pas
      de test dédié pour `CampaignAlerts.collect` (siège) dans le dépôt ; couvert indirectement
      par le flux `smoke.gd` (fin de tour, rapport de saison).

Dernier commit : voir `git log --oneline -3` sur `feat/rs-l-alerts`.

## Prochaine étape

Rien : lot terminé. Reste à fusionner via le worktree d'intégration (`../gp-rs-merge`,
`integration/rs`) puis `main`, comme les autres lots RS. Cible privée `core/target-rs-l`
supprimée en fin de lot.
