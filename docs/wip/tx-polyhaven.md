# Retrait des sols Poly Haven (ADR 0244)

État : code, données, schémas, outils, crédits, ADR faits et commités (branche worktree, pas de
fusion dans main). Reste : vérifications (cargo non touché ; pytest ; smoke.gd ; tests Godot TX).

Fait :
- Bataille : `legacy` retiré, `ground_*_array.jpg`, `near_detail/`, `terrain_tint`, uniformes
  `near_detail_*` du shader (grain TX `micro_battle` couvre le rôle, `micro_dist`).
- Campagne : couches GA4 globales + fichiers `terrain/<couche>_*` + `terrain_*_array.jpg`,
  repli 1k de `TerrainBuilder`, `load_arrays`/`layer_ids`.
- Outils : `geo/textures.py` réduit à la normale de mer ; schémas/tests Python mis à jour.
- Docs : ADR 0244 + INDEX, conséquences 0240/0243, CREDITS.md, READMEs.

Prochaine étape : lancer les tests (voir rapport), corriger, rapport final.
