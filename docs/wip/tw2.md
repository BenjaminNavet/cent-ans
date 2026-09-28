# WIP orchestrateur — TW2 mécaniques Total War (28/09)

Plan : `docs/design/2026-09-28-tw2-mecaniques-total-war.md`. Mandat : enchaîner les lots sans validation.
Chaque lot a sa note `docs/wip/tw2-<lot>.md` dans sa branche `feat/tw2-<lot>`.
Fusion : worktree `../gp-tw2-merge` (branche `integration/tw2`), `git merge main`, merge du lot, clippy +
tests + `core/build.sh` + `--import` + smoke, puis `git merge --ff-only` dans main.
Session RS parallèle (`docs/wip/restes.md`) : ADR 0098/0099 à elle ; conflits probables avec RS B
(économie) et RS F (bataille).

| Lot | Branche | État |
|---|---|---|
| SB barres de vie + rythme siège | feat/tw2-sb | fini (6fcd5080), à fusionner après vérif T1+T2 ; br3 bascule 3/10→10/10, confié à T4 |
| T1 sort de la ville prise | feat/tw2-t1 | fini (9851cd5d), fusionné dans integration/tw2 |
| T2 reconstitution + réserves | feat/tw2-t2 | fini (fd7e7275), fusionné dans integration/tw2 (conflit load.rs résolu) |
| T3 mercenaires | feat/tw2-t3 | lancé, part de integration/tw2 |
| T4 points de capture + rééquilibrage br3 (cible 4-7/10) | feat/tw2-t4 | lancé, part de feat/tw2-sb |
| T5 traditions d'armée | feat/tw2-t5 | vague 2 (après T2) |

integration/tw2 (../gp-tw2-merge) = main + T1 + T2, vérification en cours avant ff dans main.
Suites T1 : griser recrutement d'une ruine, marqueur de ruine. Suites T2 : dilution d'expérience (avec T5), IA qui se met au repos pour se reconstituer.
