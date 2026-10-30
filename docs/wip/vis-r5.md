# VIS-R5 — Eau : bathymétrie et rivières de carte

Spec : `docs/audits/2026-10-29-visuel-campagne.md` (tableau d, ligne R5 ; G2-2, G2-8 ; bible c).
Branche : worktree agent-a6e0f261dba299bab (partie de main 8323c8cdc). Ne pas pousser ni fusionner.

## Plan
1. Outil `cent-ans geo bathymetry` (`tools/cent_ans_tools/geo/bathymetry.py`) : GEBCO 2025 (hors dépôt,
   `~/dev/cent-ans-raw/gebco/`) -> warp inverse de la carte composée -> profondeur (m) -> L8 racine carrée,
   1/4 de la carte : `data/map/bathymetry.png`.
2. `sea.wesl` : binding 13 (texture seule, échantillonneur `coast_sampler` réutilisé : 0 échantillonneur de plus).
   Profondeur = bathymétrie (bornée par la distance à la côte composée). Liseré cyan retiré, estrans sable/vase.
3. Fleuves : `river_display.json` (`major_min_px`, échelle par débit, berges claires, ripisylve), Gironde.
4. Données + schémas, ADR `docs/decisions/XXXX-vis-r5-eau.md`, crédits GEBCO.

## État
- [ ] squelette
