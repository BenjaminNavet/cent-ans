# CR1 — artefacts des gros plans de bataille

État : 1-4 et 6 corrigés en code ; rondelles (5) : recette réduite, recuisson cavalry_0,1,3,4,6 en cours. Prochaine étape : tests, captures après, grille avant/après. Références : `~/dev/cent-ans-raw/cav/c1_mounted.jpg`, `c2_close4.jpg`.
Prochaine étape : bissection par options `--no-*`, identification du code fautif pour les 6 artefacts
(rectangles saumon, fantômes blancs, cubes noirs, halos blancs, disques de lance, eau éblouissante).

| # | Artefact | Cause | Correctif | Commit |
|---|---|---|---|---|
| 1 | Rectangles saumon flottants | Rubans d'ordres (`BattlePathPreview`, sans test de profondeur, levés de 0,6 m) + arcs de tir, non masqués par `--no-hud` ni en gros plan | Masqués sous 30 m de caméra et par `--no-hud` | |
| 1b/2 | Taches rouges au sol / caparaçons rougis / soldats « fantômes » blancs / bande blanche sur le pont | Décales de contour et fantôme d'arrivée (boîte 30 m, `normal_fade` 0) projetées sur figurines et murs | `cull_mask` = `BattleTerrain.DECAL_LAYER` (sol, eau, herbe) | |
| 3 | Cubes noirs | Mottes BV1 : billboard à dégradé carré 12-24 cm, découpe nette, noir à contre-jour | Silhouette bosselée, 5-12 cm, rétroéclairage, fondu tramé | |
| 4 | Halos blancs | Poussière : disque radial opaque au centre, teinte claire | Nuage bruité, teinte plus brune, alpha 0,45, fondu au contact et près de la caméra | |
| 6 | Eau blanche éblouissante | `battle_water` : ciel reflété deux fois (Fresnel maison dans EMISSION + spéculaire du moteur, rugosité 0,04 sur vagues bosselées → traînées blanches), ciel d'horizon clair jusqu'à 72 % | Spéculaire 0,45 → 0,25, rugosité 0,04 → 0,1, ciel reflété ×0,7 plafonné à 40 % | |
| 5 | Grands disques blancs sur les lances | Rondelle de 20 cm (rayon 0,1) ; la blancheur venait surtout de la décale de contour (cf. 2) | Rayon 0,07 (14 cm), recuisson des 5 recettes à lance | |
