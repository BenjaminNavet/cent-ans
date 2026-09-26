# FG1 — Corps humain des figurines fines (MakeHuman en production)

Branche : `feat/fg1-corps` (worktree agent). Plan : `docs/wip/fg-figurines-fines.md`.
Prototype : `docs/wip/fg0-prototype.md`.

## État
- [ ] Rig aux proportions réalistes (`battle_fine_rig.py`), recuisson `human` / `cavalry`
- [ ] Vérification des poses calculées (arc, arbalète, piques, étriers)
- [ ] Visages (cibles MPFB) et cheveux/barbes
- [ ] Recettes fines, chaîne LOD, export `CAM1` dans `game/assets/models/battle_fine/`
- [ ] Drapeau `--fine-figures` dans `BattleSkinned`
- [ ] Captures, smoke test, primitives `--units=50`

## Commandes
```
blender -b --factory-startup --python tools/blender_scripts/battle_fine.py -- rigs
blender -b --factory-startup --python tools/blender_scripts/battle_fine.py -- figures [--only infantry_0]
blender -b --factory-startup --python tools/blender_scripts/battle_fine.py -- check <dir>
```

## Prochaine étape
Cuire les rigs et vérifier les proportions.
