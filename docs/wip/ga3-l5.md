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

## État (terminé, 30/09)
- [x] Squelette, script `tools/blender_scripts/ga3_siege_rig.py` (trébuchet, bélier), `siege_engines.py` protégé par `__main__`
- [x] Branchement : `ga3` de `data/fx/siege_engines.json` (+ schéma, pytest), `SiegeEnginesFx.ga3_variant` /
      `kind_settings` / `instantiate` (méta `ga3`), `SiegeAssaultFx` lit `kind_settings` (corde de la poutre)
- [x] Tests : `ga3_l5_siege_test.gd` (avec et sans `--no-ga3`), smoke, `ga3_l1_decor_test` (x2),
      `nt5_cap_engines_test` (x2), `nt8_castle_test`, `nt10_test`, `nt7_anim_test`, `sb_siege_bars_test`
- [x] Capture unique `ga3_l5_siege_shot.gd` → `docs/audit/captures/ga3/ga3_l5_siege.png` (non suivie)
- [x] ADR 0140 § L5, `docs/wip/ga3.md`, notes du catalogue

## Points ouverts
- Jugement en jeu par le joueur (bataille de siège réelle).
- Tache sombre sur le bâti arrière du trébuchet (fond de la caisse GA3 resté sur le bâti, vu
  dans la capture) : retirer les faces basses de l'ancienne assise de la caisse si gênant.
- Proportions forcées (bâti ×0,86, caisse aplatie en plan), traverses étirées par le vide
  central ; poutre du bélier qui traverse le toit aux grands balancements (déjà en procédural).
- Servants (`crew.layouts`) inchangés : placés pour le bâti procédural (3,8 m de large ; GA3 ≈ 7 m).
