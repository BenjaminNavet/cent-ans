# CR1 — artefacts des gros plans de bataille

État : décales (1, 2), rubans d'ordres (1), mottes (3), poussière (4) corrigés en code ; restent eau (6), rondelles (5), vérif. tests. Références : `~/dev/cent-ans-raw/cav/c1_mounted.jpg`, `c2_close4.jpg`.
Prochaine étape : bissection par options `--no-*`, identification du code fautif pour les 6 artefacts
(rectangles saumon, fantômes blancs, cubes noirs, halos blancs, disques de lance, eau éblouissante).

| # | Artefact | Cause | Correctif | Commit |
|---|---|---|---|---|
| 1 | Rectangles saumon flottants | Rubans d'ordres (`BattlePathPreview`, sans test de profondeur, levés de 0,6 m) + arcs de tir, non masqués par `--no-hud` ni en gros plan | Masqués sous 30 m de caméra et par `--no-hud` | |
| 1b/2 | Taches rouges au sol / caparaçons rougis / soldats « fantômes » blancs / bande blanche sur le pont | Décales de contour et fantôme d'arrivée (boîte 30 m, `normal_fade` 0) projetées sur figurines et murs | `cull_mask` = `BattleTerrain.DECAL_LAYER` (sol, eau, herbe) | |
| 3 | Cubes noirs | Mottes BV1 : billboard à dégradé carré 12-24 cm, découpe nette, noir à contre-jour | Silhouette bosselée, 5-12 cm, rétroéclairage, fondu tramé | |
| 4 | Halos blancs | Poussière : disque radial opaque au centre, teinte claire | Nuage bruité, teinte plus brune, alpha 0,45, fondu au contact et près de la caméra | |
