# FK2 — assets Blender de la carte vivante

Branche `feat/fk2-assets`. Spec : `docs/design/2026-09-29-carte-vivante-folk.md` § 2.3, § 3.

## État
- [x] `tools/blender_scripts/folk_props.py` → `game/assets/models/folk/*.glb` + `manifest.json`
  (15 modèles, mètres, +X devant, créneaux en repère Godot).
- [ ] Clips `scythe`, `carry`, `plough` (`battle_skinned.py` + `battle_skinned_poses.py`),
  rigs fins recuits (`battle_fine.py -- rigs`), style `folk` dans `BattleSkinned.STYLES`.
- [ ] Figurines civiles `villager_0..3` (fines cuites + grossières).
- [ ] Import Godot, smoke, tests des figurines.

## Prochaine étape
Poses des trois clips.
