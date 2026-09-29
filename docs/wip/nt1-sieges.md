# NT1 — Sièges variés (château, bourg fortifié, cité)

Spec : `docs/superpowers/specs/2026-09-29-nt-nuit-tww3-design.md` (ligne NT1). ADR 0126.
Branche : `feat/nt1-siege-layouts`. Cargo : `CARGO_TARGET_DIR=core/target-nt1`.

## État
- [x] Données : bloc `places` de `data/rules/siege_town.json` + schéma (type par genre de localité et
  fortification, règles du château et du bourg).
- [x] Squelette : `core/crates/sim-battle/src/siege_layouts.rs` (`PlaceKind`, `PlaceRules`,
  `place_seed`, `SiegeWorks::generate_place`), `SiegeSetup.place`, `SiegeWorks.place`,
  `House.keep` (donjon, ne brûle pas), `for_battle(place, seed)`, campagne (`battle_request.rs`),
  pont (`houses[].keep`, `place`).
- [x] Génération château et bourg + tests (portes accessibles, rien hors enceinte ni sur le chemin
  de ronde, déterminisme, provinces différentes).
- [x] Rendu Godot : donjon = tour agrandie (`battle_siege.gd`), script `game/tests/nt1_siege_shot.gd` (+ `debug_stage_place_siege`).
- [x] ADR 0126.

- [x] Vérifs : `cargo fmt`, `clippy -D warnings`, `cargo test` du workspace (170 suites vertes), pytest
  (1271), `core/build.sh`, smoke Godot, `sb_siege_bars_test.gd`, `nt1_siege_shot.gd` en headless
  (château : 8 bâtiments dont 1 donjon ; bourg : 31 ; cité : 59).

## Prochaine étape
Lot terminé. Session principale : lancer `nt1_siege_shot.gd` avec affichage (captures dans
`docs/audit/captures/nt/`) et juger le rendu.

## Points ouverts
- Bourg moins dense que la cité (≈ 40 îlots) : à juger visuellement, réglable dans `places.borough`.
- Donjon rendu en tour ronde agrandie (pas de maquette de donjon carré dans le kit).
- Équilibre d'un assaut de château (petite enceinte, garnison serrée) à surveiller en partie pilote.
- `nt1_siege_shot.gd` ne capture qu'avec affichage (en headless, il ne vérifie que la mise en place).
