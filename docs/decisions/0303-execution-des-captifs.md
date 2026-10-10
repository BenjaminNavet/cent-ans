# 0303 — Exécution des captifs (WR captives)

Statut : accepté.

## Contexte
Le lot WH `uicards` (ADR 0280) avait laissé top3 : décider du sort d'un captif pris en bataille. Les captifs existaient déjà (`ransom.rs` : termes, parole, rançon). Il manquait l'issue brutale. Décision du joueur (non rediscutée) : exécuter donne du prestige (terreur) au bourreau, perd la rançon, fait fortement baisser l'opinion de la faction du captif (et de sa dynastie), un peu celle des autres seigneurs.

## Décision
- `Order::ExecuteCaptive { character }` → `ransom::execute_captive` : seul le geôlier peut l'ordonner (`own_prisoner` : refus pour un otage de traité, un non-captif ou un captif d'un autre). La mort passe par `characters::kill` (héritier, succession, suite), puis effets.
- Valeurs dans `data/rules/economy.json` `ransom.execution` (schéma `economy_rules`, `ExecutionRules`) : prestige du souverain du bourreau par rang du captif (15/10/6/3), opinion de la faction du captif −60 sur 40 saisons, autres factions dont le souverain est de la même maison −25 (40 saisons), toutes les autres −8 (20 saisons, déshonneur chevaleresque), trait `trait_cruel` pris par le souverain (existant, aucun trait créé).
- La « dynastie » du modèle est la chaîne `house` : pas de lignage inter-factions plus fin.
- Journal : deux entrées `EventKind::Ransom` (bourreau, faction du captif) plus l'événement de mort.
- IA (`ai_execution_orders`, appelée par `ai_ransom_orders`) : en guerre avec la faction du captif, captif de prestige ≥ `ai_min_prestige` (40 ; 20 si le souverain est cruel), rançon hors d'atteinte (termes `Hold` ou trésor du payeur inférieur à la rançon), puis tirage déterministe (tour, personnage) de `ai_chance_permille` (15 %) par saison.
- Pont : `get_ransoms` ajoute `execution{prestige, victim_opinion, house_opinion, others_opinion, ransom_lost, ruler_trait}` par captif (`ransom::execution_preview`). UI : bouton « Exécuter » à côté de « Libérer sur parole » dans `RansomPanel`, infobulle chiffrée lue de ce dictionnaire, confirmation en deux temps (premier clic arme le bouton rouge, second exécute).

## Conséquences
- Sauvegardes anciennes : aucun champ sérialisé nouveau.
- Les modificateurs d'opinion sont des motifs libres (non plafonnés) : exécuter plusieurs captifs s'additionne.
- Reste : la décision à l'issue de la bataille (fenêtre à 3 boutons du rapport WH) n'est pas faite ; l'exécution se décide depuis le panneau Captifs.
