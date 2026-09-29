# FK4 — scènes de province (rendu GDScript)

Branche `feat/fk4-scenes` (base `integration/fk`, 299233c8c). Spec
`docs/design/2026-09-29-carte-vivante-folk.md` §§ 2.2, 3.3, 5-7 ; suivi `docs/wip/fk.md`.

## État
- [x] Réglages alignés : `data/rules/map_scenes.json` = source unique (schéma, `MapSceneRules`,
  pont `get_map_scene_rules` complet) ; clés du rendu ajoutées (`figure_height`,
  `road_folk_per_unit`, `*_probability`, `guard_value`, `scene_figures_min/max`) ;
  `carts_per_value` → `carts_per_trade_value` (valeur du rendu, 0,02) ; `peasants_per_thousand`
  supprimé (inutilisé : la routine se règle par la population relative). pytest : toute clé lue
  par `life_folk/*.gd` est dans le schéma et le fichier.
- [ ] `folk_scenes.gd` (10 types), branchement `CampaignLife` (`--scene=`, `--folk-off=scenes`).
- [ ] Peste : cheminées coupées (`LifeEffects.quiet_settlements`) ; famine : champs vides
  (`FolkRoutine.idle_provinces`) ; fumées de bûcher et d'émeute (`LifeEffects.scene_fires`).
- [ ] Tests `fk_folk_test.gd` (chaque type forcé, loin vide, repli chef-lieu), smoke.

## Prochaine étape
Écrire `folk_scenes.gd`.
