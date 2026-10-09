# 0231 — Carte : suppression de replis morts

Statut : accepté (10-09, lot SC MB10/MB11/MB13)

## Contexte
Deux replis de rendu de la carte ne servaient plus : la déduction des champs de bataille depuis les
événements `battle` (le pont fournit `get_battle_history`, ADR 0157), et la végétation procédurale
sans splatmap (la pyramide de relief et la splat sont obligatoires, ADR 0203).

## Décision
- `war_scars.gd` : seule voie l'historique des batailles du cœur ; sans `get_battle_history`, aucune
  marque. Le test `tb4_scars_test.gd` alimente une simulation factice par `get_battle_history`.
- `vegetation_mask.gd` : `sample` lit toujours la splat ; tables procédurales par terrain retirées.
- Une trentaine de fonctions de `game/scripts/map` sans aucun appelant supprimées.

## Conséquences
Rendu inchangé avec les données actuelles. Une carte sans `splat.png` donne des densités nulles
(elle est déjà refusée à la construction du terrain).
