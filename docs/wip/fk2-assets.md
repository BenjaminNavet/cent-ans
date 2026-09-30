# FK2 — assets Blender de la carte vivante

Branche `feat/fk2-assets`. Spec : `docs/design/2026-09-29-carte-vivante-folk.md` § 2.3, § 3.

## État
- [x] `tools/blender_scripts/folk_props.py` → `game/assets/models/folk/*.glb` + `manifest.json`
  (15 modèles, mètres, +X devant, créneaux en repère Godot).
- [x] Clips `scythe` (41 im.), `carry`, `plough` (33 im., sur `Walk`) en fin de rig `human`
  fin (`battle_fine.py -- rigs`, clips antérieurs identiques octet pour octet) ; styles `folk`
  et `folk_carry` dans `BattleSkinned.STYLES`.
- [x] Figurines civiles `villager_0..3` fines (atlas 28-31) et grossières (`--no-rigs` : rig
  grossier inchangé, sans les clips de travail → repli marche/attente).
- [x] `battle_fine_bake.store_layer` : la bande d'atlas s'allonge sans perdre les couches.
- [x] Import Godot, smoke, `fk2_assets_test.gd` (fin et `--coarse-figures`), an1b, an1a, fg3, ga1 : OK.

## Prochaine étape
Lot terminé ; intégration dans `integration/fk` par l’orchestrateur, vérification visuelle (poses, accessoires) par la session principale.
