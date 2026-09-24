# C2b — colonies des îles Britanniques, Pays-Bas, Empire, Scandinavie

Tâche : `docs/design/2026-09-24-echelle-colonies.md` § 3.1. 56 provinces (régions
`angleterre_*`, `galles`, `ecosse`, `irlande`, `pays_bas`, `empire_*`, `scandinavie`).

## État : terminé

56/56 provinces écrites, 227 colonies (56 city + 89 town + 38 castle + 34 abbey +
10 village). Validation schéma + règles (une city par province, 3-6 colonies, ids
uniques, bâtiments/factions existants, coastal ⇒ ≥1 port, province == nom de fichier)
: OK sur les 56 fichiers.

Enclaves (`owner` non nul) : `set_stirling` (prov_fife) → `fac_england` (place forte
anglaise depuis la campagne d'Édouard III de 1336) ; `set_dunbar` (prov_lothian) →
`fac_scotland` (tenu par le comte de March pour la cause de Bruce, en zone sinon
anglaise).

## Prochaine étape

Tâche C2b terminée. Voir `docs/wip/colonies.md` pour la suite de l'orchestration
(C2c Sud, C3 pipeline géo, C4 refonte cœur).
