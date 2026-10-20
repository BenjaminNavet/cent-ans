# TS-T1 — sol style Total War (carte de campagne)

Lot T1 du chantier TS (ADR 0710, `docs/wip/ts-style-total-war.md`). Worktree `../gp-ts-t1`, branche `ts/t1`.
Fichiers : `client/src/campaign/terrain/terrain.wesl`, `season.rs`, `data/ui/campaign_seasons.json` (+ schéma).

## Objectifs
1. Neige teintée (albédo 0,72-0,78, froide à l'ombre, chaude au soleil).
2. Masque de neige altitude × faible pente (pente affichée, exagérée) × bruit en plaques ; idem neige fraîche météo.
3. Frange de boue au bord de la neige ; routes boueuses sous la neige.
4. Roche plus sombre et contrastée en altitude / pentes raides.
5. Sol assombri sous les forêts.
6. Transitions de sols en taches.

## État
- Implémenté (wip 1e7aabfda) : `SeasonParams` étendu (neige ombre/soleil, masque, boue, roche, sol forestier,
  taches) lu dans `campaign_seasons.json` (`snow.*`, nouveau bloc `ground_look`, schéma à jour).
- Cause des pentes blanches : névés/glaciers (`fw_glacier_apply`) gardés par la pente VRAIE (0,62 à 719 m/px,
  donc presque tout) + neige saisonnière gardée par la normale de sommet lissée du nœud LOD. Désormais toutes
  les neiges (saison, météo, névé, glacier) sont gardées par `disp_y` = min(normale interpolée, facette écran).
- Routes : `tint_roads_by_season` (terrain/mod.rs) passe rubans et routes fines en boue sombre l'hiver.
- Capture 1 (alps_winter) : roche sombre sur les pentes OK ; bug boue sur toutes les pentes raides corrigé.

- Neige mince (trous laissant voir le sol, bruit de détail), grain de teinte 0,86-1,10, neige fraîche
  météo à 0,7 × intensité (plaques), roche 0,11/0,10/0,09, ombre de neige 0,55/0,57/0,63.
- Captures de preuve (scratchpad de session, `t1/`) : `alps_mid2.jpg`, `alps_winter2.jpg`, `town_winter2.jpg`.
- Vérifs : fmt, clippy `--features dev --all-targets`, `cargo test --features dev` (1331 + intégration) OK.

## Restes / points ouverts
- Frange de boue peu visible à distance (bande étroite) ; à élargir (`snow.mud_fringe`) si le joueur la veut.
- Plaques de neige des plaines d'un motif « camouflage » assez régulier en vue moyenne ; fréquence
  `snow.patch_frequency` à ajuster à l'œil.
- Assombrissement global/froid de l'hiver = étalonnage (lot T3) ; arbres tramés / blocs gris = lot T2.

## Prochaine étape
Fusion par la session principale.
