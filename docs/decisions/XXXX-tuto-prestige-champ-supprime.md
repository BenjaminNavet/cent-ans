# XXXX — TUTO : le champ `prestige` des factions est supprimé

Statut : accepté (29/10/2026). Numéro à fixer à la fusion.

## Contexte
Les 167 fichiers `data/factions/*.json` portaient un `prestige` (20 à 90, France 90). Aucun code ne le lisait :
le prestige affiché au HUD est celui du souverain (`Character::prestige`, 0 au départ pour tous), gagné en jeu
(étendards, rançons, mariages, événements) et lu par les capitaines, la chevalerie, les rançons, les actes royaux
et les objectifs de victoire `prestige`. Le HUD affichait donc 0 alors que les données annonçaient 90 (constat du juge).

Trois voies :
1. Afficher le prestige initial sans effet sur l'IA : impossible proprement, l'IA lit le prestige du souverain ;
   il faudrait un second compteur (prestige « de façade ») qui mentirait sur ce qui pèse vraiment.
2. Brancher le champ sur le souverain (essayé au lot TEXTES) : le prestige seuille les capitaines, les rançons et les
   actes ; le test `recruits_join_a_small_army...` casse. Un effet calibré demande une passe d'équilibrage (seed bots longue,
   par faction) que ce lot ne peut pas porter, et des valeurs 20-90 posées « au jugé » n'ont jamais été calibrées.
3. Supprimer le champ.

## Décision
Voie 3. Le champ `prestige` est retiré du schéma `faction.schema.json`, de `Faction` (`data-model`) et des 167 fichiers.
Le prestige reste une quantité du souverain, gagnée en jeu. Si l'on veut un prestige de départ différencié, on le
posera sur le personnage (`data/characters`), là où il est lu, avec son calibrage, dans un chantier d'équilibrage dédié.

## Conséquences
- Plus de donnée morte qui contredit le HUD ; comportement de l'IA et des seeds inchangé (le champ n'était pas lu).
- La valeur historique (statut relatif des maisons) est perdue des fichiers factions ; elle reste dans l'historique git.
