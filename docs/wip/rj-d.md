# RJ-d — lavis par position diplomatique (ADR 0175)

Chantier parent : `docs/wip/rj-retours-joueur.md`. Worktree `../gp-rj-d`, branche `feat/rj-d`.

## Approche
- `game/scripts/map/stance_fill.gd` (StanceFill) : texture 1D par province (index raster) =
  couleur de la position du **contrôleur** envers le joueur (catégories `StanceCues`), alpha par
  catégorie ; refaite seulement si contrôleurs/positions changent (`refresh_all`).
- `game/shaders/stance_fill.gdshaderinc` : crochet `sf_fill` juste avant `fr1_borders` dans
  terrain.gdshader et terrain_parchment.gdshader (diff du shader = 2 lignes chacun).
- Réglages `data/map/stance_fill.json` (schéma `stance_fill.schema.json`).
- Option `map/stance_fill` (Réglages › Carte), défaut on ; `--no-stance-fill` pour A/B.

## État
- [x] Squelette (API, JSON, schéma, include vide)
- [ ] Implémentation script + shader + câblage campaign_map / Réglages
- [ ] Tests (GDScript pur, pytest schéma), smoke
- [ ] Capture `game/tests/rj_fill_shot.gd`, réglage alpha
- [ ] ADR 0175 section RJ-d

## Prochaine étape
Implémenter `StanceFill` et le crochet shader.
