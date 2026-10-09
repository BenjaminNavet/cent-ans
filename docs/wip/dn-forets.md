# DN-forets — massifs générés d'un seul tenant au lieu d'arbres individuels (essai)

Question du joueur (09/10) : « n'est-il pas mieux de générer une forêt entière plutôt que des arbres
individuels ? » → essai comparatif en jeu.

## Contexte
- Le jeu affiche les forêts en arbres **généralisés** (HC1, ADR 0161, `map.tree_style = generalised`) :
  un arbre ≈ un bois, hauteur monde 0,8 unité (≈ 575 m), plancher caméra 20 (`camera_floor_distance`).
- Essai : un glb = un bloc de forêt (image Z-Image « bloc de forêt vu de 45° » → 3D), semé sur le masque
  forestier (`VegetationMask.sample`) au pas de sa largeur, comparé A/B aux arbres actuels.

## Outils
- Génération : `tools/experiments/dn_forest_patch.py [--local]` → `~/dev/cent-ans-raw/dn/forets/<massif>/`
  (fal épuisé le 09/10 : « Exhausted balance » → chaîne gratuite mflux + TRELLIS HF / SF3D).
- Captures : `tools/godot_bg.sh --path game --resolution 960x600 --script res://tests/dn_forest_patch_shot.gd -- --out=<dossier>`
  (Orléans feuillus, Vosges conifères, Maures méditerranéen ; d = 20, 40, 80 ; `_trees` / `_patch`).

## État
- [x] squelette (générateur + banc de capture)
- [ ] 3 massifs générés
- [ ] captures A/B, verdict
