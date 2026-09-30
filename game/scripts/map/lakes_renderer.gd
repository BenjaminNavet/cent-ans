class_name LakesRenderer
extends MeshInstance3D

## Lot SS3 (ADR 0141 §4) : nappes d'eau des lacs de la carte de campagne, rendu seulement.
##
## Contours et niveaux lus dans `data/map/lakes.json` (`cent-ans geo lakes`) : chaque lac est un
## polygone triangulé (`Geometry2D.triangulate_polygon`), élargi de `shore_overlap_px` pour
## passer sous la berge (pas de fissure entre l'eau et la rive), posé à son niveau × HEIGHT_SCALE.
## Matériau : `river_water.gdshader` en mode « nappe » (`sheet`), qui suit comme les fleuves
## l'échelle verticale affichée (`campaign_display_height`, ZG8) sans reconstruction.
## Visible en vue 3D seulement (masqué quand le parchemin l'emporte, `ZoomTiers.strategic_weight`).
## L'eau peinte par `terrain.gdshader` reste dessous (lacs trop petits, repli).

const WATER_SHADER := preload("res://shaders/river_water.gdshader")
const LAKES_FILE := "lakes.json"

## Débord du polygone sous la rive (px carte).
@export var shore_overlap_px: float = 0.5
## Opacité de la nappe (l'eau peinte du terrain transparaît un peu).
@export var sheet_alpha: float = 0.88

## Lacs chargés : {id, name, level_m, center: Vector2, polygon: PackedVector2Array, triangles}.
var lakes: Array[Dictionary] = []
var stats: Dictionary = {}
var map_data: MapData
var water_material: ShaderMaterial


func build(data: MapData) -> void:
	var t0 := Time.get_ticks_msec()
	map_data = data
	lakes.clear()
	_load_lakes(data.map_dir.path_join(LAKES_FILE))
	var vertices := PackedVector3Array()
	var indices := PackedInt32Array()
	var failed := 0
	for lake: Dictionary in lakes:
		var polygon := _overlapped(lake["polygon"])
		var triangles := Geometry2D.triangulate_polygon(polygon)
		if triangles.is_empty():
			# Contour élargi auto-intersecté (rare) : on retente sur le contour d'origine.
			polygon = lake["polygon"]
			triangles = Geometry2D.triangulate_polygon(polygon)
		if triangles.is_empty():
			failed += 1
			continue
		var base := vertices.size()
		# Hauteur cuite à HEIGHT_SCALE sans relief exagéré ; le shader pose la hauteur affichée.
		var y := float(lake["level_m"]) * MapData.HEIGHT_SCALE
		for p in polygon:
			vertices.append(Vector3(p.x, y, p.y))
		for index in triangles:
			indices.append(base + index)
		lake["triangles"] = triangles.size() / 3
	mesh = null
	if not vertices.is_empty():
		var normals := PackedVector3Array()
		normals.resize(vertices.size())
		normals.fill(Vector3.UP)
		var arrays := []
		arrays.resize(Mesh.ARRAY_MAX)
		arrays[Mesh.ARRAY_VERTEX] = vertices
		arrays[Mesh.ARRAY_NORMAL] = normals
		arrays[Mesh.ARRAY_INDEX] = indices
		var result := ArrayMesh.new()
		result.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
		# Le shader relève les sommets (échelle verticale dynamique, décalage selon la distance).
		result.custom_aabb = _grown_aabb(vertices)
		mesh = result
	water_material = _water_material()
	material_override = water_material
	cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	stats = {
		"lakes": lakes.size(),
		"meshed": lakes.size() - failed,
		"failed": failed,
		"vertices": vertices.size(),
		"triangles": indices.size() / 3,
		"build_ms": Time.get_ticks_msec() - t0,
	}
	print("LakesRenderer: %s" % JSON.stringify(stats))


## Vue 3D seulement : masqué dès que le parchemin l'emporte (fondu croisé DV).
func update_view(strategic_weight: float) -> void:
	var show := mesh != null and strategic_weight < 0.5
	if visible != show:
		visible = show


## Lac par nom (premier trouvé) ; dictionnaire vide sinon.
func lake_named(lake_name: String) -> Dictionary:
	for lake in lakes:
		if lake["name"] == lake_name:
			return lake
	return {}


func _load_lakes(path: String) -> void:
	if not FileAccess.file_exists(path):
		return
	var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(path))
	if not parsed is Dictionary:
		push_warning("LakesRenderer: %s illisible" % path)
		return
	for entry: Dictionary in parsed.get("lakes", []):
		var coords: Array = entry.get("polygon_px", [])
		if coords.size() < 3:
			continue
		var polygon := PackedVector2Array()
		polygon.resize(coords.size())
		for i in coords.size():
			polygon[i] = Vector2(float(coords[i][0]), float(coords[i][1]))
		var center: Array = entry.get("center_px", [0, 0])
		lakes.append({
			"id": str(entry.get("id", "")),
			"name": str(entry.get("name", "")),
			"level_m": float(entry.get("level_m", 0.0)),
			"center": Vector2(float(center[0]), float(center[1])),
			"polygon": polygon,
			"triangles": 0,
		})


## Contour élargi de `shore_overlap_px` (plus grand morceau si l'élargissement en rend plusieurs).
func _overlapped(polygon: PackedVector2Array) -> PackedVector2Array:
	if shore_overlap_px <= 0.0:
		return polygon
	var grown := Geometry2D.offset_polygon(polygon, shore_overlap_px, Geometry2D.JOIN_MITER)
	var best := polygon
	var best_area := -1.0
	for candidate: PackedVector2Array in grown:
		var area := absf(_signed_area(candidate))
		if area > best_area:
			best_area = area
			best = candidate
	return best


static func _signed_area(polygon: PackedVector2Array) -> float:
	var area := 0.0
	for i in polygon.size():
		var a := polygon[i]
		var b := polygon[(i + 1) % polygon.size()]
		area += a.x * b.y - b.x * a.y
	return area * 0.5


static func _grown_aabb(vertices: PackedVector3Array) -> AABB:
	var box := AABB(vertices[0], Vector3.ZERO)
	for v in vertices:
		box = box.expand(v)
	# Relief exagéré (jusqu'à ×4,3) et décalage selon la distance : marge verticale large.
	return AABB(box.position - Vector3.ONE, box.size + Vector3(2.0, box.end.y * 4.0 + 8.0, 2.0))


func _water_material() -> ShaderMaterial:
	var material := ShaderMaterial.new()
	material.shader = WATER_SHADER
	material.set_shader_parameter("sheet", true)
	material.set_shader_parameter("sheet_alpha", sheet_alpha)
	material.set_shader_parameter("map_size", Vector2(map_data.size))
	material.set_shader_parameter("baked_vertical_scale", MapData.HEIGHT_SCALE)
	material.render_priority = 1
	return material
