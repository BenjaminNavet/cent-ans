class_name RiversRenderer
extends MeshInstance3D

## Fleuves de la carte de campagne (lot V4, A1-11), rendu seulement.
##
## - Eau : rubans `river_water.gdshader` le long des tronçons de `data/map/rivers_render.json`
##   (orientés vers l'aval, largeur selon le fleuve et l'altitude, `cent-ans geo rivers-render`),
##   avec test de profondeur (murs, ponts et collines cachent l'eau), écoulement et reflets ;
##   jamais moins de `major_min_px` / `minor_min_px` pixels écran (estompés s'ils sont plus fins).
##   Les cours d'eau mineurs (maillage enfant) n'apparaissent qu'en vue rapprochée.
## - Lit creusé : `terrain.gdshader` lit `river_bed.png` ; ce nœud lui indique chaque image les
##   tuiles de relief fin où creuser (`carved_rects`, partagé avec l'eau).
## - Colonies : l'eau passe *sous* les colonies (tronçons coupés dans l'emprise des maquettes, lit
##   effacé) ; ponts-portes aux murs (`RiverCrossings`).
## - Zones personnalisées (`river_styles.json` → `custom_zones`, ex. Paris pour le lot L1) : rien
##   n'est dessiné dedans ; voir `docs/wip/v4-fleuves-forets.md`.
## Repli sans `rivers_render.json` : rubans depuis `MapData.rivers` (largeur selon l'importance).

const MAJOR_IMPORTANCE := 3
const WATER_SHADER := preload("res://shaders/river_water.gdshader")
const RENDER_FILE := "rivers_render.json"
## Largeur (px carte) au-delà de laquelle un cours d'eau est « majeur » même d'importance faible.
const MAJOR_WIDTH := 0.5
## Part du rayon d'une maquette de colonie couverte par la ville (l'eau passe dessous).
const SETTLEMENT_COVER := 0.8

@export var base_width: float = 0.1
@export var width_per_importance: float = 0.1
@export var major_min_px: float = 1.6
@export var minor_min_px: float = 1.0
## Distance caméra au-delà de laquelle les rivières mineures sont masquées (réglée par la scène).
@export var minor_max_distance: float = 1500.0

var map_data: MapData
var terrain: TerrainBuilder
var crossings: RiverCrossings
## Lot ZG5b : hydrographie fine, routes drapées et ancrages au palier près (null sans cache).
var fine: FineGeoLayer
## Tronçons affichés : {name, importance, points: PackedVector2Array, widths: PackedFloat32Array}.
var rivers: Array[Dictionary] = []
## Zones personnalisées : {id, name, px: Vector2, radius_px, boundary_bridges}.
var zones: Array[Dictionary] = []
## Emprises couvertes par les colonies : Vector4(x, y, rayon couvert, index de colonie).
var covers: Array[Vector4] = []
var bank_px: float = 1.1
## Par emprise (même ordre que `covers`) : segments proches, PackedInt32Array de
## (index de tronçon << 16) | index du premier point.
var cover_segments: Array[PackedInt32Array] = []
## Grille des segments : Vector2i(case) → Array de codes (tronçon << 16) | premier point.
var segment_cells: Dictionary = {}
var stats: Dictionary = {}

var _minor: MeshInstance3D
var _minor_visible := true
var _water_major: ShaderMaterial
var _water_minor: ShaderMaterial
var _rects_dirty := true


## `settlements` (optionnel) : `SettlementLayer` construit, pour faire passer l'eau sous les
## colonies et poser les ponts-portes.
func build(data: MapData, terrain_builder: TerrainBuilder = null, settlements: SettlementLayer = null) -> void:
	var t0 := Time.get_ticks_msec()
	map_data = data
	terrain = terrain_builder
	_load_rivers()
	covers = _settlement_covers(settlements)
	_cover_cells.clear()
	var touched := _index_cover_segments()
	_clear_river_bed_under_covers()
	var major_lines := []
	var minor_lines := []
	for ri in rivers.size():
		var river: Dictionary = rivers[ri]
		for piece in _cut_by_covers(river["points"], river["widths"], touched.get(ri, [])):
			var major: bool = int(river["importance"]) >= MAJOR_IMPORTANCE or _max(piece[1]) >= MAJOR_WIDTH
			(major_lines if major else minor_lines).append(piece)
	mesh = _build_mesh(major_lines)
	_water_major = _water_material(major_min_px)
	material_override = _water_major
	if _minor != null:
		_minor.queue_free()
	_minor = MeshInstance3D.new()
	_minor.name = "MinorRivers"
	_minor.mesh = _build_mesh(minor_lines)
	_water_minor = _water_material(minor_min_px)
	_minor.material_override = _water_minor
	_minor.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	add_child(_minor)
	cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	if crossings != null:
		crossings.queue_free()
	crossings = RiverCrossings.new()
	crossings.name = "Crossings"
	add_child(crossings)
	crossings.build(self, settlements)
	_set_fords(crossings.ford_uniforms())
	if fine != null:
		fine.queue_free()
	fine = FineGeoLayer.new()
	fine.name = "FineGeo"
	add_child(fine)
	if not fine.setup(self, settlements):
		fine.queue_free()
		fine = null
	if terrain != null and not terrain.chunk_surface_changed.is_connected(_on_chunk_surface_changed):
		terrain.chunk_surface_changed.connect(_on_chunk_surface_changed)
	_rects_dirty = true
	stats = {
		"rivers": rivers.size(),
		"major_pieces": major_lines.size(),
		"minor_pieces": minor_lines.size(),
		"covers": covers.size(),
		"zones": zones.size(),
		"bridges": crossings.bridge_count(),
		"fine": fine != null,
		"build_ms": Time.get_ticks_msec() - t0,
	}
	print("RiversRenderer: %s" % JSON.stringify(stats))


## Zones personnalisées (lot L1) : [{id, name, px: Vector2, radius_px, boundary_bridges}].
func custom_zones() -> Array[Dictionary]:
	return zones


func in_custom_zone(p: Vector2, margin: float = 0.0) -> bool:
	for zone in zones:
		if p.distance_to(zone["px"]) < float(zone["radius_px"]) + margin:
			return true
	return false


func update_visibility(camera_distance: float) -> void:
	if _rects_dirty:
		_update_carved_rects()
	if fine != null:
		fine.update_view(camera_distance)
	var should_show := camera_distance < minor_max_distance
	if should_show == _minor_visible or _minor == null:
		return
	_minor_visible = should_show
	_minor.visible = should_show


func _on_chunk_surface_changed(_index: int) -> void:
	_rects_dirty = true


## Tuiles de relief fin : le terrain y creuse le lit et l'eau y descend sous la berge.
func _update_carved_rects() -> void:
	_rects_dirty = false
	if terrain == null or terrain.material == null:
		return
	# ZG5b : avec le réseau fin, le lit est creusé dans les pages du quadtree (FineBedCarver).
	var found := terrain.fine_chunk_rects() if fine == null else PackedVector4Array()
	var rects: Array[Vector4] = []
	for i in mini(found.size(), 4):
		rects.append(found[i])
	var count := rects.size()
	while rects.size() < 4:
		rects.append(Vector4.ZERO)
	for material: ShaderMaterial in [terrain.material, _water_major, _water_minor]:
		if material != null:
			material.set_shader_parameter("carved_rects", rects)
			material.set_shader_parameter("carved_rect_count", count)


# --- Données ------------------------------------------------------------------------------


func _load_rivers() -> void:
	rivers.clear()
	zones.clear()
	var path := map_data.map_dir.path_join(RENDER_FILE)
	var parsed: Variant = null
	if FileAccess.file_exists(path):
		parsed = JSON.parse_string(FileAccess.get_file_as_string(path))
	if parsed is Dictionary:
		bank_px = float(parsed.get("bank_px", 1.1))
		for zone: Dictionary in parsed.get("custom_zones", []):
			var center: Array = zone.get("px", [0, 0])
			zones.append({
				"id": str(zone.get("id", "")),
				"name": str(zone.get("name", "")),
				"px": Vector2(float(center[0]), float(center[1])),
				"radius_px": float(zone.get("radius_px", 0.0)),
				"boundary_bridges": bool(zone.get("boundary_bridges", true)),
			})
		for entry: Dictionary in parsed.get("rivers", []):
			var coords: Array = entry.get("points", [])
			var raw_widths: Array = entry.get("widths", [])
			var points := PackedVector2Array()
			points.resize(coords.size())
			var widths := PackedFloat32Array()
			widths.resize(coords.size())
			for i in coords.size():
				points[i] = Vector2(float(coords[i][0]), float(coords[i][1]))
				widths[i] = float(raw_widths[i]) if i < raw_widths.size() else 0.3
			rivers.append({"name": str(entry.get("name", "")), "importance": int(entry.get("importance", 0)), "points": points, "widths": widths})
		return
	# Repli : ancien rendu (largeur constante selon l'importance).
	for river in map_data.rivers:
		var points: PackedVector2Array = river["points"]
		var widths := PackedFloat32Array()
		widths.resize(points.size())
		widths.fill(base_width + width_per_importance * int(river["importance"]))
		rivers.append({"name": river["name"], "importance": river["importance"], "points": points, "widths": widths})


## Emprises couvertes par les villes (rayon réel) (hors zones personnalisées).
func _settlement_covers(settlements: SettlementLayer) -> Array[Vector4]:
	var result: Array[Vector4] = []
	if settlements == null or settlements.data == null:
		return result
	var exclusions := settlements.vegetation_exclusions()
	var count := mini(settlements.data.settlements.size(), exclusions.size())
	for i in count:
		var e := exclusions[i]
		# VT : `vegetation_exclusions` rend le finage ; l'emprise bâtie est le rayon réel de la ville.
		var built_radius := settlements.model_radius(i)
		if built_radius <= 0.3 or in_custom_zone(Vector2(e.x, e.y)):
			continue
		result.append(Vector4(e.x, e.y, built_radius * SETTLEMENT_COVER, i))
	return result


func _covered(p: Vector2) -> bool:
	for c in covers:
		if Vector2(c.x, c.y).distance_squared_to(p) < c.z * c.z:
			return true
	return false


## Index (dans `covers`) de l'emprise de colonie qui contient `p`, -1 sinon (grille de cases).
func cover_at(p: Vector2) -> int:
	if _cover_cells.is_empty() and not covers.is_empty():
		for k in covers.size():
			var c := covers[k]
			for cy in range(floori((c.y - c.z) / CELL_PX), floori((c.y + c.z) / CELL_PX) + 1):
				for cx in range(floori((c.x - c.z) / CELL_PX), floori((c.x + c.z) / CELL_PX) + 1):
					var key := Vector2i(cx, cy)
					if not _cover_cells.has(key):
						_cover_cells[key] = []
					(_cover_cells[key] as Array).append(k)
	for k: int in _cover_cells.get(Vector2i(floori(p.x / CELL_PX), floori(p.y / CELL_PX)), []):
		var c := covers[k]
		if Vector2(c.x, c.y).distance_squared_to(p) < c.z * c.z:
			return k
	return -1


var _cover_cells: Dictionary = {}


## Grille des segments (cases de `CELL_PX`) : pour chaque emprise, les segments proches
## (`cover_segments`) ; rend tronçon → emprises touchées (Array[Vector4]).
const CELL_PX := 16.0


func _index_cover_segments() -> Dictionary:
	cover_segments.clear()
	segment_cells.clear()
	var cells := segment_cells
	for ri in rivers.size():
		var points: PackedVector2Array = rivers[ri]["points"]
		for pi in points.size() - 1:
			var key := Vector2i(floori(points[pi].x / CELL_PX), floori(points[pi].y / CELL_PX))
			if not cells.has(key):
				cells[key] = []
			(cells[key] as Array).append((ri << 16) | pi)
	var touched: Dictionary = {}
	for c in covers:
		var found := PackedInt32Array()
		var reach := c.z + 3.0  # segments ≤ ~2 px : leur premier point est à moins de r + 2
		for cy in range(floori((c.y - reach) / CELL_PX), floori((c.y + reach) / CELL_PX) + 1):
			for cx in range(floori((c.x - reach) / CELL_PX), floori((c.x + reach) / CELL_PX) + 1):
				for code: int in cells.get(Vector2i(cx, cy), []):
					var ri: int = code >> 16
					var p: Vector2 = rivers[ri]["points"][code & 0xFFFF]
					if p.distance_squared_to(Vector2(c.x, c.y)) < reach * reach:
						found.append(code)
						if not touched.has(ri):
							touched[ri] = []
						if not (touched[ri] as Array).has(c):
							(touched[ri] as Array).append(c)
		cover_segments.append(found)
	return touched


## Coupe un tronçon dans les emprises des colonies (bouts recalés sur le cercle) : l'eau passe
## sous la ville. Rend [[points, largeurs], …].
func _cut_by_covers(points: PackedVector2Array, widths: PackedFloat32Array, near: Array) -> Array:
	if near.is_empty():
		return [[points, widths]]
	var saved := covers
	var local: Array[Vector4] = []
	local.assign(near)
	covers = local
	var pieces := []
	var cur_p := PackedVector2Array()
	var cur_w := PackedFloat32Array()
	for i in points.size():
		var inside := _covered(points[i])
		if not inside:
			if cur_p.is_empty() and i > 0:
				var entry := _circle_exit(points[i], points[i - 1])
				cur_p.append(entry)
				cur_w.append(widths[i])
			cur_p.append(points[i])
			cur_w.append(widths[i])
		elif not cur_p.is_empty():
			cur_p.append(_circle_exit(points[i - 1], points[i]))
			cur_w.append(widths[i - 1])
			if cur_p.size() >= 2:
				pieces.append([cur_p, cur_w])
			cur_p = PackedVector2Array()
			cur_w = PackedFloat32Array()
	if cur_p.size() >= 2:
		pieces.append([cur_p, cur_w])
	covers = saved
	return pieces


## Point du segment [outside → inside] sur le bord du cercle couvrant `inside` (dichotomie).
func _circle_exit(outside: Vector2, inside: Vector2) -> Vector2:
	var a := outside
	var b := inside
	for _i in 12:
		var m := (a + b) * 0.5
		if _covered(m):
			b = m
		else:
			a = m
	return a


## Efface le lit (texture du terrain) sous les emprises des colonies : pas de tranchée creusée
## entre les maisons.
func _clear_river_bed_under_covers() -> void:
	if terrain == null or covers.is_empty() or map_data.river_bed_image == null:
		return
	var texture := terrain.river_bed_texture()
	if texture == null:
		return
	var image := map_data.river_bed_image
	var changed := false
	for c in covers:
		var r := c.z
		for py in range(maxi(int(c.y - r) - 1, 0), mini(int(c.y + r) + 2, image.get_height())):
			for px in range(maxi(int(c.x - r) - 1, 0), mini(int(c.x + r) + 2, image.get_width())):
				var d := Vector2(px + 0.5, py + 0.5).distance_to(Vector2(c.x, c.y))
				if d < r:
					var value := image.get_pixel(px, py).r
					if value < 1.0:
						# Distance à la berge au moins égale à la distance au bord de l'emprise.
						var floor_value := (128.0 + (r - d) * 16.0) / 255.0
						if value < floor_value:
							image.set_pixel(px, py, Color(minf(floor_value, 1.0), 0, 0))
							changed = true
	if changed:
		texture.update(image)


static func _max(values: PackedFloat32Array) -> float:
	var m := 0.0
	for v in values:
		m = maxf(m, v)
	return m


# --- Maillage -----------------------------------------------------------------------------


## Rubans : chaque sommet est sur la ligne médiane (le shader l'écarte de la demi-largeur).
## NORMAL = perpendiculaire signée ; UV = (largeur, côté ±1) ; UV2 = (abscisse vers l'aval, 0).
func _build_mesh(pieces: Array) -> ArrayMesh:
	# OMR-R2 : tronçons calculés en parallèle (tableaux dimensionnés d'avance, décalage de sommets
	# connu par somme préfixe), puis concaténés dans l'ordre : même maillage qu'en série.
	var valid: Array = []
	for piece in pieces:
		if (piece[0] as PackedVector2Array).size() >= 2:
			valid.append(piece)
	var bases := PackedInt32Array()
	bases.resize(valid.size())
	var total := 0
	for n in valid.size():
		bases[n] = total
		total += (valid[n][0] as PackedVector2Array).size() * 2
	var parts: Array = []
	parts.resize(valid.size())
	var task := WorkerThreadPool.add_group_task(func(n: int) -> void:
		parts[n] = _piece_arrays(valid[n][0], valid[n][1], bases[n]), valid.size(), -1, true, "river meshes")
	WorkerThreadPool.wait_for_group_task_completion(task)
	var vertices := PackedVector3Array()
	var normals := PackedVector3Array()
	var uvs := PackedVector2Array()
	var uv2s := PackedVector2Array()
	var indices := PackedInt32Array()
	for part: Array in parts:
		vertices.append_array(part[0])
		normals.append_array(part[1])
		uvs.append_array(part[2])
		uv2s.append_array(part[3])
		indices.append_array(part[4])
	var result := ArrayMesh.new()
	if vertices.is_empty():
		return result
	var arrays := []
	arrays.resize(Mesh.ARRAY_MAX)
	arrays[Mesh.ARRAY_VERTEX] = vertices
	arrays[Mesh.ARRAY_NORMAL] = normals
	arrays[Mesh.ARRAY_TEX_UV] = uvs
	arrays[Mesh.ARRAY_TEX_UV2] = uv2s
	arrays[Mesh.ARRAY_INDEX] = indices
	result.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	# Le shader déplace les sommets (demi-largeur, élargissement écran, décalage vertical).
	result.custom_aabb = _grown_aabb(vertices)
	return result


## Sommets (deux par point : côté +1 puis −1), normales (perpendiculaire), UV (largeur, côté),
## UV2 (abscisse curviligne) et indices d'un tronçon dont le premier sommet est `base`.
func _piece_arrays(points: PackedVector2Array, widths: PackedFloat32Array, base: int) -> Array:
	var count := points.size()
	var vertices := PackedVector3Array()
	vertices.resize(count * 2)
	var normals := PackedVector3Array()
	normals.resize(count * 2)
	var uvs := PackedVector2Array()
	uvs.resize(count * 2)
	var uv2s := PackedVector2Array()
	uv2s.resize(count * 2)
	var indices := PackedInt32Array()
	indices.resize((count - 1) * 6)
	var along := 0.0
	for i in count:
		var p := points[i]
		if i > 0:
			along += p.distance_to(points[i - 1])
		var prev := points[maxi(i - 1, 0)]
		var next := points[mini(i + 1, count - 1)]
		var dir := (next - prev).normalized()
		if dir == Vector2.ZERO:
			dir = Vector2.RIGHT
		var perp := Vector3(-dir.y, 0.0, dir.x)
		# Altitude × HEIGHT_SCALE sans relief exagéré : le shader pose la hauteur affichée (ZG8).
		var center := Vector3(p.x, maxf(map_data.height_m_at(p.x, p.y), 0.0) * MapData.HEIGHT_SCALE, p.y)
		var k := i * 2
		vertices[k] = center
		vertices[k + 1] = center
		normals[k] = perp
		normals[k + 1] = perp * -1.0
		uvs[k] = Vector2(widths[i], 1.0)
		uvs[k + 1] = Vector2(widths[i], -1.0)
		uv2s[k] = Vector2(along, 0.0)
		uv2s[k + 1] = Vector2(along, 0.0)
	for i in count - 1:
		var a := base + i * 2
		var o := i * 6
		indices[o] = a
		indices[o + 1] = a + 1
		indices[o + 2] = a + 3
		indices[o + 3] = a
		indices[o + 4] = a + 3
		indices[o + 5] = a + 2
	return [vertices, normals, uvs, uv2s, indices]


static func _grown_aabb(vertices: PackedVector3Array) -> AABB:
	var box := AABB(vertices[0], Vector3.ZERO)
	for v in vertices:
		box = box.expand(v)
	return box.grow(4.0)


func _water_material(min_px: float) -> ShaderMaterial:
	var material := ShaderMaterial.new()
	material.shader = WATER_SHADER
	material.set_shader_parameter("min_px", min_px)
	material.set_shader_parameter("map_size", Vector2(map_data.size))
	material.set_shader_parameter("river_bank_px", bank_px)
	if terrain != null and terrain.river_bed_texture() != null:
		material.set_shader_parameter("river_bed", terrain.river_bed_texture())
		material.set_shader_parameter("has_river_bed", true)
		terrain.material.set_shader_parameter("river_bank_px", bank_px)
	material.render_priority = 1
	return material


func _set_fords(fords: Array[Vector4]) -> void:
	var padded := fords.duplicate()
	while padded.size() < 32:
		padded.append(Vector4.ZERO)
	padded.resize(32)
	for material: ShaderMaterial in [_water_major, _water_minor]:
		material.set_shader_parameter("fords", padded)
		material.set_shader_parameter("ford_count", mini(fords.size(), 32))


## Lot ZG5b : matériaux des anciens rubans (fondu dans le disque du réseau fin).
func old_materials() -> Array[ShaderMaterial]:
	var result: Array[ShaderMaterial] = []
	for material in [_water_major, _water_minor]:
		if material != null:
			result.append(material)
	return result


## Lot ZG5b : routes de près (lot C6) à effacer dans le disque des routes fines.
func attach_roads(roads: RoadRenderer) -> void:
	if fine != null:
		fine.attach_roads(roads)


## Lot ZG5b : charge et maille tout de suite le réseau fin autour de la vue (captures, tests).
func flush_fine(camera_distance: float) -> void:
	if fine != null:
		fine.flush(camera_distance)
