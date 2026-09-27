# 0088 — Matières cuites des figurines fines (lot FG3)

Date : 2026-09-26. Statut : accepté.

## Contexte

Les figurines de bataille n'ont aucune texture (ADR 0014) : couleur et code matière par
sommet, matières procédurales dans `battle_soldier_skinned.gdshader`. Les figurines fines
(lots FG1, FG2 et FG4, `--fine-figures`) ont 9 à 17 k triangles au LOD0, mais sans cartes
leurs mailles, plis, visages et robes de cheval restent plats de près.

Contraintes :

- un seul shader skinné, partagé avec DA1 (armoiries), BV2 (cadavres), EP5 (étendards),
  EP12 (blessés), les imposteurs et la carte de campagne ;
- rendu par défaut inchangé au pixel près tant que `--fine-figures` n'est pas le défaut ;
- mémoire : un atlas 2048² par recette (28 recettes) coûterait environ 270 Mo (FG0) ;
- les sommets sont déjà lourds (voir les mesures de FG1) : pas d'attribut en plus.

## Décision

1. **Atlas par figurine, petit.** Chaque recette cuite a son atlas 512² au LOD0 et 256² au LOD1,
   rangés en bandes verticales lues comme `Texture2DArray` (couche = `atlas_layer` du
   manifeste). Canaux : RG normale de forme (espace tangent, sources HD = pièces avant
   décimation, plis subdivisés des étoffes), B occlusion (cuite par groupe de bits de variante,
   pour que les casques superposés ne s'ombrent pas entre eux), A masque selon la matière
   (densité de barbe et de cheveux, sourcils, force du martelage des plates). Cuisson Cycles
   par `tools/blender_scripts/battle_fine_bake.py` (`battle_fine.py -- bake`). Îlots tirés
   des UV propres des pièces (MakeHuman, FG2), hampes longues plafonnées, têtes ×1,8.
2. **Tuiles de détail partagées.** Huit tuiles répétables 512² (mailles, tissage, feutre, cuir,
   acier martelé, bois, peau, cheveux), calculées en numpy (`battle_fine_tiles.py`) : RG
   normale, B relief (modulation de l'albédo), A rugosité. Posées en projection sur la position
   de repos, à une taille en mètres par matière (`FG3_TILE_SIZE`).
3. **Cheval.** Une carte 1024² partagée par toutes les montures (normale, relief de la robe,
   occlusion), réduite depuis les textures CC0 du « Rigged Horse ».
4. **UV d'atlas empaquetée dans `UV2.y`.** Format de maillage `CAM2` = `CAM1` + un flottant
   par sommet : u et v sur 11 bits chacun, 2 bits de source (1 : atlas LOD0, 3 : atlas LOD1,
   2 : cheval). Le flottant tient ces 24 bits exactement. LOD2 reste en `CAM1` (source 0 : aucune
   lecture). Aucune tangente stockée : le repère tangent est reconstruit au fragment à partir
   des dérivées écran de la position et de l'UV.
5. **Variante de shader `FG3_BAKED`.** Comme `BV2_CORPSE` : `BattleSkinned.setup_material`
   bascule le matériau sur la variante (`#define` en tête du code) seulement pour une figurine
   fine qui a un `atlas_layer`, et seulement si le matériau porte déjà le shader skinné (le
   drapeau d'EP5, qui lit la même texture d'os avec son propre shader, garde le sien). Les
   cadavres utilisent la variante `BV2_CORPSE` + `FG3_BAKED`. Au-delà de `fine_distance`
   (80 m), aucune lecture ; les imposteurs cuisent le LOD0 avec ses cartes.
6. **A/B.** `--no-fg3` après `--` : figurines fines sans cartes.

## Conséquences

- Mémoire des cartes (BC7, mipmaps compris, `tests/fg3_maps_test.gd`) : atlas LOD0 9,33 Mo,
  LOD1 2,33 Mo, tuiles 2,67 Mo, cheval 1,33 Mo, soit **15,7 Mo** (budget 60 Mo).
- Rendu par défaut identique : toutes les branches FG3 sont sous `#ifdef FG3_BAKED`.
- Coût GPU : cinq lectures de texture de plus par fragment au LOD0/LOD1 d'une figurine fine ;
  mesures `--units=50` dans `docs/wip/fg3-matieres.md`.
- Écart à la spec FG : atlas par figurine en 512² plutôt qu'un 1024² par famille. Même mémoire
  (quatre figurines = un atlas 1024²), sans UV partagées entre recettes différentes.
- Réglages tranchés en capture : normale de forme ×0,35 et occlusion adoucie sur les plates
  (le bruit de cuisson des pièces lisses ressortait en bosses), martelage léger (0,05 à 0,22),
  mailles plus contrastées et tuile de 10 cm (anneaux ~12 mm).
- Toute nouvelle recette fine doit être recuite (`bake --only <recette>`, environ 1 min à pied,
  plus pour les montés) ; sans atlas, elle retombe sur le rendu procédural.
- Limites : les UV projetées des pièces sans UV propres donnent des coutures visibles de très
  près ; le liseré de la barbe à la lèvre vient de la géométrie.
