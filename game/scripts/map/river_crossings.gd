class_name RiverCrossings
extends Node3D

## Ponts, gués et bacs de la carte de campagne (lot V4, A1-11), rendu seulement.
##
## - Franchissements : `data/map/crossings_px.json` (sortie de `cent-ans geo rivers-render`) :
##   passages historiques de `crossings.json` recalés sur le fleuve affiché (et sur la route qui
##   le franchit), plus un pont générique à chaque croisement route/cours d'eau (ponceau de
##   pierre sur les ruisseaux, pont de bois ailleurs) ; ouvrage selon `structure` (pierre, bois,
##   bateaux, bac, gué ; `BridgeMeshes`), axe selon la route portée (`axis`).
## - Ponts-portes : là où un fleuve passe sous une colonie (`RiversRenderer.covers`), un pont de
##   pierre crénelé à tours (cité, ville, château) ou un pont de bois (village, abbaye) sur le
##   bord de l'emprise ; un franchissement historique situé dans l'emprise lui donne son nom et,
##   pour les colonies ouvertes, sa structure.
## - Zones personnalisées (lot L1) : rien dedans ; pont-porte au bord si `boundary_bridges`.
## Posés sur la surface affichée et recalés quand une tuile change de niveau.

const CROSSINGS_FILE := "crossings_px.json"
## Portée de visibilité (distance caméra → pont), comme les hameaux.
const VISIBILITY_RANGE := 280.0
const WALLED := ["city", "town", "castle"]
## Exagération verticale et en largeur des ouvrages (lisibles au zoom le plus proche).
const HEIGHT_SCALE := 2.0
const DECK_SCALE := 2.0
## Largeur (px carte) sous laquelle un pont-porte n'a pas de tours (simple pont de pierre).
const GATE_MIN_WIDTH := 0.3

var renderer: RiversRenderer
## Un enregistrement par ouvrage : {id, name, structure, px: Vector2, dir: Vector2, width, node}.
var items: Array[Dictionary] = []

var _by_chunk: Dictionary = {}


func build(rivers_renderer: RiversRenderer, settlements: SettlementLayer) -> void:
	renderer = rivers_renderer
	for child in get_children():
		child.queue_free()
	items.clear()
	_by_chunk.clear()
	var in_cover: Dictionary = {}  # index de colonie → franchissement historique
	for entry in _load_crossings():
		var p: Vector2 = entry["px"]
		if bool(entry.get("in_custom_zone", false)) or renderer.in_custom_zone(p, 0.5):
			continue
		var cover := _cover_at(p)
		if cover >= 0:
			if str(entry["type"]) != "road":
				in_cover[cover] = entry
			continue
		_add(entry)
	_add_gate_bridges(settlements, in_cover)
	_add_zone_bridges()
	var terrain := renderer.terrain
	if terrain != null and not terrain.chunk_surface_changed.is_connected(_on_chunk_surface_changed):
		terrain.chunk_surface_changed.connect(_on_chunk_surface_changed)


func bridge_count() -> int:
	return items.size()


## Gués et bacs pour `river_water.gdshader` : Vector4(x, z, rayon, 1 gué / 0 bac).
func ford_uniforms() -> Array[Vector4]:
	var result: Array[Vector4] = []
	for item in items:
		var structure: String = item["structure"]
		if structure == "ford" or structure == "ferry":
			var p: Vector2 = item["px"]
			result.append(Vector4(p.x, p.y, float(item["width"]) * 0.8 + 0.35, 1.0 if structure == "ford" else 0.25))
	return result


func _load_crossings() -> Array[Dictionary]:
	var result: Array[Dictionary] = []
	var path := renderer.map_data.map_dir.path_join(CROSSINGS_FILE)
	if not FileAccess.file_exists(path):
		return result
	var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(path))
	if not (parsed is Dictionary):
		return result
	for raw: Dictionary in parsed.get("crossings", []):
		if not bool(raw.get("snapped", false)):
			continue
		var px: Array = raw.get("px", [0, 0])
		var dir: Array = raw.get("dir", [1, 0])
		var entry := {
			"id": str(raw.get("id", "")),
			"name": str(raw.get("name", "")),
			"structure": str(raw.get("structure", "stone")),
			"px": Vector2(float(px[0]), float(px[1])),
			"dir": Vector2(float(dir[0]), float(dir[1])).normalized(),
			"width": float(raw.get("width", 0.5)),
			"type": str(raw.get("type", "bridge")),
			"in_custom_zone": bool(raw.get("in_custom_zone", false)),
		}
		var axis: Variant = raw.get("axis")
		if axis is Array:
			entry["axis"] = Vector2(float(axis[0]), float(axis[1])).normalized()
		result.append(entry)
	return result


## Index de la colonie dont l'emprise contient `p`, -1 sinon.
func _cover_at(p: Vector2) -> int:
	var k := renderer.cover_at(p)
	return int(renderer.covers[k].w) if k >= 0 else -1


## Ponts-portes sur le bord des emprises des colonies traversées par un fleuve.
func _add_gate_bridges(settlements: SettlementLayer, in_cover: Dictionary) -> void:
	for k in renderer.covers.size():
		var c := renderer.covers[k]
		var center := Vector2(c.x, c.y)
		var index := int(c.w)
		var kind := str(settlements.data.settlements[index].get("kind", "")) if settlements != null else "city"
		var entry: Dictionary = settlements.data.settlements[index] if settlements != null else {}
		var historic: Dictionary = in_cover.get(index, {})
		for code in renderer.cover_segments[k]:
			var river: Dictionary = renderer.rivers[code >> 16]
			var points: PackedVector2Array = river["points"]
			var widths: PackedFloat32Array = river["widths"]
			var i: int = code & 0xFFFF
			var a := points[i]
			var b := points[i + 1]
			var inside_a := a.distance_squared_to(center) < c.z * c.z
			var inside_b := b.distance_squared_to(center) < c.z * c.z
			if inside_a == inside_b:
				continue
			var hit := _circle_point(a, b, center, c.z)
			var structure := ("gate" if widths[i] >= GATE_MIN_WIDTH else "stone") if kind in WALLED else str(historic.get("structure", "wood"))
			if structure == "ferry" or structure == "ford":
				structure = "wood"
			var name := str(historic.get("name", "")) if not historic.is_empty() else "Pont de %s" % str(entry.get("name", ""))
			_add({"id": "gate_%d_%d" % [index, items.size()], "name": name, "structure": structure,
				"px": hit, "dir": (b - a).normalized(), "width": widths[i]})


## Ponts-portes au bord des zones personnalisées (bouts de tronçons coupés sur le cercle).
func _add_zone_bridges() -> void:
	for zone in renderer.zones:
		if not bool(zone["boundary_bridges"]):
			continue
		var center: Vector2 = zone["px"]
		var radius: float = zone["radius_px"]
		for river in renderer.rivers:
			var points: PackedVector2Array = river["points"]
			var widths: PackedFloat32Array = river["widths"]
			var n := points.size()
			for end in [0, n - 1]:
				var p := points[end]
				if absf(p.distance_to(center) - radius) > 0.6:
					continue
				var other := points[1] if end == 0 else points[n - 2]
				_add({"id": "zone_%s_%d" % [zone["id"], items.size()], "name": str(zone["name"]), "structure": "gate",
					"px": p, "dir": (other - p).normalized() if end == 0 else (p - other).normalized(), "width": widths[end]})


static func _circle_point(a: Vector2, b: Vector2, center: Vector2, radius: float) -> Vector2:
	var inside_a := a.distance_squared_to(center) < radius * radius
	var lo := a
	var hi := b
	for _i in 14:
		var m := (lo + hi) * 0.5
		if (m.distance_squared_to(center) < radius * radius) == inside_a:
			lo = m
		else:
			hi = m
	return (lo + hi) * 0.5


## Enregistre un ouvrage ; son maillage n'est construit que lorsque sa tuile de terrain passe au
## niveau proche ou fin (`_on_chunk_surface_changed`) : pas de coût au démarrage pour les
## centaines de ponts lointains.
func _add(entry: Dictionary) -> void:
	var item := entry.duplicate()
	if (item["dir"] as Vector2) == Vector2.ZERO:
		item["dir"] = Vector2.DOWN
	item["width"] = maxf(float(entry["width"]), 0.15)
	item["node"] = null
	items.append(item)
	var p: Vector2 = entry["px"]
	var chunk := renderer.terrain.chunk_index_at(p.x, p.y) if renderer.terrain != null else -1
	if not _by_chunk.has(chunk):
		_by_chunk[chunk] = []
	(_by_chunk[chunk] as Array).append(items.size() - 1)
	if renderer.terrain == null or renderer.terrain.chunk_level(chunk) >= 1:
		_instantiate(item)


func _instantiate(item: Dictionary) -> void:
	var dir: Vector2 = item["dir"]
	var instance := MeshInstance3D.new()
	instance.name = str(item["id"])
	instance.mesh = BridgeMeshes.build(str(item["structure"]), float(item["width"]), absi(str(item["id"]).hash()))
	# X local en travers du fleuve (ou selon la route portée), Z = X × Y (repère direct).
	var across := Vector3(-dir.y, 0.0, dir.x)
	if item.has("axis"):
		var axis: Vector2 = item["axis"]
		across = Vector3(axis.x, 0.0, axis.y)
	var along := across.cross(Vector3.UP)
	# Exagération (comme les maquettes de colonies) : hauteur et largeur du tablier ; la longueur
	# reste celle du fleuve à franchir.
	instance.transform = Transform3D(Basis(across, Vector3.UP * HEIGHT_SCALE, along * DECK_SCALE), Vector3.ZERO)
	instance.visibility_range_end = VISIBILITY_RANGE
	instance.visibility_range_end_margin = VISIBILITY_RANGE * 0.15
	instance.visibility_range_fade_mode = GeometryInstance3D.VISIBILITY_RANGE_FADE_SELF
	add_child(instance)
	item["node"] = instance
	_ground(item)


## Nombre d'ouvrages dont le maillage est construit (tests).
func instantiated_count() -> int:
	var count := 0
	for item in items:
		if item["node"] != null:
			count += 1
	return count


## Niveau de l'eau sous le pont : relief non creusé au milieu du fleuve (surface affichée).
func _ground(item: Dictionary) -> void:
	var node: MeshInstance3D = item["node"]
	if node == null:
		return
	var p: Vector2 = item["px"]
	var y := renderer.map_data.surface_world_at(p.x, p.y)
	if renderer.terrain != null:
		y = maxf(y, renderer.terrain.surface_height_at(p.x, p.y))
	node.position = Vector3(p.x, y, p.y)


func _on_chunk_surface_changed(index: int) -> void:
	var near := renderer.terrain.chunk_level(index) >= 1
	for i in _by_chunk.get(index, []):
		var item: Dictionary = items[i]
		if item["node"] == null:
			if near:
				_instantiate(item)
		else:
			_ground(item)
