# DV — deux vues de la carte de campagne

Spec : `docs/superpowers/specs/2026-09-29-dv-deux-vues-campagne-design.md`. ADR 0124.
Plan : `~/.claude/plans/docs-superpowers-specs-2026-09-29-dv-deu-delightful-snail.md`.
Worktree `../game_project-dv`, branche `feat/dv`. Coût cloud : 0 $.

## État : TERMINÉ (30/09)
- [x] DV0 squelette, ADR 0124.
- [x] DV1 câblage des vues (strategic_view, CityMarkers supprimé, routes, armées, vie,
  `political_amount = 0` dans terrain.gdshader) ; notes `docs/wip/dv1.md`.
- [x] DV2 lieux (écu seul, HeraldryAtlas, json allégé, PNG et outil supprimés, légende) ;
  notes `docs/wip/dv2.md`.
- [x] DV3 enveloppes retirées, `dv_two_views_test` actif, main (FK) fusionné, ombres des
  maquettes coupées > 500, banc A/B (ADR 0124), 2 captures (1100, 1400) conformes.

## Points ouverts
- Appels de dessin +21 % à d = 1100 : à mesurer sur machine calme (chantier FPS carte).
- Taille et écart de l'écu au nom : à juger par le joueur en partie.
- `da7d_overlap_test` au-dessus de son seuil sous charge (déjà le cas sur la base) ;
  `sz4b_colonies_forests_test` 6 échecs antérieurs à DV ; erreur « SimFacade » à la compilation
  de `map_mode_controller.gd:388` dans ux1 (le test passe).
- Peintures sources `tools/assets/map_markers/*.jpg` inutilisées : à supprimer ou garder.
