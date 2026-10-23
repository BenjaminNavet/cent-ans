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
