# CC C2 — relief composé

Branche `cc/c2` (worktree `game_project-cc2`). Contrat : `docs/wip/cc-carte-composee.md`.
Réglages : `data/map/compose/relief.json` (schéma `compose_relief`). Module : `tools/cent_ans_tools/compose/relief.py`.
Commande : `cent-ans geo compose-relief`.

## Plan
1. Base : plaines plates + rampe côtière, zones de collines/plateaux (relief réel déformé très lissé).
2. Massifs et collines bâtis sur `ridges.json` (profil transversal, bruit ridged le long de la crête, contreforts).
3. Cols (24, dont Arlberg et Perthus rattachés), entailles nettes.
4. Plateformes sous les 400 lieux.
5. Fleuves au fond (lit monotone), vallées ; côtes cohérentes avec `land_mask.png`.
6. Sorties : heightmap.png, heightmap_render.png, tuiles height/ (28x24 de 512, canevas complété), relief_shade ; map.json.
7. Tests `tools/tests/test_compose_relief.py`, planche hors dépôt.

## État
- Squelette : config + schéma posés.

## Prochaine étape
- relief.py.
