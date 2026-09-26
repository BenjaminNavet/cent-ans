# Figurines de bataille fines (lot FG1)

Chargées par `BattleSkinned` seulement avec `--fine-figures` après `--` (le rendu par défaut
reste `battle_skinned/` jusqu'au lot FG5). Même format que `battle_skinned/` (voir son
`SOURCE.md` : `CAM1`, `CAB1`, `manifest.json`) ; le manifeste fusionné renomme les rigs
`fine_human` / `fine_cavalry`.

- **Générées par** : `tools/blender_scripts/battle_fine.py` (Blender 5.2,
  `blender -b --factory-startup --python tools/blender_scripts/battle_fine.py -- all`), avec
  `battle_fine_rig.py` (proportions du rig, pose de l'arc), `battle_fine_figures.py`
  (recettes : corps, vêtements, visages, niveaux de détail) et les modules FG0
  (`battle_fine_proto.py`, `battle_fine_equipment.py`) et V2 (`battle_skinned*.py`).
  Vérification sur les données cuites : `battle_fine_check.py`. Ne pas retoucher à la main.
- **Sources** :
  - corps, visages (cibles MPFB) et poids : MakeHuman via MPFB 2.0.17, **CC0 1.0**
    (`game/assets/third_party/characters/makehuman_base/`, voir son `SOURCE.md`) ;
  - squelette, animations et cheval : Quaternius, **CC0 1.0** (voir `battle_skinned/SOURCE.md`).
- **Modifications** : rig `human` et os `R:` du rig `cavalry` aux proportions réalistes
  (humérus 0,25 m, avant-bras 0,26 m, tronc +5 %, épaules élargies ; mêmes os, mêmes clips) ;
  corps ajusté membres joint à joint, 8 visages, cheveux courts et barbes, vêtements en coques
  du corps, équipement V2 reconstruit sur le corps, kit FG0 pour `infantry_0` et `cavalry_0`.
- **Licence du dérivé** : CC0 1.0.
