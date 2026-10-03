# RJ-c — expliquer la conquête, bannières sur les villes

Branche `feat/rj-c`, worktree `../gp-rj-c`. Chantier : `docs/wip/rj-retours-joueur.md`. ADR 0175 (sections Explication et Bannières).

## Plan
1. Cœur : `sim-campaign/src/possession.rs` (statut d'un lieu / d'une province pour un observateur : `own`, `own_occupied`, `occupied_by_viewer`, `foreign`, `foreign_occupied` ; places tenues / total ; détenteur de la province entière). Pont : `campaign_sim_possession.rs` (`province_possession`, `settlement_possession`). Rappel « occuper n'est pas posséder » dans le texte de capture (`capture.rs`, titre « X est prise »).
2. UI : `game/scripts/ui/possession_text.gd` (textes FR des statuts, couleur de position) ; survol de province sur la carte ; en-tête et liste du panneau de province ; panneau de colonie ; entrée du Codex « Conquête et possession » (`encyclopedia.gd`).
3. Bannières : écu du propriétaire au-dessus du nom (MultiMesh existant `settlement_icon.gdshader`), second petit écu de l'occupant si occupée, liseré de couleur de position (ADR 0155) ; parchemin : fanion du propriétaire et de l'occupant sur les cités. Paramètres `data/map/settlement_markers.json` (`banner`).
4. Tests GDScript, cargo test, smoke.

## État
- [x] 1 cœur + pont (`cargo test --test rj_possession` vert)
- [x] 2 UI explication (panneaux, survol, codex `mech_conquest`, infobulle `province_possession`)
- [x] 3 bannières (shader + `settlement_layer.gd` + `parchment_overlay.gd`)
- [x] ADR 0175 (sections RJ-c ; section RJ-d vide)
- [ ] 4 tests : `game/tests/rj_possession_test.gd` à faire passer après `core/build.sh` ; clippy + cargo test complets ; smoke

## Prochaine étape
Build, `godot --headless --path game --script res://tests/rj_possession_test.gd`, smoke.
