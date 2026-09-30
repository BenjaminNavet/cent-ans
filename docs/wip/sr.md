# SR — Figurines semi-réalistes (orchestration)

Spec : `docs/superpowers/specs/2026-09-30-sr-semi-realiste-figurines-design.md`.
Branche `feat/sr`, worktree `../game_project-sr`. Brutes hors worktree : `~/dev/cent-ans-raw/`.
Mandat : autonomie totale (joueur, 30/09), NB2 ≈ 20 $ au total, sans variantes.

## Consigne de reprise
> Lis ce fichier et `git log --oneline feat/sr -15`, continue à la première case non cochée.

## Lots
- [ ] SR1 matières scannées ambientCG (agent `cent-ans-dev`)
- [x] SR3 planches NB2 (session principale, ≤ 1 $) → liste d'écarts
- [ ] SR2 usure et métal (après SR1, même shader)
- [ ] SR3b corrections des recettes Blender + recuisson
- [ ] SR4 captures A/B, perf, tests, ADR 0136, fusion

## Journal
- 09-30 : spec écrite, worktree créé.

## SR1 — en cours (agent)
- État : chaîne `source: ambientcg` faite, 8 scans en cache, tableaux reconstruits
  (`cent-ans assets materials --out <scratch> --scans --build --layers-sheet docs/img/sr/sr1_layers.jpg`),
  shader `GA1_TILE_SIZE` + plate ga_mix 0,6, tests pytest verts.
- Prochaine étape : SOURCE.md/CREDITS.md, tests Godot (build + import + ga1_maps_test, smoke).

## SR3b — en cours (agent)
- État : écarts écrits (`docs/research/sr3b-ecarts.md`) ; retenus 1-6 (camail sous chapel,
  manchettes de gantelets, gants de cuir, bourse+dague, bocle à la ceinture, chausses).
- Prochaine étape : `tools/blender_scripts/battle_fine_sr.py` + branchement dans `build_figure`.
