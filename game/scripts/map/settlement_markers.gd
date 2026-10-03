class_name SettlementMarkers
extends RefCounted

## Lot DA3 (ADR 0066), refondu au lot DV2 (ADR 0124) : rangs, tailles et densité des lieux de la
## carte de campagne. Lecture seule de `data/map/settlement_markers.json` : rang (1 à 4) déduit
## des données statiques des colonies (type, poids, fortification), taille écran de l'écu par
## rang, distance de retrait par rang (densité des noms), dé-encombrement écran. Plus de marqueur
## peint : la vue normale montre maquette + nom + écu, le parchemin des vignettes à l'encre.
## L'atlas des écus vit dans `HeraldryAtlas`. Aucune règle de jeu.

const FILE_NAME := "settlement_markers.json"

var catalog: Dictionary = {}


## Catalogue du dossier de données courant (`MapPaths`, variable `CENT_ANS_DATA_DIR` comprise),
## repli sur `data/` du dépôt (jeux d'essai sans ce fichier).
static func load_default() -> SettlementMarkers:
	# Script chargé à l'exécution : l'autoload `MapPaths` est inconnu des tests `--script`.
	var paths: GDScript = load("res://scripts/map/map_paths.gd")
	var path: String = str(paths.call("default_data_dir")).path_join("map").path_join(FILE_NAME)
	if not FileAccess.file_exists(path):
		path = str(paths.call("project_root")).path_join("data/map").path_join(FILE_NAME)
	return load_from(path)


static func load_from(path: String) -> SettlementMarkers:
	var result := SettlementMarkers.new()
	if FileAccess.file_exists(path):
		var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(path))
		if parsed is Dictionary:
			result.catalog = parsed
	return result


func is_valid() -> bool:
	return not catalog.is_empty()


## Rang (1 à 4) d'une colonie : première règle de `rank_rules` qui s'applique.
func rank_of(entry: Dictionary) -> int:
	var id := str(entry.get("id", ""))
	var kind := str(entry.get("kind", ""))
	for rule: Dictionary in catalog.get("rank_rules", []):
		if rule.has("ids") and not (rule["ids"] as Array).has(id):
			continue
		if rule.has("kind") and str(rule["kind"]) != kind:
			continue
		if rule.has("min_fortification") and int(entry.get("fortification_level", 0)) < int(rule["min_fortification"]):
			continue
		if rule.has("min_weight") and int(entry.get("weight", 0)) < int(rule["min_weight"]):
			continue
		return int(rule.get("rank", 1))
	return int(catalog.get("default_rank", 1))


## Taille écran (px) d'un marqueur selon le rang et le type.
func size_px(kind: String, rank: int) -> float:
	var sizes: Dictionary = catalog.get("size_px", {})
	var size := float(sizes.get(str(rank), sizes.get("1", 24.0)))
	return size * float(catalog.get("kind_size_factor", {}).get(kind, 1.0))


## Distance caméra jusqu'à laquelle le marqueur reste affiché (dé-encombrement) : la plus
## grande `max_distance` des paliers où le rang atteint le minimum de son type ; 0 = jamais.
func visible_until(kind: String, rank: int) -> float:
	var result := 0.0
	for tier: Dictionary in catalog.get("visibility", []):
		var minimum: Dictionary = tier.get("min_rank", {})
		if minimum.has(kind) and rank >= int(minimum[kind]):
			result = maxf(result, float(tier.get("max_distance", 0.0)))
	return result


func fade_distance() -> float:
	return float(catalog.get("fade_distance", 60.0))


## Lot DA7d : paramètre du dé-encombrement écran (bloc `declutter` du catalogue).
func declutter_value(key: String, default: Variant) -> Variant:
	return catalog.get("declutter", {}).get(key, default)


## Lot DV2 : écu au-dessus du nom : taille (fraction de `size_px`) et écart (px) au texte.
func shield_size_factor() -> float:
	return float(catalog.get("shield", {}).get("size_factor", 0.45))


## Lot TB2 : distance caméra jusqu'à laquelle un lieu de rang `rank` porte son écu
## (`shield.max_distance_by_rank`) ; au-delà, le nom seul. Rang absent : écu à toute distance.
func shield_until(rank: int) -> float:
	var limits: Dictionary = catalog.get("shield", {}).get("max_distance_by_rank", {})
	return float(limits.get(str(rank), 100000.0))


func shield_gap_px() -> float:
	return float(catalog.get("shield", {}).get("gap_px", 1.0))


## Lot RJ-c (ADR 0175) : réglage `key` de la bannière possesseur / occupant (`banner`).
func banner_value(key: String, default: Variant) -> Variant:
	return catalog.get("banner", {}).get(key, default)
