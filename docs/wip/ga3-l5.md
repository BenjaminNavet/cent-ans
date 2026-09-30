# GA3-L5 — trébuchet et bélier générés branchés sur les engins animés

Branche `feat/ga3-l5`. Contexte : `docs/wip/ga3.md` (L1, L1b), ADR 0140 § « Engins de siège (L5) ».

## Approche retenue
Pas de régénération (0 $) : les LOD GA3 existants (`props_ga/ga3_{trebuchet,ram}_lod{0,2}.glb`,
déjà UV et texturés) sont **découpés par régions** (centroïdes des faces) dans Blender, chaque
pièce recalée dans le repère du nœud procédural qu'anime `SiegeEnginesFx` / `SiegeAssaultFx`
(mêmes noms : `Frame`, `ArmBeam`, `CounterweightBox`, `Shed`, `Wheel_i`, `Beam`), et le reste
(fronde, pierre, treuil, axe) reste procédural. Sortie : `game/assets/models/siege/ga3_<engin>.glb`
et `ga3_<engin>_lod.glb` (hiérarchie identique au procédural). `SiegeEnginesFx.instantiate`
choisit la variante GA3 (`data/fx/siege_engines.json`, `ga3`) sauf `--no-ga3`.

## État
- [x] Squelette (note, script Blender, test désactivé)
- [x] Script `tools/blender_scripts/ga3_siege_rig.py` : trébuchet
- [x] Script : bélier
- [x] Branchement Godot (`siege_engines.json` `ga3` + schéma + pytest, `SiegeEnginesFx.ga3_variant/kind_settings/instantiate`, `SiegeAssaultFx` lit `kind_settings`)
- [ ] Test `game/tests/ga3_l5_siege_test.gd` (écrit, à faire passer) + tests de siège + smoke
- [ ] Capture unique de contrôle
- [ ] ADR 0140 § L5, `docs/wip/ga3.md`

## Prochaine étape
Import Godot du worktree (plante parfois : relancer), puis `ga3_l5_siege_test.gd` avec et sans `--no-ga3`.
