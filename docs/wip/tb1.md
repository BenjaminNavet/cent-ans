# TB1 — saisons visibles à tous les zooms (note de reprise)

Spec : `docs/design/2026-10-02-campagne-tob.md` § 3 « TB1 ». Branche `feat/tb1`, worktree
`/Users/jean_hubert/dev/gp-tb1`. Rendu Godot seulement (`core/` intact).

## État
- [x] Squelette du test `game/tests/tb1_seasons_test.gd`.
- [x] 1. Delta saisonnier par-dessus la carte de couleur (`satellite_ground` : `season_k`, `hb_ground` : couleur de saison par parcelle + `k_wild`/`k_forest`), neige de plaine en plaques régionales (`season_snow`, `winter_south`).
- [ ] 2. Mer selon la saison (hiver sombre et gris, écume de tempête).
- [ ] 3. Étalonnage par saison (`data/ui/` + schéma).
- [ ] 4. Neige sur les toits des villes 1:1 (uniform `snow`).

## Prochaine étape
Point 2 : mer selon la saison dans `water.gdshader` (teinte d'hiver, écume de tempête via
`weather_mask` + `province_ids`), valeurs lues dans `data/ui/campaign_seasons.json` (point 3).

Mesure : `godot --path game --resolution 1600x900 --script res://tests/ss_shot.gd -- --stats
--season=<saison> --distances=1100,400,90 --hide=Clouds --param=weather_enabled=false
--param=cloud_shadow_amount=0` (sans ces trois réglages, les nuées font varier la moyenne à 1100).

## Points ouverts
(aucun pour l'instant)
