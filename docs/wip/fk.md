# FK — carte vivante (gens, marchands, scènes, incidents)

Spec : `docs/design/2026-09-29-carte-vivante-folk.md`. ADR : 0122 (expiration → option de l'IA).
Coût cloud : 0 $.

## État
- [x] FK0 squelette : `map_scenes.rs` (API vide), `tests/fk_map_scenes.rs` (#[ignore]),
  `game/scripts/map/life_folk/*.gd` vides, `game/tests/fk_folk_test.gd` (SKIPPED), ADR 0122.
- [ ] Vague 1 (worktrees) : FK1 cœur, FK2 assets, FK3 pool + routine + charrettes.
- [ ] Vague 2 : FK4 scènes, FK5 incidents + 15 événements.
- [ ] FK6 : A/B `--no-folk`, captures (≤ 3), relecture, fusion.

## Prochaine étape
Lancer la vague 1 (branches `feat/fk1-core`, `feat/fk2-assets`, `feat/fk3-folk`), intégration
dans `integration/fk`.

## Points ouverts
