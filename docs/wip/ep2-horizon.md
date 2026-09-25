# EP2 — Horizon des batailles

Branche : `worktree-agent-a5e4208be5556aa24`. ADR : `docs/decisions/0032-horizon-de-bataille.md`.
Plan d'ensemble : `docs/wip/epic.md`.

## Conception retenue
- **Relief réel** : `cent-ans geo horizon` cuit, par province (centroïde, ou point à 3-5 km de la
  mer ouverte pour une province côtière), une tuile 26 × 26 km à 100 m (Copernicus GLO-90 moyenné,
  repli tuiles fines 360 m hors emprise), altitude int16 (quart de mètre) + classe (forêt de
  `splat.png`, mer) + profil de ligne d'horizon réel 12-150 km sur 256 azimuts, dégonflés dans
  `game/assets/horizon/relief/<prov>.bin` (132 tuiles, 11 Mo) + `index.json`.
- **Raccord** : `BattleTerrain.world_height` → `BattleHorizon.blend` (0,7 → 3 km du bord du champ ;
  rivière 450 m et côte B5 gardent la main). Tuile tournée pour que la mer réelle tombe du côté du
  flanc côtier de la simulation. Bois lointains = forêts réelles.
- **Anneau d'horizon** 7 → 13 km (pas 400 m, `battle_horizon_land.gdshader`, brume `FOG` calée
  sur celle de la scène, allégée en altitude, neige en plaques au-dessus de la limite de saison),
  bord extérieur plongeant ; **mer lointaine** (`battle_sea` réutilisé).
- **Panorama** (`battle_panorama.gdshader`) : cylindre 14,6 km qui suit la caméra ; chaque colonne
  peinte est recalée sur la crête réelle de son azimut (`paint_detail` = part du relief peint
  gardée) ; secteurs de mer = horizon marin.
- **Silhouettes** : villages (église + maisons), château sur une hauteur, fumées (shader billboard).
- **Données** : `data/fx/horizon.json` + `data/schemas/fx_horizon.schema.json`.
- Drapeaux : `--no-horizon` (A/B), `--horizon-province=<id>`, `--panorama=<id>`, `--no-hud`
  (captures sans interface, `BattleScene._take_screenshot`).

## État
- [x] cuisson relief + index (`tools/cent_ans_tools/geo/horizon.py`, tests `tools/tests/test_horizon.py`)
- [x] panoramas : 13 bandes (sonde puis lot, Flandre régénérée), 0,57 $ consignés dans `docs/budget.md`
  (`cent-ans assets horizon-panoramas [--generate]`, originaux `tools/horizon_raw/`)
- [x] intégration Godot (anneau, mer, cylindre, silhouettes), test `game/tests/ep2_horizon_test.gd`
- [x] brume / fog (densité par météo dans `horizon.json`)
- [x] mesures A/B, captures `docs/img/ep2/` (before_/after_ : plaine, cote, montagne, alpes, hiver)
- [ ] fusion de main, smoke

## Mesures (M4 Pro partagé avec d'autres agents, Vulkan, 1600 × 900, qualité Haute, 1 160 soldats)
`--benchmark --bench-repeat=2 --terrain=hills --horizon-province=prov_bearn [--camera=600,740,40,180]
[--no-horizon]`, temps GPU moyen (ms) :
- vue par défaut (plongeante) : horizon 9,30 / 9,31 ; sans 9,37 / 9,81 → pas d'écart mesurable ;
- vue rasante face aux Pyrénées : horizon 10,28 / 9,79 / 9,95 (une passe aberrante à 20,9 écartée,
  machine chargée) ; sans 10,77 / 9,22 / 9,06 → **+0,6 à +0,9 ms** au pire, dans le budget de 1 ms.
  Anneau d'horizon : 5 004 triangles ; panorama : 256 triangles ; silhouettes : quelques centaines.

## Prochaine étape
Fusion de main, smoke ; ensuite (hors lot) : tuile au lieu exact de la bataille quand la campagne
exportera une position, tuiles des sites historiques pour EP7.

## Captures
`godot --path game --resolution 1600x900 res://scenes/battle/battle.tscn -- --deploy-shot
--screenshot=<png> --camera=<x,z,d,lacet> --no-hud --terrain=<…> --horizon-province=<id>
[--coast] [--season=winter] [--no-horizon]`.
