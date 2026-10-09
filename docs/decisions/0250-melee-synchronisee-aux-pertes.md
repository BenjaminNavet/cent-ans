# 0250 — Mêlée des figurines synchronisée sur les pertes résolues par le cœur

Statut : accepté (2026-10-09)

## Contexte
La revue RX (animation) relevait que la mêlée est une loterie de clips (mode CYCLE, hachage) sans
lien avec les coups du cœur : un recul `hit` jouait sur un soldat qui ne reçoit rien, et une
perte du régiment n'avait aucun écho visible.

## Décision
- Le cœur et le pont ne changent pas (le rendu lit seulement). L'événement d'impact est la perte
  de figurines déjà détectée par `BattleSoldiers._spawn_corpses` (le même qui fait tomber les
  cadavres), avec un seuil minimal de `melee_hit.window_s` entre deux événements.
- À chaque événement, le régiment reçoit `hit_time` (son horloge propre, `_lag` comprise) ; le
  shader fait jouer à une part `hit_share` des soldats (tirage stable par événement, décalage
  par soldat jusqu'à 0,25 s) un clip du jeu `hit` pendant sa durée, puis reprend le cycle.
- Les clips `hit*` quittent le tirage de base de la mêlée (jeux `STYLES.*.melee`), le jeu `hit` est
  une entrée de plus (`"hit": [...]`). Cavalerie : `c_rear`, part `cavalry_share` plus faible.
- Cycle de mêlée = durée du plus long clip non bouclé du jeu (plus de coup coupé, plus de pose
  finale tenue), calculé dans `state_config`.
- Réglages dans `data/fx/battle_animation.json` (`melee_hit`), schéma mis à jour.

## Conséquences
- Pas de synchronisation individuelle (qui frappe qui) : le cœur n'expose pas d'attaquant par
  soldat ; le coup porté par l'attaquant n'est pas encore lié aux pertes de l'adversaire (reste
  possible : `strike_time` du tueur via `loss_by`).
- Aucun changement de simulation ni de reproductibilité.
