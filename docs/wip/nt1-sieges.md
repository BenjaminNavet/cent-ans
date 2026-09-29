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
- [ ] Génération château et bourg + tests (portes accessibles, rien hors enceinte ni sur le chemin
  de ronde, déterminisme, provinces différentes).
- [ ] Rendu Godot : donjon = tour agrandie (`battle_siege.gd`), script `game/tests/nt1_siege_shot.gd`.
- [ ] ADR 0126.

## Prochaine étape
Implémenter `generate_place` (château puis bourg) dans `siege_layouts.rs`.

## Points ouverts
