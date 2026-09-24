class_name SettlementData
extends RefCounted

## Données de rendu des colonies, hameaux et routes (lot C6). Lecture seule, aucune règle :
## - `data/settlements/<province>.json` : id, province, kind, nom (statique) ;
## - `data/map/settlements_px.json` : position de jeu (pixels carte 4096) ;
## - `data/map/hamlets.json` : hameaux décoratifs `{name, px, province}` ;
## - `data/map/roads.geojson` : routes `LineString` en pixels carte (`type` main / secondary /
##   computed).
## `apply_live(sim)` met à jour contrôleur / propriétaire depuis `CampaignSim.settlements()`
## (repli : propriétaire de la province dans `data/`). Tous les fichiers sont facultatifs.

const KINDS: Array[String] = ["city", "town", "castle", "abbey", "village"]
## Priorité d'étiquette (plus petit = prioritaire) : cité > ville > autres.
const LABEL_PRIORITY := {"city": 0, "town": 1, "castle": 2, "abbey": 3, "village": 4}

## Colonies triées par priorité puis id : {id, province, kind, name, px: Vector2, controller,
## owner, fortification_level, port}.
var settlements: Array[Dictionary] = []
var index_by_id: Dictionary = {}
var hamlets: Array[Dictionary] = []  # {name, px: Vector2, province}
var roads: Array[Dictionary] = []  # {type, main: bool, points: PackedVector2Array}
var load_ms: int = 0


static func load_from(data_dir: String, map_dir: String) -> SettlementData:
	var result := SettlementData.new()
	var t0 := Time.get_ticks_msec()
	result._load_settlements(data_dir, map_dir)
	result._load_hamlets(map_dir)
	result._load_roads(map_dir)
	result.load_ms = Time.get_ticks_msec() - t0
	return result


static func _read_json(path: String) -> Variant:
	if not FileAccess.file_exists(path):
		return null
	return JSON.parse_string(FileAccess.get_file_as_string(path))


func _load_settlements(data_dir: String, map_dir: String) -> void:
	var positions: Variant = _read_json(map_dir.path_join("settlements_px.json"))
	if not (positions is Dictionary):
		return
	var owners := {}
	var provinces_dir := data_dir.path_join("provinces")
	var settlements_dir := data_dir.path_join("settlements")
	var dir := DirAccess.open(settlements_dir)
	if dir == null:
		return
	for file_name in dir.get_files():
		if not file_name.ends_with(".json") or not file_name.begins_with("prov_"):
			continue
		var entries: Variant = _read_json(settlements_dir.path_join(file_name))
		if not (entries is Array):
			continue
		for entry in entries:
			if not (entry is Dictionary):
				continue
			var id := str(entry.get("id", ""))
			var px: Variant = positions.get(id)
			if id == "" or not (px is Array) or (px as Array).size() < 2:
				continue
			var province := str(entry.get("province", ""))
			if not owners.has(province):
				var province_data: Variant = _read_json(provinces_dir.path_join(province + ".json"))
				owners[province] = str(province_data.get("owner", "")) if province_data is Dictionary else ""
			var owner: Variant = entry.get("owner")
			var owner_id: String = str(owner) if owner != null else str(owners[province])
			var name_data: Variant = entry.get("name", {})
			var display := str(name_data.get("display", id)) if name_data is Dictionary else str(name_data)
			settlements.append({
				"id": id,
				"province": province,
				"kind": str(entry.get("kind", "village")),
				"name": display,
				"px": Vector2(float(px[0]), float(px[1])),
				"owner": owner_id,
				"controller": owner_id,
				"fortification_level": int(entry.get("fortification_level", 0)),
				"port": bool(entry.get("port", false)),
			})
	settlements.sort_custom(func(a: Dictionary, b: Dictionary) -> bool:
		var pa: int = LABEL_PRIORITY.get(a["kind"], 9)
		var pb: int = LABEL_PRIORITY.get(b["kind"], 9)
		return pa < pb if pa != pb else str(a["id"]) < str(b["id"]))
	for i in settlements.size():
		index_by_id[settlements[i]["id"]] = i


func _load_hamlets(map_dir: String) -> void:
	var entries: Variant = _read_json(map_dir.path_join("hamlets.json"))
	if not (entries is Array):
		return
	for entry in entries:
		if entry is Dictionary and entry.get("px") is Array:
			var px: Array = entry["px"]
			hamlets.append({"name": str(entry.get("name", "")), "px": Vector2(float(px[0]), float(px[1])), "province": str(entry.get("province", ""))})


func _load_roads(map_dir: String) -> void:
	var collection: Variant = _read_json(map_dir.path_join("roads.geojson"))
	if not (collection is Dictionary):
		return
	for feature in collection.get("features", []):
		var geometry: Dictionary = feature.get("geometry", {})
		var properties: Dictionary = feature.get("properties", {})
		var road_type := str(properties.get("type", "secondary"))
		var lines: Array = []
		match str(geometry.get("type", "")):
			"LineString":
				lines = [geometry.get("coordinates", [])]
			"MultiLineString":
				lines = geometry.get("coordinates", [])
		for coords in lines:
			var points := PackedVector2Array()
			for c in coords:
				points.append(Vector2(float(c[0]), float(c[1])))
			if points.size() >= 2:
				roads.append({"type": road_type, "main": road_type == "main", "points": points})


## Contrôleur et propriétaire courants depuis la simulation (`CampaignSim.settlements()`).
## Renvoie vrai si au moins une colonie a changé.
func apply_live(sim: Object) -> bool:
	if sim == null or not sim.has_method("settlements"):
		return false
	var changed := false
	for live in sim.call("settlements"):
		if not (live is Dictionary):
			continue
		var index: int = index_by_id.get(str(live.get("id", "")), -1)
		if index < 0:
			continue
		var entry: Dictionary = settlements[index]
		for key in ["controller", "owner"]:
			var value := str(live.get(key, entry[key]))
			if value != entry[key]:
				entry[key] = value
				changed = true
		if live.has("name"):
			entry["name"] = str(live["name"])
	return changed


func get_settlement(id: String) -> Dictionary:
	var index: int = index_by_id.get(id, -1)
	return settlements[index] if index >= 0 else {}
