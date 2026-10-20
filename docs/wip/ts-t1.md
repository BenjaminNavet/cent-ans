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
- Squelette : note wip créée.

## Prochaine étape
Diagnostic de la normale du masque de neige (dump), puis implémentation.
