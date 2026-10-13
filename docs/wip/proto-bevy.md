# P0 — prototype de mesure Bevy contre Godot

Spec : `docs/superpowers/specs/2026-10-13-proto-bevy-design.md` ; plan :
`docs/superpowers/plans/2026-10-13-proto-bevy-plan.md`. Worktree `../game_project-proto-bevy`,
branche `proto-bevy`. **Aucun push** (consigne du joueur).

## État
- Lot 0 (squelette) : fait. Première compilation de Bevy 0.20 : 2 min 36 s (dev, deps en opt 3).
  - Dossier `tools/proto_bevy/` (et non `tools/proto_moteur/`, ignoré en bloc par `.gitignore:60`).
  - Restaurés depuis `a600b88f2^` : `scene.json` v1 (1 428 placements, caméra, soleil, brouillard),
    `terrain.py`, `make_scene.py`, `godot/project.godot`, `godot/proto_shot.gd`.
  - Assets glTF copiés depuis `tools/proto_moteur/assets` du checkout principal (ignorés par git,
    35 Mo) ; `godot/assets` idem (44 Mo).
  - Crate `bevy/` (Bevy `=0.20.0`, workspace autonome), options `--scene --metrics --shot --at
    --dump-state --set` parsées.
  - `bench/` : projet `uv` vide.

## Prochaine étape
Vague 1 : lots A (scène + banc), B (client Bevy), C (client Godot) en parallèle.

## Points ouverts
- Source de la heightmap : le relief de bataille du jeu si exploitable, sinon `terrain.py` agrandi.
  (`game/assets/horizon/relief/*.bin` = lignes d'horizon, pas des hauteurs.)
