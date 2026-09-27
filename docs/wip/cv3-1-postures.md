# CV3-1 — Postures d'armée et résultats nuancés (core)

Branche : `feat/cv3-1-postures`. Spec : `docs/design/2026-09-27-campagne-vivante.md` § 1, § 3.
Orchestration : `docs/wip/cv3-campagne-vivante.md`. ADR : `docs/decisions/0094-postures-embuscade.md` (0093 pris).

## Contrat 3D (pour CV3-2) — commité dans le squelette
`core/crates/sim-battle/src/setup.rs` :
- `BattleOpening { Standard (défaut), Ambush { victim: SideId } }` (serde `tag = "kind"`, snake_case),
  `BattleSetup.opening` (serde default, omis si Standard).
- `SideSetup.forced_march: bool`, `SideSetup.entrenched: bool`, `SideSetup.start_fatigue: f64`
  (jauge 0-100 de `Unit::fatigue`, rempli depuis `postures.json` `forced_march.start_fatigue`).
- Le lot 2 lit ces champs ; il n'a pas besoin de lire `postures.json`.

## État
- [x] Squelette : Stance étendu, `posture.rs`, `battle_outcome.rs`, règles/schémas/tests Python,
  `CoverMap` (data-model/src/cover.rs), `Army.morale_modifiers`, `BattleRequest.opening`.
- [ ] Postures : validation, effets de tour, vision, déclenchement embuscade, auto-résolution.
- [ ] Résultats nuancés branchés dans `apply_battle_result`.
- [ ] Pont : `get_stance_options`, classe de résultat exposée.
- [ ] Tests `cv3_postures.rs`, `cv3_outcomes.rs`, ancienne sauvegarde.

## Choix
- Couvert : forêt = raster de `forest_cover.json` (`splat.png` canal B), pas `forest_kind.png`
  (qui est la part de résineux, pas la couverture) ; marais = `wetlands.png` (max RGB), raster de
  `wetlands.json` ; bocage = terrain de la province. Sans rasters : terrain de la province.

## Prochaine étape
Implémenter `posture.rs` (validation + effets), puis le déclenchement dans `march.rs`.
