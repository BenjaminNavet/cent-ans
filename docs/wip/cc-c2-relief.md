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

## État (14/10) : C2 TERMINÉ
- `cent-ans geo compose-relief` (≈ 3 min ; `--no-write` = planche seule). Réglages `data/map/compose/relief.json` (schéma `compose_relief`).
- Écrit : `heightmap.png`, `heightmap_render.png` (= moyenne 2x2 de la finesse ; `render_heightmap.boost` = null, profil RE neutralisé), `height/` (28 x 24 tuiles 512, canevas complété 7168 x 6144 -> 14336 x 12288), `relief_shade_bc5_*.bin` (+ `map.json.relief_shade.bc5`), `map.json` (`height_tiles`, `render_heightmap`, `sources.dem`).
- Rattache Arlberg et Perthus (`ridges.json`, `attach_free_px` 140) ; 24 cols creusés une seule fois (crête la plus proche).
- Tests : `tools/tests/test_compose_relief.py` (15). Planche : `~/dev/cent-ans-raw/compose/c2/planche_c2.png`, `planche_c2_alpes.png`.

## Reste pour C4 / autres
- Navgrid : la règle `massif_impassable` se calcule telle quelle sur ce relief (test) ; régénérer : `cent-ans geo navgrid` (après provinces/lieux C3).
- Neige/roche : pas de couche ; à dériver de l'altitude (> ~2500 m) dans colormap/splat (C4).
- `geo/horizon.py` lit `height_tiles` comme relief réel : à repointer (C4/C5). Pyramide `relief_pyramid`, `relief_occlusion.png` : périmées, à couper ou régénérer.
- Tests GDScript à revoir (altitudes) : `zg8_relief_test.gd`, `sz1_mountain_test.gd` (non lancés). Pas de test tools zg8/sz1.
- Chaînes minces : seules 29 crêtes `massif` (Alpes, Pyrénées, Caucase, Atlas, Balkans) ; Carpates/Massif central en collines (<= 950 m). Ajustable dans `ridges.json` (éditeur CE).

## Révision (14/10, retour coordinateur)
- Socle commun des chaînes (`relief.json` `socle`) : union épaisse des massifs + fermeture + flou, sommets ridged à 3 échelles ; Alpes d'un seul bloc.
- `compose/ridges_author.py` (appliqué par `compose-relief`, idempotent) : 14 chaînes `massif` d'auteur (Pyrénées continues, Apennins, Dinarides, Balkans, Rhodope, Carpates en arc, Scandes, Massif central, Taurus, Pontiques, Atlas, cordillères ibériques) ; collines en boucles/courtes/tortueuses retirées (193 -> peu), les zones restent des bosses douces.
- Cols en U (largeur ≈ 3x, fond quasi plat, fondu aux extrémités), plancher <= 950 m.
- Hauteurs modérées (Apennins 1900, Dinarides 2000, Carpates 2000, Massif central 1750) : lents mais passables ; le mur strict Hongrie/Pologne exige des cols des Carpates dans `crossings.json` (C4/C6).
