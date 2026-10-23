# foule-mouvement : contrôle visuel de la foule en mouvement (optimisations actives vs CENT_ANS_CROWD=off)

## État (PAUSE GPU demandée par la session principale, aucun lancement client en cours)
- Client debug construit dans le worktree (CARGO_INCREMENTAL=0, features dev).
- Série 1 (bataille perso, `CENT_ANS_ORDERS_DEMO=1 CENT_ANS_NO_DEPLOY=1 CENT_ANS_FIXED_DT_MS=16.7 --state Battle --seed 2`, 40 captures tous les 20 images) faite ON et OFF : pas de contact atteint (marche/tir seulement). Différence moyenne 0,04 à 0,22 sur les images alignées ; ~11 sur d'autres images : cause = éclairage global différent (battle_lighting, toute la scène plus chaude/claire), pas la foule. Cartes de diff : n'éclairent que les lignes de soldats, faiblement.
- Mêlée : Crécy défenseur via menus (script type tools/bevy_bench/battle_open.txt) + `click speed:4` : charge vers l'image ~8250, mêlée ~8300-8600 (elapsed ~545 s).
- Série 2 « m_on » (Crécy, 50 captures tous les 15 images à partir de l'image 8000) faite ON.

## Reste à lancer
1. m_off : même script avec `CENT_ANS_CROWD=off`.
2. Comparer m_on / m_off (scripts cmp.py, best.py, trip.py dans le scratchpad), lire <= 8 images.
3. Éventuellement caméra tournante (CENT_ANS_BATTLE_CAM yaw) ; correctif seulement si défaut visible.

## Résultat final (Crécy, dt fixe 16,7 ms, x4, images 8000-8735, mêlée vers 8300+)
- ON vs OFF, images au même éclairage : différence moyenne 0,01-0,02 avant contact ; en mêlée 2-3 % des pixels (>40/255) différents, soldats proches du bord bas-gauche manquants (lances/soldats) et ombres associées.
- Cause : le culling par soldat (CULL=0 avec habillage d'ombre actif revient à 0,17 de moyenne et 0,1 % de pixels : identique à OFF). L'habillage allégé des ombres est donc hors de cause.
- Cause probable : sphère de culling centrée au pied (rayon 2,4 m) alors que les soldats avec lance font 5 m+ : un soldat dont le pied est sous le bord de l'écran disparaît. Essai : rayons 4,5 / 6,5 m (data/fx/battle_crowd.json) : 1-1,8 % de pixels au lieu de 2-3 %, amélioration partielle, pas résolu. Non itéré (GPU repris par l'autre session).
- Éclairage global qui diffère d'un processus à l'autre (images ~11 de diff moyenne) : non élucidé.
- Non vérifié : caméra tournante, 0,25 s après changement de clip isolé, bataille 300 images après la modif data.
