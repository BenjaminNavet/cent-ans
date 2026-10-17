# Correctifs du playtest du 17/10

Source : `docs/wip/playtest-2026-10-17/SYNTHESE.md`. Mandat joueur (17/10) : corriger tout ce qui a été relevé, via agents.
Un worktree par lot `../gp-fix-<lot>`, branche `fix/<lot>` depuis main 77d21ef8a, note `docs/wip/fix-<lot>.md` sur la branche.
L'orchestrateur relit, fusionne dans un worktree d'intégration puis `--ff-only` dans main. Pas de push.

| Lot | Contenu | Modèle | État |
|---|---|---|---|
| 1 render | terrain noir ultra plein écran, étiquettes sur le menu pause, doublon d'étiquettes | Opus | fusionné |
| 2 campui | recrutement, économie, traditions, UI très grande, sélection de faction, « à paris », boutons fermer | Sonnet | fusionné |
| 3 battleui | dialogue pré-bataille, écran de résultat, élision, discours, F1, icônes | Sonnet | fusionné |
| 4 overlays | bulle du conseiller, file des modales, raccourcis J/C/T, pause | Sonnet | fusionné |
| 5 saveload | journal/chronique/féodal/tutoriel perdus au chargement | Opus | fusionné |
| 6 battlesim | déploiement, issue du siège, IA passive, fuyards figés, chances navales | Opus | fusionné |
| 7 camprules | retraite après défaite, emplacements 9/6, entretien, 213 provinces, eau ×2 | Opus | fusionné |
| 8 mapfx | rochers noirs, charrette, mât, échelles, chevaux, pavois | Opus | fusionné |
| 9 mapgeo | baie du Mont-Saint-Michel, Léman, villes sur l'eau, rivières, côtes, étiquettes | Opus | fusionné |
| 10 fog | météo brouillard → brume visible (décision joueur 17/10) | Sonnet | en cours, branche depuis fix/integ |
| 11 perf | campagne < 40 i/s, dérive du temps d'image | Opus | après fog (machine calme) |

Reprise : `git worktree list | grep gp-fix`, lire `docs/wip/fix-<lot>.md` sur chaque branche.

## État au 17/10 midi
Lots 1-9 dans main (78fb7f887) : tests core/client/Python, smoke 10/10, seed bots rapides 0 constat.
Restes par lot : `docs/wip/fix-<lot>.md`. Notables : terrain noir ultra plein écran non prouvé (procédure joueur dans fix-render.md),
Londres (Tamise cuite dans le glb Meshy), Mont non visible, charrette poreuse au loin, morts clairs, forêts d'hiver, Pétra,
attaquant IA lent face à un défenseur passif (ADR 0184), pas d'ADR pour la sortie du défenseur.
Pyramide partagée `~/dev/cent-ans-raw/compose/pyramid` retouchée par mapgeo (coast-patch, lake-patch) : cohérente avec main.
