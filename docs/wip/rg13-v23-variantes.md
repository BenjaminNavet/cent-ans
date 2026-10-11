# RG13-V23 — variantes de rendu de la salle du conseil

Comparaison de trois rendus des conseillers dans la même salle (les modèles image→3D sont jugés laids).

## Changer de style
- Défaut du jeu : clé racine `figure_style` de `data/art/council_catalog.json` (`mesh` | `mesh_multiview` | `painted`).
- Surcharge tests/captures : argument utilisateur `--council-style=<v>` (après `--`), ou `CouncilRoom.style_override`.
- Capture : `tools/godot_bg.sh --path game --script res://tests/council_shot.gd -- --council-style=painted`
  écrit `user://council_shot_<style effectif>.png`.

## Styles
- `mesh` : `game/assets/models/council/council_<x>.glb` (actuel).
- `mesh_multiview` : mêmes noms sous `.../council/multiview/` (autre agent) ; absent -> repli `mesh`, un seul avertissement.
  Un fichier manquant isolé retombe sur le modèle `mesh` correspondant.
- `painted` : quads billboard Y (alpha scissor) `game/assets/illustrations/council/council_<x>.png`
  (`tools/experiments/council_painted_prep.py`, rognés à la boîte alpha, hauteur 1,30 m, pieds au sol, ombre radiale),
  mobilier fait main `.../council/handmade/council_{table,throne,stool}.glb` (`tools/experiments/council_furniture.py`,
  `blender -b --python ...`). Survol = éclaircissement du quad ; clic = mêmes zones que les figures 3D.

## État
Code, assets, tests écrits. Reste : coup d'oeil du joueur (captures par la session principale).
