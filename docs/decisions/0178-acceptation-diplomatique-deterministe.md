# 0178 — Acceptation diplomatique déterministe pour le joueur

## Contexte

L'audit joueur A6 (constat M6) : une proposition affichée « 65 % » a été refusée (revendication de
couronne −45). Le joueur ne peut pas raisonner sur un pourcentage dont le tirage est caché. Les
grands jeux de stratégie (Crusader Kings) donnent un score signé : accepté ou non, sans hasard.

## Décision

- Une proposition **du joueur** est acceptée si et seulement si `score >= ACCEPT_SCORE` (0) et
  qu'aucun article n'est bloqué (`negotiation::propose_treaty`, `TreatyEvaluation::accept`).
  Plus aucun tirage (`answer_roll`) pour le joueur.
- Les propositions **entre IA** gardent le tirage déterministe par (graine, tour, paire) contre le
  pourcentage logistique (`chance_of`) : l'aléa y sert la variété de l'histoire, pas le joueur.
- Le seuil 0 coïncide avec l'ancien « 50 % » : `chance_of(0) = 50`. Le champ `chance` reste dans
  l'API (IA, contre-offres, `ai_min_chance`) mais l'interface ne l'affiche plus.
- L'interface (`diplomacy_panel.gd`) affiche « Accepterait (+12) » ou « Refuserait (−5) » : le
  score signé, avec ses facteurs ligne à ligne (déjà fournis par `explain_treaty`). Le résumé
  (`treaty_explain`) et le message de refus (`propose_treaty`) citent le score et le seuil.

## Conséquences

- Ce qui est affiché est ce qui arrive : un score ≥ 0 est toujours signé, un score < 0 toujours refusé.
- Les contre-offres (`counter_proposal`) visent toujours `chance >= 50`, soit score ≥ 0 : cohérentes.
- Les rares cas où `chance_of` arrondit à 50 pour un score légèrement négatif ne comptent plus
  pour le joueur (le test porte sur le score).
