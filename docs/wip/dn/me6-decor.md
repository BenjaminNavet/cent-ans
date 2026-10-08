# DN ME6+ME7+ME9 — décor ponctuel hors villes

Branche `dn/me6-decor` (worktree `../gp-dn-me6-decor`). Plan : `docs/wip/dn/carte-extra.md` §4. Doc : `docs/godot-map.md` (« Décor ponctuel hors les villes »).

## État : terminé, à fusionner par l'orchestrateur
- Données : `data/map/map_landmarks_extra.json` (50 types, 30 règles, 86 sites réels) + schéma + `tools/cent_ans_tools/decor_bake.py` (lonlat -> px, étapes de route -> colonies) + `tools/tests/test_decor_bake.py`.
- `DecorPlanner` (placement pur, 8 200 instances, ~0,4 s hors fil principal) et `DecorLayer` (MultiMesh par type, taille tenue à l'écran, période/saison/brouillard, fumée) branchés dans `SettlementLayer` (`decor`).
- Test : `game/tests/me6_decor_test.gd`. Smoke, tb3_growth_test, pytest schémas verts.

## Reste / points ouverts
- Aucun glb DN encore ingéré (manifeste vide) : volumes procéduraux de repli (`shape`/`color`/`size_m`). Dès que `dn-ingest` inscrit un `env_*`, il remplace le repli sans autre changement ; vérifier alors pivot au pied et `size_m` (largeur de référence du facteur d'écran).
- Pas de capture ni de banc de frame (règle : orchestrateur) ; coût estimé : 8-11 nœuds en zoom moyen, rebuild < 3 ms, aucune instance au-delà de la portée des types.
- Lieux hors carte : Skellig Michael (île trop petite pour 719 m/px) est ignoré par le recalage (7 px).
- Équilibrage des densités (`spacing_px`, `chance`) à juger à l'oeil ; `--no-me6` pour l'A/B.
- Un `queue_free` du monde de test se bloque (même sans la couche, avec `--no-me6`) : sans rapport, le test quitte sans libérer.
