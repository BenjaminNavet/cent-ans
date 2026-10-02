# TB1 — saisons visibles à tous les zooms (note de reprise)

Spec : `docs/design/2026-10-02-campagne-tob.md` § 3 « TB1 ». Branche `feat/tb1`, worktree
`/Users/jean_hubert/dev/gp-tb1`. Rendu Godot seulement (`core/` intact).

## État
- [x] Squelette du test `game/tests/tb1_seasons_test.gd`.
- [x] 1. Delta saisonnier par-dessus la carte de couleur (`satellite_ground` : `season_k`, `hb_ground` : couleur de saison par parcelle + `k_wild`/`k_forest`), neige de plaine en plaques régionales (`season_snow`, `winter_south`).
- [x] 2. Mer selon la saison (`water.gdshader` : `season_*`, `storm_*` ; `Sea.apply_season` / `sync_weather` appelés par `CampaignLife._sync_sea` ; valeurs dans `data/ui/campaign_seasons.json`, lues par `SeasonLook`).
- [x] 3. Étalonnage par saison : bloc `grade` de `data/ui/campaign_seasons.json`, composé par `CampaignAtmosphere.resolve_preset` (ADR 0150).
- [ ] 4. Neige sur les toits des villes 1:1 (uniform `snow`).

## Prochaine étape
Point 4 : neige des toits des villes 1:1 par paramètre global `campaign_roof_snow`
(`project.godot`, `town_building.gdshader`, `town_far.gdshader`), publié par `CampaignLife`.

Mesure : `godot --path game --resolution 1600x900 --script res://tests/ss_shot.gd -- --stats
--season=<saison> --distances=1100,400,90 --hide=Clouds --param=weather_enabled=false
--param=cloud_shadow_amount=0` (sans ces trois réglages, les nuées font varier la moyenne à 1100).

Mer : `--at=1000,2800` (Atlantique, mer franche à 400 et 90), `--at=2030,2950` (Manche, 90 seulement).

## Points ouverts
- `terroir_burn` (brûlis) est posé avant `sg_apply` / `hb_apply` : la carte de couleur et les
  matières le recouvrent encore (hors lot TB1, même correctif possible).
- Classe de culture des parcelles HB tirée au sort (pas liée à la matière de la parcelle).
- L'étalonnage par saison existait déjà (`data/fx/atmosphere.json`, PO3) : le point 3 ajoute une
  surcouche de carte dans `data/ui/` (ADR 0150).
- Écume de tempête : non mesurée en rendu (dépend de la météo du cœur) ; à juger sur capture.
