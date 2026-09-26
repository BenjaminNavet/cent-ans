# Figurines de bataille fines (lots FG1 à FG4)

Chargées par `BattleSkinned` seulement avec `--fine-figures` après `--` (le rendu par défaut
reste `battle_skinned/` jusqu'au lot FG5). Même format que `battle_skinned/` (voir son
`SOURCE.md` : `CAM1`, `CAB1`, `manifest.json`) ; le manifeste fusionné renomme les rigs
`fine_human` / `fine_cavalry`.

- **Générées par** : `tools/blender_scripts/battle_fine.py` (Blender 5.2,
  `blender -b --factory-startup --python tools/blender_scripts/battle_fine.py -- all`), avec
  `battle_fine_rig.py` (proportions du rig, pose de l'arc), `battle_fine_figures.py`
  (recettes : corps, vêtements, visages, niveaux de détail), `battle_fine_gear.py` et
  `battle_fine_weapons.py` (lot FG2 : casques, armures, armes, écus) et les modules FG0
  (`battle_fine_proto.py`, `battle_fine_equipment.py`) et V2 (`battle_skinned*.py`).
  Vérification sur les données cuites : `battle_fine_check.py`. Ne pas retoucher à la main.
- **Sources** :
  - corps, visages (cibles MPFB) et poids : MakeHuman via MPFB 2.0.17, **CC0 1.0**
    (`game/assets/third_party/characters/makehuman_base/`, voir son `SOURCE.md`) ;
  - squelette et animations : Quaternius, **CC0 1.0** (voir `battle_skinned/SOURCE.md`) ;
  - cheval des montés (lot FG4) : « Rigged Horse » de Lyndon Daniels (OpenGameArt), **CC0 1.0**
    (`game/assets/third_party/animals/oga_rigged_horse/SOURCE.md`), ajusté aux os du cheval
    Quaternius par `battle_fine_horse.py`, harnachement et bardes par `battle_fine_cavalry.py`.
- **Modifications** : rig `human` et os `R:` du rig `cavalry` aux proportions réalistes
  (humérus 0,25 m, avant-bras 0,26 m, tronc +5 %, épaules élargies ; mêmes os, mêmes clips) ;
  corps ajusté membres joint à joint, 8 visages, cheveux courts et barbes, vêtements en coques
  du corps ; équipement fin par script (FG2 : casques, plates des membres, jaque, brigandine,
  surcots plissés, armes et écus d'époque, UV par pièce, faces cachées supprimées).
- **Textures (lot FG3, `textures/`)** : toutes produites par script, aucune image tierce
  hormis le pelage CC0 du cheval ; ADR 0088.
  - `fine_atlas_lod0.png` (512² × 28) et `fine_atlas_lod1.png` (256² × 28) : atlas par
    figurine (bandes verticales lues en `Texture2DArray`, couche = `atlas_layer` du manifeste),
    **cuits** dans Blender/Cycles par `battle_fine_bake.py` (`battle_fine.py -- bake`) : RG
    normale de forme (pièces avant décimation, plis subdivisés des étoffes), B occlusion
    (par groupe de variantes), A masque (barbe/cheveux, sourcils, martelage des plates) ;
  - `fine_detail.png` (512² × 8) : tuiles de détail répétables (mailles, tissage, feutre,
    cuir, acier martelé, bois, peau, cheveux), **calculées** en numpy par
    `battle_fine_tiles.py` (champs de hauteur procéduraux, aucune photo) ;
  - `fine_horse.png` (1024²) : normale, relief de la robe et occlusion du pelage réduits depuis
    les textures 2k de « Rigged Horse » (Lyndon Daniels, OpenGameArt, **CC0 1.0**, voir
    `game/assets/third_party/animals/oga_rigged_horse/SOURCE.md`) ;
  - les `.import` (BC7, Texture2DArray) sont écrits par le script ; ne pas retoucher à la main.
  - Maillages `CAM2` (LOD0/LOD1 des figurines cuites) : UV d'atlas empaquetée dans `UV2.y`.
- **Licence du dérivé** : CC0 1.0.
