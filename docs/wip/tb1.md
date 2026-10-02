# TB1 — saisons visibles à tous les zooms (note de reprise)

Spec : `docs/design/2026-10-02-campagne-tob.md` § 3 « TB1 ». Branche `feat/tb1`, worktree
`/Users/jean_hubert/dev/gp-tb1`. Rendu Godot seulement (`core/` intact).

## État
- [x] Squelette du test `game/tests/tb1_seasons_test.gd`.
- [x] 1. Delta saisonnier par-dessus la carte de couleur (`satellite_ground` : `season_k`, `hb_ground` : couleur de saison par parcelle + `k_wild`/`k_forest`), neige de plaine en plaques régionales (`season_snow`, `winter_south`).
- [x] 2. Mer selon la saison (`water.gdshader` : `season_*`, `storm_*` ; `Sea.apply_season` / `sync_weather` appelés par `CampaignLife._sync_sea` ; valeurs dans `data/ui/campaign_seasons.json`, lues par `SeasonLook`).
- [x] 3. Étalonnage par saison : bloc `grade` de `data/ui/campaign_seasons.json`, composé par `CampaignAtmosphere.resolve_preset` (ADR 0150).
- [x] 4. Neige sur les toits des villes 1:1 : paramètre global `campaign_roof_snow` (`project.godot`), lu par `roofscape.gdshaderinc` (`roof_snow`) dans `town_building` et `town_far`, publié par `CampaignLife._sync_sea` depuis `SeasonLook.roof_snow`. Suie non faite (état par ville, voir ADR 0150).

## Prochaine étape
Lot livré (4 points). Reste à la session principale : capture de contrôle (Paris `--at=2213.2,3203.9`
à 1100 / 400 / 90 en `--season=summer|autumn|winter` ; mer `--at=1000,2800`), puis réglage à l'œil
de `sg_season_strength` (1,3), `hb_season_strength` (0,8), `winter_cover`, et des valeurs de
`data/ui/campaign_seasons.json`.

Mesure : `godot --path game --resolution 1600x900 --script res://tests/ss_shot.gd -- --stats
--season=<saison> --distances=1100,400,90 --hide=Clouds --param=weather_enabled=false
--param=cloud_shadow_amount=0` (sans ces trois réglages, les nuées font varier la moyenne à 1100).

Chiffres `--stats` (moyenne RVB du tiers bas, Paris, nuées masquées), 02/10 :

| Distance | Été | Automne | Hiver | Printemps |
|---|---|---|---|---|
| 1100 | 96 86 59 | 112 85 59 | 106 108 115 | 94 105 92 |
| 400 | 99 89 53 | 113 87 54 | 88 89 89 | 85 97 58 |
| 90 | 95 78 48 | 105 78 53 | 70 70 68 | 79 82 51 |

Avant TB1 (nuées non masquées) : été 91 84 62 / 78 73 48 / 100 86 59 ; automne 92 87 70 / 95 85 66 /
111 95 72 ; hiver 115 115 111 / 82 84 70 / 96 92 80.
Mer (Atlantique, 400 / 90) : été 8 15 20 / 12 22 25 ; automne 7 12 16 / 15 22 25 ; hiver 6 9 13 / 10 14 18.

Mer : `--at=1000,2800` (Atlantique, mer franche à 400 et 90), `--at=2030,2950` (Manche, 90 seulement).

## Points ouverts
- `terroir_burn` (brûlis) est posé avant `sg_apply` / `hb_apply` : la carte de couleur et les
  matières le recouvrent encore (hors lot TB1, même correctif possible).
- Classe de culture des parcelles HB tirée au sort (pas liée à la matière de la parcelle).
- L'étalonnage par saison existait déjà (`data/fx/atmosphere.json`, PO3) : le point 3 ajoute une
  surcouche de carte dans `data/ui/` (ADR 0150).
- Écume de tempête : non mesurée en rendu (dépend de la météo du cœur) ; à juger sur capture.
- Suie des villes 1:1 saccagées : non faite (demande un attribut par ville).
- `zg8_relief_test` échoue déjà sur `feat/tb1` avant TB1 (`rock_outcrops.gdshader` redéclare
  `campaign_vertical_scale`) : sans rapport avec ce lot.
- Nouveau paramètre global dans `game/project.godot` (`campaign_roof_snow`) : conflit possible à
  la fusion si un autre lot touche `[shader_globals]`.
- Nouvelle classe `SeasonLook` : relancer `godot --headless --path game --import` après fusion
  (cache des classes globales).
