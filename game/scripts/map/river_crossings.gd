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
## Lot ZG5b : réduction des ouvrages en mode fin (culées de 0,5 unité → 50 m, tablier ≈ 10-25 m).
const FINE_SCALE := 0.14

var renderer: RiversRenderer
## Un enregistrement par ouvrage : {id, name, structure, px: Vector2, dir: Vector2, width, node}.
var items: Array[Dictionary] = []

var _by_chunk: Dictionary = {}
## Lot ZG5b : ancrages fins (`fine_anchors.json` → `crossings`, ordre de `crossings_px.json`) et
## mode fin (au palier près, sur le fleuve fin : position, sens du courant, largeur réelle, eau
## à `z_water`) ; ponts-portes cachés (recalculés sur le fleuve fin par `FineGeoLayer`).
var _fine_anchors: Array[Dictionary] = []
var _fine_mode := false
var _gates_hidden := false
## Lot ZG4b : ouvrages à remettre en forme après une bascule de mode (index dans `items`),
## étalés sur plusieurs images (`FrameBudget`) : la bascule d'un bloc coûtait ~35 ms.
var _reshape_queue: Array[int] = []
## Lot ZG7a : maillages fins préparés dans un fil dès `set_fine_anchors` (clé du cache
## `BridgeMeshes` → tableaux) : la bascule n'a plus qu'à créer les `ArrayMesh` (≤ 1 ms par
## ouvrage au lieu de 5-16 ms de `SurfaceTool` sous charge).
var _prepared_fine: Dictionary = {}
var _prepare_task := -1
var _prepare_mutex := Mutex.new()
## Remise en forme la plus longue (ms, mesures).
var reshape_ms_max := 0.0


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
	var parsed: Variant = DataFile.parse_file(path)
	if not (parsed is Dictionary):
		return result
	var raw_list: Array = parsed.get("crossings", [])
	for raw_index in raw_list.size():
		var raw: Dictionary = raw_list[raw_index]
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
			"index": raw_index,
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
				"px": hit, "dir": (b - a).normalized(), "width": renderer.generalised_width(widths[i])})


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
					"px": p, "dir": (other - p).normalized() if end == 0 else (p - other).normalized(), "width": renderer.generalised_width(widths[end])})


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
	item["fine_mode"] = _fine_mode
	var instance := MeshInstance3D.new()
	instance.name = str(item["id"])
	instance.visibility_range_end = VISIBILITY_RANGE
	instance.visibility_range_end_margin = VISIBILITY_RANGE * 0.15
	instance.visibility_range_fade_mode = GeometryInstance3D.VISIBILITY_RANGE_FADE_SELF
	add_child(instance)
	item["node"] = instance
	if not item.has("index"):
		instance.visible = not _gates_hidden
	_shape(item)


## Maillage et orientation de l'ouvrage (ancrage fin en mode fin, sinon tracé V4).
func _shape(item: Dictionary) -> void:
	var instance: MeshInstance3D = item["node"]
	var fine := _fine_of(item)
	var dir: Vector2 = fine["dir"] if not fine.is_empty() else item["dir"]
	var width: float = fine["width"] if not fine.is_empty() else float(item["width"])
	if not fine.is_empty():
		# ZG5b : ouvrage à l'échelle réelle (maillage d'une largeur `width / FINE_SCALE` réduit de
		# `FINE_SCALE` : la portée reste celle du fleuve fin, culées et tablier rétrécissent).
		width /= FINE_SCALE
	# ZG4b : un maillage par mode, gardé (un retour au mode précédent ne remaille rien).
	var mesh_key := "mesh_fine" if not fine.is_empty() else "mesh_v4"
	if not item.has(mesh_key):
		var seed_value := absi(str(item["id"]).hash())
		var key := BridgeMeshes.cache_key(str(item["structure"]), width, seed_value)
		_prepare_mutex.lock()
		var surfaces: Array = _prepared_fine.get(key, [])
		_prepare_mutex.unlock()
		if not fine.is_empty() and not surfaces.is_empty():
			item[mesh_key] = BridgeMeshes.build_from(key, surfaces)
		else:
			item[mesh_key] = BridgeMeshes.build(str(item["structure"]), width, seed_value)
	instance.mesh = item[mesh_key]
	# X local en travers du fleuve (ou selon la route portée), Z = X × Y (repère direct).
	var across := Vector3(-dir.y, 0.0, dir.x)
	if item.has("axis") and fine.is_empty():
		var axis: Vector2 = item["axis"]
		across = Vector3(axis.x, 0.0, axis.y)
	var along := across.cross(Vector3.UP)
	# Exagération (comme les maquettes de colonies) : hauteur et largeur du tablier ; la longueur
	# reste celle du fleuve à franchir.
	if fine.is_empty():
		instance.transform = Transform3D(Basis(across, Vector3.UP * HEIGHT_SCALE, along * DECK_SCALE), Vector3.ZERO)
	else:
		item["fine_basis"] = [across, along, width]
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
	var fine := _fine_of(item)
	if not fine.is_empty():
		# ZG5b : origine au niveau de l'eau du fleuve fin (lit creusé dessous).
		var q: Vector2 = fine["px"]
		# Tablier à `z_deck` : hauteur du maillage (sommet du tablier ≈ 0,05 + 0,02 × largeur)
		# mise à l'échelle de la hauteur réelle au-dessus de l'eau × échelle verticale courante.
		var basis: Array = item.get("fine_basis", [])
		if not basis.is_empty():
			var deck_top := (0.05 + 0.02 * float(basis[2])) * FINE_SCALE
			# ZG8 : hauteur affichée du tablier au-dessus de l'eau (relief local exagéré compris).
			var z_water := float(fine["z_water"])
			var rise := MapData.display_height(z_water + maxf(float(fine["z_deck"]) - z_water, 3.0), q.x, q.y) - MapData.display_height(z_water, q.x, q.y)
			var k_h := clampf(rise / maxf(deck_top, 1e-4), 0.3, 12.0)
			# ZG7a : tablier à sa largeur réelle le long du courant.
			var k_along := BridgeMeshes.fine_deck_scale(str(item["structure"]), float(basis[2]), renderer.map_data.meters_per_px, FINE_SCALE)
			node.transform.basis = Basis(basis[0] * FINE_SCALE, Vector3.UP * FINE_SCALE * k_h, basis[1] * k_along)
		node.position = Vector3(q.x, maxf(MapData.display_height(float(fine["z_water"]), q.x, q.y), 0.0), q.y)
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


# --- Lot ZG5b : ancrages fins -------------------------------------------------------------


## Ancrages de `fine_anchors.json` (même ordre que `crossings_px.json`).
func set_fine_anchors(anchors: Array[Dictionary]) -> void:
	_fine_anchors = anchors
	_wait_prepare()
	var todo: Array = []
	for item in items:
		var fine := _fine_anchor_width(item)
		if fine > 0.0:
			var width := fine / FINE_SCALE
			var seed_value := absi(str(item["id"]).hash())
			todo.append([BridgeMeshes.cache_key(str(item["structure"]), width, seed_value), str(item["structure"]), width, seed_value])
	if not todo.is_empty():
		_prepare_task = WorkerThreadPool.add_task(_prepare_fine.bind(todo), false, "fine bridge meshes")


## Largeur fine (unités) de l'ouvrage, 0 sans ancrage fin accroché.
func _fine_anchor_width(item: Dictionary) -> float:
	if not item.has("index"):
		return 0.0
	var index: int = item["index"]
	if index < 0 or index >= _fine_anchors.size():
		return 0.0
	var anchor: Dictionary = _fine_anchors[index]
	if not bool(anchor.get("snapped", false)) or str(anchor.get("id", "")) != str(item["id"]):
		return 0.0
	return maxf(renderer.generalised_width(float(anchor.get("width_m", 0.0)) / renderer.map_data.meters_per_px), 0.01)


func _prepare_fine(todo: Array) -> void:
	for entry: Array in todo:
		var surfaces := BridgeMeshes.build_arrays(entry[1], entry[2], entry[3])
		_prepare_mutex.lock()
		_prepared_fine[entry[0]] = surfaces
		_prepare_mutex.unlock()


func _wait_prepare() -> void:
	if _prepare_task >= 0:
		WorkerThreadPool.wait_for_task_completion(_prepare_task)
		_prepare_task = -1


func _exit_tree() -> void:
	_wait_prepare()


## Ancrage fin d'un ouvrage en mode fin : {px, dir, width (unités), z_water} ou {}. Mode de
## l'ouvrage lui-même (`fine_mode`, ZG4b) : pendant une bascule étalée, les ouvrages pas encore
## remis en forme restent cohérents avec leur maillage.
func _fine_of(item: Dictionary) -> Dictionary:
	if not bool(item.get("fine_mode", false)) or not item.has("index"):
		return {}
	var index: int = item["index"]
	if index < 0 or index >= _fine_anchors.size():
		return {}
	var anchor: Dictionary = _fine_anchors[index]
	if not bool(anchor.get("snapped", false)) or str(anchor.get("id", "")) != str(item["id"]):
		return {}
	var width_m := float(anchor.get("width_m", 0.0))
	return {"px": anchor["px"], "dir": anchor["dir"], "width": maxf(renderer.generalised_width(width_m / renderer.map_data.meters_per_px), 0.01), "z_water": anchor["z_water"], "z_deck": anchor["z_deck"]}


## Ancrage fin (mode fin actif ou non) de l'ouvrage `id`, pour les tests : {} si aucun.
func fine_anchor_of(id: String) -> Dictionary:
	for item in items:
		if str(item["id"]) == id and item.has("index"):
			var index: int = item["index"]
			return _fine_anchors[index] if index < _fine_anchors.size() else {}
	return {}


## Bascule tous les ouvrages construits entre le tracé V4 et les ancrages fins. ZG4b : remise en
## forme étalée (au moins un ouvrage par image, puis tant que `FrameBudget.has_time()`) ; hors
## d'une image ouverte (tests), tout est fait tout de suite.
func set_fine_mode(on: bool) -> void:
	if on == _fine_mode:
		return
	_fine_mode = on
	_reshape_queue.clear()
	for i in items.size():
		var item: Dictionary = items[i]
		if item["node"] != null and item.has("index") and bool(item.get("fine_mode", false)) != on:
			_reshape_queue.append(i)
	pump_reshape()


## Remet en forme les ouvrages en attente (voir `set_fine_mode`) ; `all` : tous (captures).
func pump_reshape(all: bool = false) -> void:
	var first := true
	while not _reshape_queue.is_empty() and (all or first or FrameBudget.has_time()):
		var item: Dictionary = items[_reshape_queue.pop_back()]
		first = false
		if item["node"] == null or bool(item.get("fine_mode", false)) == _fine_mode:
			continue
		item["fine_mode"] = _fine_mode
		var t0 := Time.get_ticks_usec()
		_shape(item)
		reshape_ms_max = maxf(reshape_ms_max, (Time.get_ticks_usec() - t0) / 1000.0)
	set_process(not _reshape_queue.is_empty())


func _process(_delta: float) -> void:
	pump_reshape()


## Nombre d'ouvrages en attente de remise en forme (tests, mesures).
func pending_reshapes() -> int:
	return _reshape_queue.size()


## Ponts-portes et ponts de zone (tracé V4) cachés quand `FineGeoLayer` pose les siens.
func set_gates_hidden(hidden: bool) -> void:
	if hidden == _gates_hidden:
		return
	_gates_hidden = hidden
	for item in items:
		if not item.has("index") and item["node"] != null:
			(item["node"] as Node3D).visible = not hidden


func fine_mode() -> bool:
	return _fine_mode
