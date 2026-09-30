class_name SeaLaneLayer
extends Node3D

## Lot SL1 (ADR 0117) : routes maritimes tracées sur la mer, en tirets à l'encre comme sur un
## portulan. Géométrie : `data/map/sea_lanes_px.json` (sortie de `cent-ans geo sea-lanes`,
## pixels carte = coordonnées monde X/Z) ; état du tour : `CampaignSim.get_sea_lanes()` (mer,
## maîtrise, blocus, gros temps, routes commerciales). Rendu seulement : les règles vivent dans
## `sim-campaign::sea_lanes`.
##
## Trois maillages, un par encre : bleu nuit (mer libre), bleu profond (mer tenue par la
## faction joueuse ou un allié non hostile), rouge sang (mer tenue par un ennemi). La haute mer
## est en tirets longs, le cabotage en tirets courts. Couche masquée aux paliers vallée / site
## comme les routes commerciales ; infobulle de la route survolée (`nearest_lane`).

const LANES_FILE := "sea_lanes_px.json"
## Largeur monde des rubans (unités carte ≈ 719 m) ; la largeur écran minimale vient du shader.
const WIDTH := 1.6
const LIFT := 0.4
const INK := Color(0.12, 0.20, 0.33, 0.8)
const INK_OWN := Color(0.05, 0.22, 0.45, 0.9)
const INK_HOSTILE := Color(0.55, 0.10, 0.07, 0.9)
## Tirets en pixels écran : haute mer longue, cabotage court.
const DASH_OPEN := 16.0
const DASH_COASTAL := 8.0
const SHADER := preload("res://shaders/sea_lane.gdshader")

var map_data: MapData
## id -> PackedVector2Array (pixels carte), dans l'ordre `from` -> `to` du catalogue.
var _points: Dictionary = {}
## id -> {from, to}
var _ends: Dictionary = {}
## id -> dictionnaire `get_sea_lanes()` du tour.
var _state: Dictionary = {}
## « style » -> MeshInstance3D (neutral_open, neutral_coastal, own_*, hostile_*).
var _meshes: Dictionary = {}
var _close_hidden := false


func setup(data: MapData) -> void:
	map_data = data
	_load_geometry()


func has_lanes() -> bool:
	return not _points.is_empty()


func _load_geometry() -> void:
	_points.clear()
	_ends.clear()
	if map_data == null:
		return
	var path := map_data.map_dir.path_join(LANES_FILE)
	if not FileAccess.file_exists(path):
		return
	var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(path))
	if not (parsed is Dictionary):
		push_warning("SeaLaneLayer : %s illisible" % path)
		return
	for lane_variant in (parsed as Dictionary).get("lanes", []):
		var lane: Dictionary = lane_variant
		var points := PackedVector2Array()
		for p in lane.get("points", []):
			points.append(Vector2(float(p[0]), float(p[1])))
		if points.size() < 2:
			continue
		var id := str(lane.get("id", ""))
		_points[id] = points
		_ends[id] = {"from": str(lane.get("from", "")), "to": str(lane.get("to", ""))}


## Tracé de la route entre deux ports, orienté de `from` vers `to` ; vide si aucune route
## ne les relie (courts passages du graphe : trait droit laissé à l'appelant).
func lane_points(from: String, to: String) -> PackedVector2Array:
	for id in _ends:
		var ends: Dictionary = _ends[id]
		if ends["from"] == from and ends["to"] == to:
			return _points[id]
		if ends["from"] == to and ends["to"] == from:
			var reversed: PackedVector2Array = (_points[id] as PackedVector2Array).duplicate()
			reversed.reverse()
			return reversed
	return PackedVector2Array()


## `lanes` : tableau `CampaignSim.get_sea_lanes()`. `player_faction` : pour l'encre des mers
## tenues par la faction joueuse.
func refresh(lanes: Array, player_faction: String) -> void:
	_state.clear()
	var groups: Dictionary = {}
	for lane_variant in lanes:
		var lane: Dictionary = lane_variant
		var id := str(lane.get("id", ""))
		if not _points.has(id):
			continue
		_state[id] = lane
		var holder := "neutral"
		if bool(lane.get("hostile", false)):
			holder = "hostile"
		elif str(lane.get("control_faction", "")) == player_faction and player_faction != "":
			holder = "own"
		var style := "%s_%s" % [holder, "open" if str(lane.get("kind", "")) == "open_sea" else "coastal"]
		if not groups.has(style):
			groups[style] = []
		(groups[style] as Array).append(id)
	for style in _meshes:
		(_meshes[style] as MeshInstance3D).mesh = null
	for style in groups:
		_mesh_for(style).mesh = _build(groups[style])
	_apply_visible()


## Masque la couche aux paliers vallée / site (appelé par la carte à chaque changement de zoom).
func set_close_hidden(value: bool) -> void:
	if value != _close_hidden:
		_close_hidden = value
		_apply_visible()


func _apply_visible() -> void:
	visible = not _close_hidden


func _mesh_for(style: String) -> MeshInstance3D:
	if _meshes.has(style):
		return _meshes[style]
	var instance := MeshInstance3D.new()
	instance.name = "Lanes_" + style
	instance.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	var material := ShaderMaterial.new()
	material.shader = SHADER
	var holder := style.get_slice("_", 0)
	var ink := INK_HOSTILE if holder == "hostile" else (INK_OWN if holder == "own" else INK)
	material.set_shader_parameter("color", ink)
	material.set_shader_parameter("min_px", 2.2 if holder == "hostile" else 1.6)
	material.set_shader_parameter("dash_px", DASH_OPEN if style.ends_with("open") else DASH_COASTAL)
	material.render_priority = 1
	instance.material_override = material
	add_child(instance)
	_meshes[style] = instance
	return instance


## Rubans « ligne médiane + perpendiculaire » (comme `PolylineMesh.build_screen_lines`) avec
## l'abscisse curviligne monde en UV2.x pour les tirets du shader.
func _build(ids: Array) -> ArrayMesh:
	var vertices := PackedVector3Array()
	var normals := PackedVector3Array()
	var uvs := PackedVector2Array()
	var uv2s := PackedVector2Array()
	var indices := PackedInt32Array()
	for id in ids:
		var points: PackedVector2Array = _points[id]
		var base := vertices.size()
		var count := points.size()
		var arc := 0.0
		for i in count:
			var p := points[i]
			if i > 0:
				arc += p.distance_to(points[i - 1])
			var dir := (points[mini(i + 1, count - 1)] - points[maxi(i - 1, 0)]).normalized()
			if dir == Vector2.ZERO:
				dir = Vector2.RIGHT
			var perp := Vector3(-dir.y, 0.0, dir.x)
			var center := Vector3(p.x, map_data.surface_world_at(p.x, p.y) + LIFT, p.y)
			for side: float in [1.0, -1.0]:
				vertices.append(center)
				normals.append(perp * side)
				uvs.append(Vector2(WIDTH, side))
				uv2s.append(Vector2(arc, 0.0))
		for i in count - 1:
			var a := base + i * 2
			indices.append_array([a, a + 1, a + 3, a, a + 3, a + 2])
	var mesh := ArrayMesh.new()
	if vertices.is_empty():
		return mesh
	var arrays := []
	arrays.resize(Mesh.ARRAY_MAX)
	arrays[Mesh.ARRAY_VERTEX] = vertices
	arrays[Mesh.ARRAY_NORMAL] = normals
	arrays[Mesh.ARRAY_TEX_UV] = uvs
	arrays[Mesh.ARRAY_TEX_UV2] = uv2s
	arrays[Mesh.ARRAY_INDEX] = indices
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	return mesh


## Route la plus proche de `world_xz` sous `max_distance` (unités carte) : dictionnaire
## `get_sea_lanes()` de la route, `{}` si aucune.
func nearest_lane(world_xz: Vector2, max_distance: float) -> Dictionary:
	if not visible:
		return {}
	var best: Dictionary = {}
	var best_dist := max_distance
	for id in _state:
		var points: PackedVector2Array = _points[id]
		for i in points.size() - 1:
			var d := TradeRouteLayer._distance_to_segment(world_xz, points[i], points[i + 1])
			if d < best_dist:
				best_dist = d
				best = _state[id]
	return best


## Infobulle d'une route (dictionnaire `get_sea_lanes()`).
static func tooltip(lane: Dictionary) -> String:
	var lines := PackedStringArray()
	lines.append("%s : %s ↔ %s" % [lane.get("name", ""), lane.get("from_name", ""), lane.get("to_name", "")])
	lines.append("%s, %s, %d km — une saison de traversée" % [
		str(lane.get("kind_name", "")).capitalize(),
		lane.get("sea_name", ""),
		int(round(float(lane.get("length_km", 0.0)))),
	])
	var level := int(lane.get("control_level", 0))
	if level > 0:
		var holder := str(lane.get("control_name", ""))
		if bool(lane.get("hostile", false)):
			var threat := "blocus, commerce coupé" if bool(lane.get("blockade", false)) else "traversées et commerce menacés"
			lines.append("Mer tenue par l'ennemi (%s, %d %%) : %s" % [holder, level, threat])
		else:
			lines.append("Mer tenue par %s (%d %%)" % [holder, level])
	var storm := float(lane.get("storm_loss_percent", 0.0))
	if storm > 0.0:
		lines.append("Gros temps : ~%d %% des hommes perdus en traversée cette saison" % int(ceil(storm)))
	var trade := float(lane.get("trade_season_factor", 1.0))
	if trade < 0.99:
		lines.append("Commerce réduit par la saison (×%.2f)" % trade)
	var routes: PackedStringArray = lane.get("trade_routes", PackedStringArray())
	if not routes.is_empty():
		lines.append("Routes commerciales : %d" % routes.size())
	return "\n".join(lines)
