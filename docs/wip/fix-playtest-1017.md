# Correctifs du playtest du 17/10

Source : `docs/wip/playtest-2026-10-17/SYNTHESE.md`. Mandat joueur (17/10) : corriger tout ce qui a été relevé, via agents.
Un worktree par lot `../gp-fix-<lot>`, branche `fix/<lot>` depuis main 77d21ef8a, note `docs/wip/fix-<lot>.md` sur la branche.
L'orchestrateur relit, fusionne dans un worktree d'intégration puis `--ff-only` dans main. Pas de push.

| Lot | Contenu | Modèle | État |
|---|---|---|---|
| 1 render | terrain noir ultra plein écran, étiquettes sur le menu pause, doublon d'étiquettes | Opus | en cours |
| 2 campui | recrutement, économie, traditions, UI très grande, sélection de faction, « à paris », boutons fermer | Sonnet | en cours |
| 3 battleui | dialogue pré-bataille, écran de résultat, élision, discours, F1, icônes | Sonnet | en cours |
| 4 overlays | bulle du conseiller, file des modales, raccourcis J/C/T, pause | Sonnet | en cours |
| 5 saveload | journal/chronique/féodal/tutoriel perdus au chargement | Opus | en cours |
| 6 battlesim | déploiement, issue du siège, IA passive, fuyards figés, chances navales | Opus | en cours |
| 7 camprules | retraite après défaite, emplacements 9/6, entretien, 213 provinces, eau ×2 | Opus | en cours |
| 8 mapfx | rochers noirs, charrette, mât, échelles, chevaux, pavois | Opus | en cours |
| 9 mapgeo | baie du Mont-Saint-Michel, Léman, villes sur l'eau, rivières, côtes, étiquettes | Opus | en cours |
| 10 perf | campagne < 40 i/s, dérive du temps d'image | — | vague 2, après fusion (machine calme) |

Reprise : `git worktree list | grep gp-fix`, lire `docs/wip/fix-<lot>.md` sur chaque branche.
