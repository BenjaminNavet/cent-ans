class_name UnitRank
extends RefCounted

## Rang d'expérience d'un régiment (carte du bandeau d'ost, infobulle). Niveau maximal et libellés
## des paliers dans `data/ui/unit_ranks.json` (schéma `unit_ranks_ui`) ; le niveau vient du cœur
## (`unit.experience`). Aucune règle de jeu.

const DEFAULTS := {"max_level": 10, "tiers": [{"min_level": 0, "name": "Levée"}]}

static var _lookup := JsonLookup.new("ui/unit_ranks.json", DEFAULTS)


## Niveau maximal d'expérience.
static func max_level() -> int:
	return maxi(int(_lookup.value("max_level", 10)), 1)


## Niveau borné de l'unité (`experience` du Dictionary d'unité).
static func level_of(unit: Dictionary) -> int:
	return clampi(int(unit.get("experience", 0)), 0, max_level())


## Nombre de chevrons à dessiner (un par niveau).
static func chevron_count(unit: Dictionary) -> int:
	return level_of(unit)


## Libellé du palier le plus haut atteint par `level`.
static func tier_name(level: int) -> String:
	var best := ""
	var best_min := -1
	for tier in _lookup.value("tiers", []):
		var min_level := int(tier.get("min_level", 0))
		if min_level <= level and min_level > best_min:
			best_min = min_level
			best = str(tier.get("name", ""))
	return best


## « Expérience : 4/10 (Éprouvé) ».
static func summary(unit: Dictionary) -> String:
	var level := level_of(unit)
	return "Expérience : %d/%d (%s)" % [level, max_level(), tier_name(level)]


## Vide le cache (tests).
static func reload() -> void:
	_lookup.reload()
