# DV — deux vues de la carte de campagne

Spec : `docs/superpowers/specs/2026-09-29-dv-deux-vues-campagne-design.md`. ADR 0124.
Plan : `~/.claude/plans/docs-superpowers-specs-2026-09-29-dv-deu-delightful-snail.md`.
Worktree `../game_project-dv`, branche `feat/dv`. Coût cloud : 0 $.

## État
- [x] DV0 squelette : `ZoomTiers` à deux vues (`strategic_weight`, `Tier.STRATEGIC`,
  `model_range 1250`) ; `far_weight` / `medium_weight` gardés en enveloppes temporaires ;
  `dv_two_views_test.gd` (SKIPPED) ; ADR 0124.
- [ ] Vague 1 : DV1 câblage des vues (strategic_view, campaign_map, CityMarkers, armées, vie,
  terrain politique) ; DV2 lieux (settlement_layer sans pictogramme, écu seul, HeraldryAtlas,
  json allégé, légende).
- [ ] DV3 : retrait des enveloppes, test activé, banc FPS à 1100, 2 captures, fusion.

## Prochaine étape
Lancer DV1 et DV2 (agents cent-ans-dev, worktrees depuis feat/dv).

## Points ouverts
- `integration/fk` touche campaign_map.gd, life_effects.gd, campaign_life.gd : petits conflits à la fusion.
