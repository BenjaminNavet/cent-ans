# SR — Figurines semi-réalistes (orchestration)

Spec : `docs/superpowers/specs/2026-09-30-sr-semi-realiste-figurines-design.md`.
Branche `feat/sr`, worktree `../game_project-sr`. Brutes hors worktree : `~/dev/cent-ans-raw/`.
Mandat : autonomie totale (joueur, 30/09), NB2 ≈ 20 $ au total, sans variantes.

## Consigne de reprise
> Lis ce fichier et `git log --oneline feat/sr -15`, continue à la première case non cochée.

## Lots
- [ ] SR1 matières scannées ambientCG (agent `cent-ans-dev`)
- [ ] SR3 planches NB2 (session principale, ≤ 1 $) → liste d'écarts
- [ ] SR2 usure et métal (après SR1, même shader)
- [ ] SR3b corrections des recettes Blender + recuisson
- [ ] SR4 captures A/B, perf, tests, ADR 0136, fusion

## Journal
- 09-30 : spec écrite, worktree créé.

## SR1 — en cours (agent)
- État : squelette ; chaîne `source: ambientcg:<Id>` à écrire dans `material_gen.py`.
- Prochaine étape : téléchargement + traitement des 8 scans, `scan_m`/`tile_m`, shader, tests.
