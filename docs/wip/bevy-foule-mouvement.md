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
