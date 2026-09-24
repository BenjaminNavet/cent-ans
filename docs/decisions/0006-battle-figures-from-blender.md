# 0006 — Figurines de bataille modelées sous Blender, animées par shader

Date : 2026-09-24 (lot B1)

## Contexte
Les figurines des batailles (V4) étaient construites à l'exécution par `SurfaceTool` à partir de
cylindres et d'ellipsoïdes : proportions de mannequin, chevaux informes. Le rendu doit se
rapprocher d'un Total War moderne sans changer le pipeline d'animation (MultiMesh par régiment,
membres rigides tournés par `battle_soldier.gdshader`, pas de squelette).

## Décision
- Les fantassins, archers et cavaliers sont modelés par un script Blender reproductible
  (`tools/blender/battle_figures.py`, lancé en `--background`), à base de « lofts » (sections
  super-elliptiques le long d'un chemin) lissés par une subdivision Catmull-Clark. Pas de fichier
  `.blend` : le script est la source, les `.glb` exportés sous `game/assets/models/battle/` sont
  des artefacts versionnés avec leur `figures.json` (pivots des membres, des genoux et jarrets).
- Le glb porte le format du shader dans des attributs standard (COLOR_0 = couleur linéaire et
  code matière, TEXCOORD_0 = UV du blason, TEXCOORD_1 = membre et poids de flexion) ;
  `BattleMeshes` le convertit au chargement vers CUSTOM0 (membre, pivot, poids) et UV2 (pivot du
  genou), avec cache. Les engins de siège restent procéduraux ; le procédural V4 sert de repli
  et de référence (`--legacy-figures`).
- Trois niveaux de détail : complet (< 32 m), moyen (< 75 m), lointain (au-delà et ombres).

## Conséquences
- Toute retouche de figurine passe par le script Blender puis `godot --import` ; le format des
  sommets reste unique pour le shader (les deux sources le respectent).
- Le shader gagne une articulation (genou / jarret) sans squelette : allures du cheval lisibles
  (pas, galop), mais toujours pas de déformation fine (coudes, torsion du buste).
- Coût : +16 % de primitives à 4 800 soldats (vue lointaine), +44 % en gros plan ; pas d'écart
  de temps d'image mesurable au-delà du bruit de la machine partagée (≈ −5 %).
