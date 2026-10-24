# Lot BATAILLE — mise en scène (wip)
## Fait (commits wip sur la branche du worktree)
- (3) Compteur « Nuit dans » : `BattleSim::deadline_s` (core), ADR 0772, tests.
- (1) Siège : cadrage d'ouverture derrière l'assaillant, 170 m, pivot entre armée et porte (`terrain.rs::siege_opening_frame`) ; suivi de l'engagement (`camera.rs::follow_engagement`, après 5 s sans entrée, dérive 0,7/s vers le noyau de mêlée le plus dense). Désactivé avec `CENT_ANS_BATTLE_CAM`.
- (2) Lisibilité : traînée de flèches large en fonction de la distance (0,25 %/côté, plafond 0,7 m), alpha 0,7, longueur 7 m ; mesh de flèche x2,8 ; poussière entretenue en mêlée ; étincelles d'impact x1,7 (data/fx/battle_impact.json).
- (5) Cartes : compactes (58 px) non sélectionnées, 90 px sélectionnées (`cards.rs::card_height`).
- (6) Premier plan : 24 touffes de buissons derrière chaque armée (`trees.rs::plan_foreground`), rendu seul, sauf siège.
## Reste
- (4) unit_size/max_figures non changés (bench à charge 45 : non fiable, mêlée à 59 fps sans marge) ; imposteurs animés non faits.
- Ciel/exposition : hors périmètre (lighting.rs interdit).
## Mesures siege (captures out/shots)
- pivot 0.55 / 170 m : armee assaillante visible, pas de toits, remparts hors cadre.
- pivot 0.65-0.8 / 200-240 m : face a la tour de porte, trop pres. A regler. deploy.rs utilise le cadrage de siege meme sans regiments poses (le joueur de la demo est defenseur).
## Fin
- Siege: pivot 0.42, 210 m, tangage 20 : armee et remparts visibles (valide par capture). Les cinematiques du Capitaine deplacent la camera : capturer apres Echap.
- Bench release unique (load 19-22, bruite) : calme 75,05, melee 55,4 (ref 75/59). Marge nulle : A/B a refaire par integrateur en machine calme (suspects : poussiere de melee, etincelles x1,7).
- unit_size/max_figures inchanges : 1.0 = 1 figurine par homme, le plafond 15000 ne mord que si unit_size > 1 ; monter coute des fps sans marge.
