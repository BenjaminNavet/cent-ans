# CR1 — artefacts des gros plans de bataille

État : CR1 terminé (6 artefacts corrigés), grille avant/après `docs/img/cr/cr1_ab.jpg` (captures brutes `~/dev/cent-ans-raw/cr1/`).
Prochaine étape : jugement du joueur ; points ouverts plus bas. Références : `~/dev/cent-ans-raw/cav/c1_mounted.jpg`, `c2_close4.jpg`.
Prochaine étape : bissection par options `--no-*`, identification du code fautif pour les 6 artefacts
(rectangles saumon, fantômes blancs, cubes noirs, halos blancs, disques de lance, eau éblouissante).

| # | Artefact | Cause | Correctif | Commit |
|---|---|---|---|---|
| 1 | Rectangles saumon flottants à hauteur d'épaule | Rubans des ordres donnés (`BattlePathPreview`, 1,1 m, levés de 0,6 m, sans test de profondeur) et arcs de tir, jamais masqués en gros plan ni par `--no-hud` | Masqués sous 30 m de caméra (`ORDERS_NEAR_HIDE_M`, l'aperçu en direct reste) ; `--no-hud` cache aussi contours, trajets, arcs | 3dfbabb01 |
| 1b | Taches rouges au sol, caparaçons rougis | Décale de contour ennemi (boîte 30 m, `normal_fade` 0) projetée sur tout, figurines comprises | `cull_mask` = `BattleTerrain.DECAL_LAYER` (sol, rivière, ruisseaux, herbe) | 3dfbabb01 |
| 2 | Soldats « fantômes » blancs, bande blanche sur le pont | Même cause : trait or pâle des décales de contour/fantôme peint verticalement sur figurines et piles | Idem 1b | 3dfbabb01 |
| 3 | Cubes noirs en l'air | Mottes BV1 : dégradé carré 12-24 cm, découpe nette, noires à contre-jour, lancées à 6,5 m/s, aussi sur le tablier des ponts | Silhouette bosselée, 4-10 cm, rétroéclairage, fondu tramé, 2-4,5 m/s, jamais sur un pont | 3dfbabb01, 7abcf6bd4, 0c02393d0 |
| 4 | Halos blancs le long du parapet | Surtout des gerbes d'eau (disque blanc sans éclairage) émises pour les régiments sur le pont (emprise au-dessus de la rivière) ; en plus poussière en disque clair et fumée de camp proche | Pas de gerbes au-dessus de 2,5 m du lit (`_on_bridge`) ; poussière bruitée plus brune (alpha 0,32, fondu au contact et près de la caméra) ; fumée de camp alpha 0,3 et fondu sous 25 m (`near_fade` de `fire_smoke`) | 0c02393d0 |
| 5 | Grands disques blancs sur les lances | Blancheur : décale de contour (cf. 2). Taille : rondelle de 20 cm | Rondelle 14 cm ; recuisson cavalry_0,1,3,4,6 (+ atlas partagés) | 414a7eb3a, 0c02393d0 |
| 6 | Eau blanche éblouissante | `battle_water` : ciel compté deux fois (Fresnel maison dans EMISSION + spéculaire moteur à rugosité 0,04 sur vagues bosselées), ciel d'horizon clair jusqu'à 72 % | Spéculaire 0,45 → 0,25, rugosité 0,1, ciel reflété ×0,7 plafonné à 40 % | 414a7eb3a |

Tests : smoke, fg3_maps_test, sr2_weathering_test, cb_m1, cb_m2, cb_m4 verts.
Échecs préexistants (hors CR1) : `cb_m3_queue_test` (« last queued point where clicked », échoue aussi sur ddb7bd1cf),
`bv1_check` (« traits dans les pavois : 0/256 », `BattleVolleys`, non touché).

Points ouverts :
- Rebord blanc au pied des piles du pont (écume EP3 ou dessus des avant-becs) : non traité.
- Les mottes restent visibles en rondelles brunes à 2-3 m : acceptable, à juger.
- Eau : `EMISSION` ne suit pas la lumière du jour (au crépuscule la rivière reste claire) ; non traité.

# CR2 — acier crédible (plates, casques)

État : démarré 2026-09-30 (agent visuel, worktree `game_project-sr`, branche `feat/sr`). Captures brutes `~/dev/cent-ans-raw/cr2/`.
Prochaine étape : captures de référence, bissection du chemin C_PLATE (`battle_soldier_skinned.gdshader`).

# CR3 — caparaçons en drap lourd

État : pas commencé.
Prochaine étape : lire `tools/blender_scripts/battle_fine_horse.py` (`caparison`), plan drapé/ourlet/fentes.
