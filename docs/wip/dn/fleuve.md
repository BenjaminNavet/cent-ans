# DN-FLEUVE : glb générés sur et au bord de l'eau (carte de campagne)

Branche worktree `worktree-agent-a956a1c66be203390`, non fusionnée. ADR provisoire 0216 (à renuméroter à la fusion).

Périmètre : navires d'armée et de commerce, bateaux de fleuve, ponts, moulins à eau, ports, chantiers navals, épaves, oiseaux d'eau. Hors périmètre : rendu de l'eau (DN-MER), forêts, champs, roches, pâtures.

## Données
- `data/art/dn_water_models.json` (+ `data/schemas/art_dn_water_models.schema.json`, test `tools/tests/test_schemas.py`) : registre `models` (id -> glb sans `_lodN`, longueur en unités carte) et règles : `fleet` (bassin, culture), `sea_lanes` (bassin), `river_boats` (nom de fleuve), `bridges` (structure, `by_id`), `mills`, `ports`, `shipyards`, `wrecks`.
- `game/scripts/map/dn_water_models.gd` (`DnWaterModels`) : lecture, choix par règle, mesure du glb et repère de pose (proue sur +X, axe long auto-aligné, `yaw_deg` par modèle).
- Modèles dans `game/assets/models/dn/` (gitignoré, lien symbolique dans le worktree).

## Plan (lots)
1. Squelette : données, schéma, résolveur.
2. Navires d'armée (`army_figures.gd::_build_fleet`).
3. Lignes maritimes et fleuves (`life_ambient.gd`).
4. Ponts (`river_crossings.gd`).
5. Décors de rive : moulins, ports, chantiers, épaves (`water_props_layer.gd`, créé par `campaign_life.gd`).
6. Oiseaux d'eau (`data/map/map_birds.json`, `map_bird_flocks.gd`).
7. Tests, mesure de coût, captures (5 au plus).

## État
- [x] 1 squelette
