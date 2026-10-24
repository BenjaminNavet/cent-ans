# ADR 0783 — Paliers de comportement de la difficulté de l'IA (lot IA-DOCTRINES)

Date : 2026-10-24. Statut : accepté. Complète l'ADR 0037 (difficulté) et l'ADR 0258 (IA, difficulté et sièges).

## Contexte

La difficulté ne modifiait que des chiffres (revenus, entretien, moral, supériorité exigée, bruit de jugement). Le
juge demande que la difficulté change aussi le *comportement* de l'IA sans retirer les bonus.

## Décision

`data/rules/difficulty.json` (schéma `difficulty_rules.schema.json`) gagne quatre leviers optionnels, neutres par
défaut (le niveau « normal » reste neutre au bit près) :

| Levier | Effet | Facile | Normal | Difficile | Très difficile |
|---|---|---|---|---|---|
| `ai_prudence_percent` | échelle la supériorité exigée avant d'attaquer une armée et le seuil de repli | 80 | 100 | 115 | 130 |
| `ai_anticipation` | l'IA compte les armées ennemies en marche vers une province comme déjà arrivées (destination lue) | 0 | 0 | 1 | 1 |
| `ai_coordination` | les armées de l'IA comptent celles du royaume présentes ou en marche vers la même cible pour juger un siège ou une défense | 0 | 0 | 0 | 1 |
| `ai_vision_provinces` | brume de guerre : l'IA ne voit les armées ennemies qu'à n provinces de ses terres ou de ses armées (0 = tout voir) | 2 | 0 | 0 | 0 |

Les bonus chiffrés sont conservés. La prudence se compose avec la doctrine (`Strategy::engage_factor`,
`retreat_factor`), la brume s'applique aux menaces de la campagne et aux armées visibles de la grille. Le joueur n'est
jamais touché (sa faction est neutre). `difficulty_lines` décrit les paliers dans l'écran de choix.

## Conséquences

- Un niveau « facile » devient une IA qui se bat à la légère et ne voit pas loin, pas seulement une IA plus pauvre.
- Anticipation et coordination ne coûtent presque rien en temps de calcul (une entrée de plus par armée en marche).
- Mesures et seed bots : `docs/wip/ia-doctrines.md`.
