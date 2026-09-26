# FG1 — Corps humain des figurines fines (MakeHuman en production)

Branche : `feat/fg1-corps` (worktree agent). Plan : `docs/wip/fg-figurines-fines.md`.
Prototype : `docs/wip/fg0-prototype.md`.

## État
- [x] Rig aux proportions réalistes (`battle_fine_rig.py`), rigs `human` / `cavalry` recuits
  dans `game/assets/models/battle_fine/` (mêmes os, mêmes clips ; os du cheval intouchés)
- [x] Arc long : main d'arc placée sur la ligne ancre (commissure droite) + visée, torse
  tourné à 60° ; la corde et la main droite finissent à la joue (±3 cm)
- [x] Visages : 8 clés de forme MPFB dans `fg_base_male.blend` (`fg_makehuman_base.py`,
  `FACES`), une tête par variante au LOD0 ; cheveux courts et barbes en coques
- [x] Recettes fines (`battle_fine_figures.py`) : tenues par pièces Quaternius (harnois,
  tunique, paysan, capuche), équipement V2 reconstruit sur le corps, kit FG0 pour
  `infantry_0` / `cavalry_0`, tabard drapé
- [x] Drapeau `--fine-figures` dans `BattleSkinned` (manifeste fusionné, rigs `fine_*`)
- [ ] Toutes les recettes exportées, vérifiées (`battle_fine_check.py`)
- [ ] Captures, smoke test, primitives `--units=50`

## Proportions retenues (rig, pose de liaison)
| Segment | Quaternius | FG1 |
|---|---|---|
| hanches → cou | 0,60 m | 0,63 m (×1,05 sur Abdomen/Torso/Chest/Neck) |
| cou → tête (pivot) | 0,074 m | 0,097 m |
| épaule (articulation) | x ±0,15, z 1,37 | x ±0,18, 3 cm plus bas |
| humérus | 0,176 m | 0,251 m |
| avant-bras | 0,228 m | 0,256 m |
| jambes, pieds | inchangés | inchangés (pieds enfants de `Root`, translations animées) |

Corps MakeHuman ajusté à l'échelle 0,955 (tronc) au lieu de 0,904 : 1,70 m, tête à sa
taille réelle (la grosse tête Quaternius n'est pas reproduite : corrigée par l'échelle du
corps, pas par le rig).

## Commandes
```
blender -b --python tools/blender_scripts/fg_makehuman_base.py          # MPFB requis (visages)
blender -b --factory-startup --python tools/blender_scripts/battle_fine.py -- rigs
blender -b --factory-startup --python tools/blender_scripts/battle_fine.py -- figures [--only infantry_0]
blender -b --factory-startup --python tools/blender_scripts/battle_fine_check.py -- <dir>
godot --path game -- --fine-figures
```

## Prochaine étape
Export de toutes les recettes, vérification cuite, captures en jeu et primitives.
