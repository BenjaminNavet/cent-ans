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
  (fal épuisé le 09/10 : « Exhausted balance » ; TRELLIS HF : quota ZeroGPU épuisé → SF3D local).
- Décimation : `blender -b --factory-startup --python tools/blender_scripts/dn_forest_patch_lod.py -- IN.glb OUT.glb 3000`
  (le banc prend `3d/lod_*.glb` d'abord ; brut 59 k triangles × 10 k massifs = GPU décroché, « timeout waiting for fence »).
- Captures : `tools/godot_bg.sh --path game --resolution 960x600 --script res://tests/dn_forest_patch_shot.gd -- --out=<dossier>`
  (Orléans feuillus, Vosges conifères, Maures méditerranéen ; d = 20, 40, 80 ; `_trees` / `_patch`).

## État
- [x] squelette (générateur + banc de capture)
- [x] 3 massifs générés (Z-Image local + SF3D, 0 $), galerie 8765 (`forest_*`)
- [x] captures A/B : galerie 8765 → Chantiers → DN-forets (18 vues)
- [ ] décision du joueur

## Verdict de l'essai (09/10)
- Pour : vrai volume bosselé de près (d 20), une seule instance par bloc (455 à 6 700 massifs selon d,
  contre 10-20 k arbres par partie de tuile).
- Contre : texture SF3D grise et délavée (pas de teinte de feuillage, pas de saisons, pas de
  peuplements ADR 0221) ; bloc rigide aux lisières franches ; répétition visible dès d 40 avec un seul
  modèle ; déborde sur les villes (pas de dégagement `TreeClearance`) ; posé à plat sur les pentes.
- Piste si on poursuit : massif comme **couche lointaine** (d ≥ 60) à la place des arbres généralisés,
  4-6 variantes par peuplement en TRELLIS (fal rechargé), teinte du shader de feuillage, dégagements
  réutilisés ; garder les arbres DN-FORET de près.

## Incidents
- Classes fusionnées par d'autres sessions (DecorHover, champs) non importées : `settlement_layer` nul,
  boucle d'erreurs 40 min → `godot --headless --path game --import` avant tout banc.
