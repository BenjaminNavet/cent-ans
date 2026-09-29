# FK — carte vivante (gens, marchands, scènes, incidents)

Spec : `docs/design/2026-09-29-carte-vivante-folk.md`. ADR : 0122 (expiration → option de l'IA).
Coût cloud : 0 $.

## État
- [x] FK0 squelette : `map_scenes.rs` (API vide), `tests/fk_map_scenes.rs` (#[ignore]),
  `game/scripts/map/life_folk/*.gd` vides, `game/tests/fk_folk_test.gd` (SKIPPED), ADR 0122.
- [~] Vague 1 lancée 09-29 (3 agents cent-ans-dev en worktree) : FK1 cœur, FK2 assets, FK3 pool + routine + charrettes.
- [ ] Vague 2 : FK4 scènes, FK5 incidents + 15 événements.
- [ ] FK6 : A/B `--no-folk`, captures (≤ 3), relecture, fusion.

## Intégration
Worktree `../game_project-fk`, branche `integration/fk` : FK2 + FK3 fusionnés (25273ae81). Rebranchement des modèles FK2 dans `FolkModels` en cours (figurines `villager_*`, accessoires face +X). FK1 (feat/fk1-core) : taux d'incidents mesuré 0,147/tour (sous la cible 0,25-0,5 ; FK5 ajoute 15 événements).
FK3 : clés de `map_scenes.json` lues par le rendu à aligner avec le schéma FK1 (voir docs/wip/fk3-folk.md).

## Prochaine étape
Lancer la vague 1 (branches `feat/fk1-core`, `feat/fk2-assets`, `feat/fk3-folk`), intégration
dans `integration/fk`.

## Points ouverts
- FK2 fini (feat/fk2-assets, a31bbf3a9) : 15 .glb + manifest.json, clips scythe/carry/plough (rig fin, 61 clips), figurines `villager_0..3` ; note docs/wip/fk2-assets.md. À vérifier en capture : faux vs jambe gauche, sac vs tête, torche non émissive, lisibilité à l'échelle de la carte ; rig grossier sans les 3 clips (repli marche/attente).
