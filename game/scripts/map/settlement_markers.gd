class_name SettlementMarkers
extends RefCounted

## Lot DA3 (ADR 0066) : langage unique des marqueurs de lieux (bible DA § 8). Lecture seule de
## `data/map/settlement_markers.json` : forme = type (pictogramme peint de l'atlas), écu =
## détenteur (armoiries de la faction, atlas d'écus composé ici depuis
## `PortraitLoader.heraldry_texture`), taille = rang. Aucune règle de jeu : le rang se déduit des
## données statiques des colonies (type, poids, fortification) par les règles du catalogue.

const FILE_NAME := "settlement_markers.json"
## Taille d'une case de l'atlas d'écus (px) et nombre de colonnes.
const SHIELD_CELL := 64
const SHIELD_COLUMNS := 8

var catalog: Dictionary = {}
## id de pictogramme → case de l'atlas.
var cells: Dictionary = {}
var atlas_columns: int = 4
var atlas_rows: int = 3
var atlas: Texture2D
## Atlas des écus de faction (construit à la demande) et index par faction.
var shield_atlas: Texture2D
var shield_index: Dictionary = {}
var shield_columns: int = SHIELD_COLUMNS
var shield_rows: int = 1


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
	result._index()
	return result


func is_valid() -> bool:
	return not catalog.is_empty()


func _index() -> void:
	for pictogram in catalog.get("pictograms", []):
		cells[str(pictogram.get("id", ""))] = int(pictogram.get("cell", 0))
	var atlas_data: Dictionary = catalog.get("atlas", {})
	atlas_columns = maxi(int(atlas_data.get("columns", 4)), 1)
	var max_cell := 0
	for cell in cells.values():
		max_cell = maxi(max_cell, int(cell))
	atlas_rows = max_cell / atlas_columns + 1
	var path := str(atlas_data.get("path", ""))
	if path != "":
		atlas = PortraitLoader.load_texture(path)


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


## Pictogramme d'un type à un rang (rang absent : le rang inférieur le plus proche).
func pictogram_for(kind: String, rank: int) -> String:
	var by_rank: Dictionary = catalog.get("kinds", {}).get(kind, {})
	for r in range(rank, 0, -1):
		if by_rank.has(str(r)):
			return str(by_rank[str(r)])
	for r in range(rank + 1, 5):
		if by_rank.has(str(r)):
			return str(by_rank[str(r)])
	return ""


func cell_of(pictogram_id: String) -> int:
	return int(cells.get(pictogram_id, -1))


## Case de l'insigne de port, -1 sans insigne.
func port_cell() -> int:
	return cell_of(str(catalog.get("port_badge", "")))


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


## Placement (centre, demi-taille en fraction du quad) de l'écu ou de l'insigne.
func placement(key: String) -> Vector3:
	var data: Dictionary = catalog.get(key, {})
	var center: Array = data.get("center", [0.5, 0.5])
	return Vector3(float(center[0]), float(center[1]), float(data.get("half_size", 0.2)))


## Pictogramme et libellé par type pour la légende : [{kind, rank, id, label, cell}].
func legend_entries() -> Array[Dictionary]:
	var labels := {}
	for pictogram in catalog.get("pictograms", []):
		labels[str(pictogram.get("id", ""))] = str(pictogram.get("label", ""))
	var result: Array[Dictionary] = []
	for kind in catalog.get("kinds", {}):
		var by_rank: Dictionary = catalog["kinds"][kind]
		for rank in by_rank:
			var id := str(by_rank[rank])
			result.append({"kind": kind, "rank": int(rank), "id": id, "label": labels.get(id, id), "cell": cell_of(id)})
	return result


## Atlas des écus des factions données (ordre stable) ; `shield_index[faction]` = case.
## Une faction sans écu garde la case -1 (le shader n'en dessine pas).
func build_shield_atlas(factions: Array) -> void:
	shield_index.clear()
	var images: Array[Image] = []
	for faction in factions:
		var id := str(faction)
		if id == "" or shield_index.has(id):
			continue
		var texture := PortraitLoader.heraldry_texture(id)
		if texture == null:
			continue
		var image := texture.get_image()
		if image == null or image.is_empty():
			continue
		image = image.duplicate() as Image
		if image.is_compressed():
			image.decompress()
		image.convert(Image.FORMAT_RGBA8)
		image.resize(SHIELD_CELL, SHIELD_CELL, Image.INTERPOLATE_LANCZOS)
		shield_index[id] = images.size()
		images.append(image)
	shield_rows = maxi((images.size() + shield_columns - 1) / shield_columns, 1)
	var sheet := Image.create(shield_columns * SHIELD_CELL, shield_rows * SHIELD_CELL, true, Image.FORMAT_RGBA8)
	for i in images.size():
		sheet.blit_rect(images[i], Rect2i(0, 0, SHIELD_CELL, SHIELD_CELL), Vector2i((i % shield_columns) * SHIELD_CELL, (i / shield_columns) * SHIELD_CELL))
	sheet.generate_mipmaps()
	shield_atlas = ImageTexture.create_from_image(sheet)


func shield_of(faction: String) -> int:
	return int(shield_index.get(faction, -1))


## Rectangle (UV 0-1) d'une case de l'atlas des pictogrammes.
func cell_uv_rect(cell: int) -> Rect2:
	var size := Vector2(1.0 / atlas_columns, 1.0 / atlas_rows)
	return Rect2(Vector2(cell % atlas_columns, cell / atlas_columns) * size, size)


## Rectangle (UV 0-1) d'une case de l'atlas des écus.
func shield_uv_rect(index: int) -> Rect2:
	var size := Vector2(1.0 / shield_columns, 1.0 / shield_rows)
	return Rect2(Vector2(index % shield_columns, index / shield_columns) * size, size)
