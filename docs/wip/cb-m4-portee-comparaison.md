# CB-M4 — Portée au sol et comparaison au survol (état)

Branche : `feat/cb-m4-range-compare` (partie de `main` 88ec35be, qui contient CB-M2).
Plan : `docs/superpowers/plans/2026-09-27-controles-bataille-tw.md` (section CB-M4).
Cible cargo privée : `CARGO_TARGET_DIR=<worktree>/core/target-cbm4`.

## État : implémentation écrite, tests en cours

- [x] Données : `range_arc.fire_half_angle_deg` (60°) dans `data/rules/battle_hover.json`,
      schéma et test pytest à jour. Le cœur ne restreint pas l'angle de tir (le tireur pivote
      vers sa cible) : c'est le secteur dessiné, couvert sans pivoter.
- [x] Cœur `hover.rs` : `RangeArcRules`, `BattleSim::ground_range(unit)` (portée effective
      contre le sol à ses pieds ; 0 sans tir ou sans munitions). Test `tests/cb_range.rs` (2).
- [x] Pont `get_units` : `effective_range`, `fire_arc` (radians) — deux lignes.
- [x] Godot `battle_range_arc.gd` (arc + bords du secteur, sommets à `get_walk_height`),
      `battle_compare_panel.gd` (9 lignes, vert/rouge selon `advantages`), branchés dans
      `battle_scene.gd::_update_outlines` (survolés = terrain, bannière, carte).
- [ ] Test Godot `cb_m4_range_compare_test.gd`, capture `cbm4_compare_shot.gd` (+ `--probe`).
- [ ] Vérifications finales (fmt, clippy, tests, pytest, smoke, tests CB).

## Prochaine étape

Écrire le test Godot et le script de capture, build (`core/build.sh` avec la cible privée),
`godot --import`, lancer les tests.
