# DA3 — Marqueurs de carte (langage unique)

Lot DA3 de `docs/wip/da-direction-artistique.md`, bible `docs/design/2026-09-25-bible-da.md` § 7-8.
Branche `worktree-agent-a4046e3d4756f0516` (fusionnée avec `feat/da-direction-artistique` et main).
Plafond du lot : 1,5 $ ; **dépensé 0,59 $** (sonde 3 images 0,14 $ + 10 images 0,45 $). ADR 0066.
**État : terminé, non fusionné dans main.**

## Constat (captures `docs/img/da3/*_avant.jpg`)
- `SettlementLayer` : formes SDF remplies de la couleur de faction (rosaces roses, soleils jaunes,
  pastilles bleues, écus crénelés), palier moyen seulement.
- Palier Europe (parchemin CM2) : vignettes à l'encre de toutes les cités (autre langage).
- Légende UX1 : copie des formes SDF.

## Fait
- `data/map/settlement_markers.json` + schéma : 12 pictogrammes (capitale, grande cité, cité, ville
  close, ville, bourg, forteresse, château, tour, grande abbaye, abbaye, insigne de port), rang
  1-4 par règles (listes capitales / grandes cités, fortification, poids), tailles par rang,
  paliers de dé-encombrement (tout < 380 ; rangs 2+ < 700 ; au-delà capitales, grandes cités,
  forteresses), placement écu / insigne.
- `tools/cent_ans_tools/map_markers.py` + CLI `cent-ans assets map-markers [--dry-run] [--only id]
  [--atlas-only]` : génération gpt-5-image-mini (sources `tools/assets/map_markers/*.jpg`),
  détourage (inondation arrêtée par le trait d'encre), liseré d'encre + halo vélin communs,
  atlas `game/assets/map/markers/settlement_markers.png` (4×3 cases de 128 px, mipmaps).
  Tests `tools/tests/test_map_markers.py`.
- `SettlementMarkers` (GDScript, lecture du catalogue, rang, visibilité, atlas d'écus depuis
  `PortraitLoader.heraldry_texture`) ; `settlement_icon.gdshader` réécrit (atlas + écu + port,
  fondu par instance, quad replié hors palier) ; `SettlementLayer` (instances en ordre inverse :
  grands lieux dessus ; picking et étiquettes à la taille du marqueur ; marqueurs aussi au palier
  Europe) ; parchemin sans vignettes de villes ; légende avec les vrais marqueurs.
- Correctif : `PROJECTION_MATRIX[1][1]` signé retournait les quads (180°).
- Test `game/tests/da3_markers_test.gd` ; `ux1_test` adapté.

## Perf (`--fps-probe`, 1600×900, Metal, machine partagée, base = f6ab5a23 alternée, 3 tours)
| Vue | Base i/s (médiane) | DA3 i/s (médiane) | c6_layers_ms base → DA3 |
|---|---|---|---|
| Europe d=1400 | 86,1 | 88,1 | 0,17 → 0,18 |
| Province d=420 | 64,9 (80,5 / 64,9 / 59,9) | 74,7 | 0,61 → 0,63 |
| Comté d=110 | 40,9 | 40,6 (−0,7 %) | 0,41 → 0,41 |
Appels de dessin : Europe 1690 → 1451 (vignettes du parchemin retirées), sinon identiques.
Pas de régression > 2 % (la dispersion de la machine dépasse l'écart).

## Captures
`docs/img/da3/` : `{europe,province,comte}_{avant,apres}.jpg`, `legende_apres.jpg`,
`selection_apres.jpg`, `atlas_pictogrammes.jpg`.

## Limites / suites
- Pas de dé-encombrement écran (chevauchements possibles dans les régions denses au palier
  moyen, ex. pays de Galles) : seulement par rang et distance.
- L'écu montre le contrôleur ; écus de maison (DA1) branchables sur le même atlas.
- Bible § 10 ligne 3 à marquer « fait (DA3) » à la fusion.
