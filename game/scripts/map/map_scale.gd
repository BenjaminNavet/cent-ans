class_name MapScale
extends RefCounted

## Lot GC (ADR 0158) : carte généralisée. Les positions sont vraies, les objets sont grossis d'un
## facteur constant, indépendant de la distance de caméra. Purement visuel.
##
## `town_scale()` : grossissement des villes autour de leur centre (`map.town_scale` de
## `data/ui/campaign_map.json`, 1 = échelle réelle). Les villes gardent leurs plans en mètres ;
## elles sont posées avec un nombre de mètres par unité divisé par ce facteur
## (`TownData.meters_per_unit`, `LandmarkV2Library.town_meters_per_unit`). `--town-scale=<k>` après
## `--` le remplace (captures de réglage).

static var _town_scale: float = -1.0


static func town_scale() -> float:
	if _town_scale < 0.0:
		_town_scale = maxf(float(ArmyFigures.map_settings().get("town_scale", 1.0)), 1.0)
		if CmdArgs.has("--town-scale"):
			_town_scale = maxf(CmdArgs.number("--town-scale"), 1.0)
	return _town_scale


## Multiplicateur des distances de rendu des villes (`TownRenderProfile`) : `map.town_range_scale`,
## à défaut le grossissement lui-même ; `--town-range-scale=<k>` le remplace.
static func town_range_scale() -> float:
	var k := float(ArmyFigures.map_settings().get("town_range_scale", town_scale()))
	k = CmdArgs.number("--town-range-scale", k)
	return maxf(k, 1.0)


static func clear_cache() -> void:
	_town_scale = -1.0
