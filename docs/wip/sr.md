# SR — Figurines semi-réalistes (orchestration)

Spec : `docs/superpowers/specs/2026-09-30-sr-semi-realiste-figurines-design.md`.
Branche `feat/sr`, worktree `../game_project-sr`. Brutes hors worktree : `~/dev/cent-ans-raw/`.
Mandat : autonomie totale (joueur, 30/09), NB2 ≈ 20 $ au total, sans variantes.

## Consigne de reprise
> Lis ce fichier et `git log --oneline feat/sr -15`, continue à la première case non cochée.

## Lots
- [x] SR1 matières scannées ambientCG (agent `cent-ans-dev`) — couches 0-7 = scans CC0 entiers
  (normale/rugosité/déplacement du scan), `tile_m` physique (0,15/0,13/0,16/0,32/0,51/0,3/0,3/0,8 m),
  couches 8-11 recopiées ; plate ga_mix 0,6 ; planche `docs/img/sr/sr1_layers.jpg`.
- [x] SR3 planches NB2 (session principale, ≤ 1 $) → liste d'écarts
- [ ] SR2 usure et métal (après SR1, même shader)
- [ ] SR3b corrections des recettes Blender + recuisson
- [ ] SR4 captures A/B, perf, tests, ADR 0136, fusion

## Journal
- 09-30 : spec écrite, worktree créé.

## SR1 — fait (agent)
- Chaîne : `uv run --project tools cent-ans assets materials --out <scratch> --scans --build --layers-sheet docs/img/sr/sr1_layers.jpg`.
- Tests : pytest `test_material_gen.py` + `test_sr1_scans.py` (21), `ga1_maps_test.gd` OK (2,33 Mo), `smoke.gd` OK.
- À juger en jeu (SR4) : échelle des fils (0,6-2 mm, fondus par les mipmaps de loin), normale
  plate/cuir renforcée (x4/x3), rugosité laine/gambeson/bois relevée.

## SR2 — en cours (agent)
- Squelette : uniformes `weathering` (0,6) et `sr2_mud_height` (0,45 ; cavalerie 0,65) dans
  la variante FG3_BAKED, posés par `BattleSkinned._setup_fine_maps`, `--no-sr2` → 0.
- Prochaine étape : effets dans le fragment (boue, crasse, acier, teintes), test `sr2_weathering_test.gd`.

## SR5 — en cours (agent)
- Include `game/shaders/building_aging.gdshaderinc` (usure procédurale, sans texture).
- Prochaine étape : SR5a (atlas + villes), SR5b (`building_pbr.gdshader`, `--no-sr5`), tests.
