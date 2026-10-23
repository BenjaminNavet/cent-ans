# 0773 — Mise en scène des batailles : cadrage de siège, suivi, cartes compactes, premier plan

## Contexte
Juge iter-03 : le siège s'ouvre dans les toits, la mêlée se joue en bord d'écran, les cartes d'unité forment une frise lourde, la moitié basse de l'écran est une prairie vide.

## Décision
- Ouverture de siège (`terrain.rs::siege_opening_frame`) : toujours depuis l'assaillant, pivot entre son armée et la porte, vers la muraille ; distance propre au siège (`opening_distance`).
- Suivi de l'engagement (`camera.rs::follow_engagement`) : après 5 s sans entrée, le pivot dérive vers le noyau de mêlée le plus dense. Coupé par `CENT_ANS_BATTLE_CAM`.
- Cartes : 58 px non sélectionnées, 90 px sélectionnées.
- Premier plan : touffes de buissons derrière chaque armée (rendu seul, aucun obstacle, pas en siège).
- Projectiles : traînées larges avec la distance, mêlée poussiéreuse, étincelles plus grandes.

## Conséquences
Rendu seulement, aucune règle modifiée (la règle de durée est dans l'ADR 0772).
