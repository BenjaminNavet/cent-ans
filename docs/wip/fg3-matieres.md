# FG3 — Matières cuites des figurines fines

Branche : `feat/fg3-materials` (worktree agent). Plan : `docs/wip/fg-figurines-fines.md`.
Précédents : `fg0-prototype.md` (cuisson test), `fg1-corps.md`, `fg2-equipement.md`, `fg4-cheval.md`.

## État
- [x] Squelette : commande `bake`, format `CAM2` (UV d'atlas), variante `FG3_BAKED` du shader,
  chargement des textures dans `BattleSkinned`, `--no-fg3` (A/B)
- [x] Tuiles de détail (`battle_fine_tiles.py`, numpy) : mailles, tissage, feutre, cuir, acier
  martelé, bois, peau, cheveux ; 512² × 8, Texture2DArray BC7 (2,7 Mo)
- [x] Atlas par figurine : LOD0 512², LOD1 256² (bandes Texture2DArray, couche = rang de la
  recette) : normale de forme (sources HD = pièces avant décimation + plis subdivisés), AO par
  groupe de variantes, masque (densité barbe/cheveux, sourcils, martelage)
- [x] Cheval : `fine_horse.png` 1024² (normale CC0, relief de la robe, AO), partagé
- [x] Shader : branches FG3 (atlas + tuiles selon le code), LOD2 sans lecture ; imposteurs
  avec cartes (`fine_distance` levé dans `battle_impostors.gd`)
- [x] Camail : faces tournées vers l'intérieur depuis FG0 (normales bmesh jamais calculées) :
  corrigé dans `battle_fine_equipment.aventail` (et bardes `battle_fine_cavalry`)
- [ ] Cuisson des 28 recettes (en cours), captures, tests, mémoire, banc, ADR

## Choix d'architecture
- UV d'atlas empaquetée dans UV2.y (11 bits u, 11 bits v, 2 bits source : 1 atlas LOD0,
  3 atlas LOD1, 2 cheval) : aucun attribut de sommet en plus ; LOD2 : 0 (pas de lecture).
- Repère tangent reconstruit au fragment (dérivées écran) : pas de tangentes dans le maillage.
- Variante du shader `#define FG3_BAKED` (comme `BV2_CORPSE`) posée par
  `BattleSkinned.setup_material` pour les figurines fines cuites seulement : le rendu par
  défaut est compilé sans une ligne de FG3.
- Îlots d'atlas tirés des UV propres des pièces (MakeHuman, FG2) : la projection intelligente
  des coques décimées donnait ~1 100 éclats (remplissage 10 %).

## Commandes
```
blender -b --factory-startup --python tools/blender_scripts/battle_fine.py -- bake [--only a,b]
godot --headless --path game --import
godot --headless --path game --script res://tests/fg3_maps_test.gd -- --fine-figures
```

## Prochaine étape
Fin de la cuisson complète, captures `docs/img/fg/fg3_*.png`, banc, ADR.
