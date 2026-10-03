# RJ-d — lavis par position diplomatique (ADR 0175)

Chantier parent : `docs/wip/rj-retours-joueur.md`. Worktree `../gp-rj-d`, branche `feat/rj-d`.

## Approche
- `game/scripts/map/stance_fill.gd` (StanceFill) : texture 1D par province (index raster) =
  couleur de la position du **contrôleur** envers le joueur (catégories `StanceCues`), poids par
  catégorie ; refaite seulement si contrôleurs/positions changent (`refresh_all`).
- `game/shaders/stance_fill.gdshaderinc` : crochet `sf_fill` juste avant `fr1_borders` dans
  terrain.gdshader et terrain_parchment.gdshader (diff du shader = 2 lignes chacun).
- Réglages `data/map/stance_fill.json` (schéma `stance_fill.schema.json`).
- Option `map/stance_fill` (Réglages › Carte), défaut on ; `--no-stance-fill` pour A/B.

## État
- [x] Squelette, implémentation, câblage campaign_map / Réglages
- [x] Tests : `rj_stance_fill_test.gd` OK, pytest schéma OK
- [x] Captures (3/3) : poids relevés à 0,45/0,42/0,35/0,10, saturation 0,8
- [x] ADR 0175 section RJ-d
- [ ] smoke.gd (en cours au dernier commit)

## Points ouverts
- Pas de bench FPS chiffré (coût : 1 texelFetch + ~15 ALU par fragment, early-out uniforme) ;
  A/B possible avec `--no-stance-fill` et la sonde FPS de la carte.
- Vue rapprochée et parchemin non capturés (budget 3 captures) : à juger en partie pilote.
- Fusion : ADR 0175 partagé avec RJ-c (sections distinctes) ; terrain.gdshader touché par RV
  (2 lignes isolées ici).
