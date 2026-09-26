class_name SettlementLayer
extends Node3D

## Colonies et hameaux sur la carte de campagne (lot C6), rendu seulement :
## - paliers Europe et moyen : marqueurs (`settlement_icon.gdshader`, un `MultiMesh`), langage
##   unique du lot DA3 (ADR 0066, `SettlementMarkers`) : pictogramme peint selon le type, écu du
##   détenteur, taille selon le rang, dé-encombrement par distance caméra (données) ; noms des
##   cités (et des villes en se rapprochant) ;
## - palier près : maquettes 3D par type (`ModelLibrary.settlement_model`), posées sur la surface
##   exacte du terrain affiché et recalées quand une tuile change de niveau ; hameaux en
##   `MultiMesh` par tuile (orientation et variante déterministes, brûlés selon la dévastation de
##   la province) ; noms de toutes les colonies.
## Étiquettes : masquage des chevauchements par priorité (cité > ville > château > abbaye >
## village). Picking écran : `pick_screen` → id, `select` → surbrillance + signal.

signal settlement_selected(id: String)

const KIND_INDEX := {"city": 0, "town": 1, "castle": 2, "abbey": 3, "village": 4}
const LABEL_FONT := {"city": 22, "town": 18, "castle": 16, "abbey": 16, "village": 15}
## Rayon de picking d'un marqueur, en fraction de sa taille écran.
const PICK_ICON_FRACTION := 0.45
## Hauteur du centre du marqueur au-dessus du lieu, en fraction de sa taille (cf. shader).
const ICON_CENTER_LIFT := 0.42
## Proportion de hameaux brûlés = dévastation (%) × ce facteur (au-delà d'un seuil).
const BURN_THRESHOLD := 10.0
## Partage de l'écart entre deux maquettes voisines (voir `_fit_models`).
const FIT_WEIGHT := {"city": 3.0, "town": 2.0, "castle": 1.5, "abbey": 1.3, "village": 1.0}
const MIN_FIT_SCALE := 0.55

@export var tiers: ZoomTiers
## Échelle globale des marqueurs (tailles par rang dans `data/map/settlement_markers.json`).
@export var icon_size_scale: float = 1.0
@export var label_color: Color = Color(0.16, 0.10, 0.05)
@export var label_outline: Color = Color(0.95, 0.90, 0.78)
@export var declutter_interval: float = 0.15
@export var declutter_margin: float = 3.0
@export var max_hamlet_builds_per_frame: int = 4

var map_data: MapData
var terrain: TerrainBuilder
var data: SettlementData
var selected_id: String = ""
var stats: Dictionary = {}

var _icons: MultiMeshInstance3D
var _icon_material: ShaderMaterial
## Lot DA3 : catalogue des marqueurs ; par colonie : rang, taille écran (px), distance de retrait.
var markers: SettlementMarkers
var _marker_rank: PackedInt32Array = PackedInt32Array()
var _marker_size: PackedFloat32Array = PackedFloat32Array()
var _marker_until: PackedFloat32Array = PackedFloat32Array()
## Écu affiché par colonie (faction), pour ne réécrire que ce qui change.
var _marker_holder: PackedStringArray = PackedStringArray()
var _icon_distance := -1.0
var _labels: Array[Label3D] = []
var _models: Array = []  # par colonie : Node3D ou null
var _model_radius: PackedFloat32Array = PackedFloat32Array()
var _model_top: PackedFloat32Array = PackedFloat32Array()
var _models_root: Node3D
## Villes emblématiques (lot L1) : index de colonie → LandmarkModel (toujours visibles, LOD par
## portées de visibilité), sous `_landmarks_root`.
var _landmarks: Dictionary = {}
var _landmarks_root: Node3D
## ZG4 : maquettes masquées au palier « site ».
var _site_hidden: bool = false
var _labels_dirty: bool = false
var _labels_root: Node3D
var _hamlets_root: Node3D
var _selection_ring: MeshInstance3D
var _settlements_by_chunk: Dictionary = {}
var _hamlets_by_chunk: Dictionary = {}
var _hamlet_nodes: Dictionary = {}  # index de tuile → Node3D
var _hamlet_dirty: Dictionary = {}
var _burned_material: StandardMaterial3D
## Dévastation par province (0-100), lue depuis la simulation au rafraîchissement.
var _devastation: Dictionary = {}
var _colors: PackedColorArray = PackedColorArray()
var _declutter_timer := 0.0
var _weights := Vector3(-1, -1, -1)  # près, moyen, loin
## Palier près (`_weights.x > 0.35`) des étiquettes au dernier placement : -1 inconnu, 0 non, 1 oui.
var _label_near := -1
var _camera_distance := 0.0
var _regrounded: Dictionary = {}
## ZG6 : villes ordinaires à l'échelle réelle (paliers vallée et site), voir `TownLayer`.
var towns: TownLayer
var _towns_version := -1
## Lot ZG5b : positions de rendu affinées (`fine_anchors.json`) des maquettes (index → Vector2)
## et des hameaux (x, y, z, déplacement), sans toucher aux positions de règles (`data`).
var _anchor_px: Dictionary = {}
var _hamlet_anchors: PackedVector4Array = PackedVector4Array()


func setup(map: MapData, terrain_builder: TerrainBuilder, settlement_data: SettlementData, zoom_tiers: ZoomTiers) -> void:
	for child in get_children():
		child.queue_free()
	map_data = map
	terrain = terrain_builder
	data = settlement_data
	tiers = zoom_tiers if zoom_tiers != null else ZoomTiers.new()
	_labels.clear()
	_models.clear()
	_settlements_by_chunk.clear()
	_hamlets_by_chunk.clear()
	_hamlet_nodes.clear()
	_burned_material = StandardMaterial3D.new()
	_burned_material.albedo_color = Color(0.075, 0.062, 0.05)
	_burned_material.roughness = 1.0
	_models_root = Node3D.new()
	_models_root.name = "Models"
	add_child(_models_root)
	_landmarks.clear()
	_landmarks_root = Node3D.new()
	_landmarks_root.name = "Landmarks"
	add_child(_landmarks_root)
	_labels_root = Node3D.new()
	_labels_root.name = "Labels"
	add_child(_labels_root)
	_hamlets_root = Node3D.new()
	_hamlets_root.name = "Hamlets"
	add_child(_hamlets_root)
	var count := data.settlements.size()
	_colors.resize(count)
	_colors.fill(Color(0.6, 0.6, 0.6))
	_model_radius.resize(count)
	_model_top.resize(count)
	for i in count:
		var entry: Dictionary = data.settlements[i]
		var px: Vector2 = entry["px"]
		_register(_settlements_by_chunk, terrain.chunk_index_at(px.x, px.y), i)
		_build_model(i, entry)
		_build_label(i, entry)
	_fit_models()
	for i in data.hamlets.size():
		var hpx: Vector2 = data.hamlets[i]["px"]
		_register(_hamlets_by_chunk, terrain.chunk_index_at(hpx.x, hpx.y), i)
	_build_icons()
	_build_selection_ring()
	_setup_towns()
	if not terrain.chunk_surface_changed.is_connected(_on_chunk_surface_changed):
		terrain.chunk_surface_changed.connect(_on_chunk_surface_changed)
	stats = {"settlements": count, "hamlets": data.hamlets.size(), "models": _models.filter(func(m: Variant) -> bool: return m != null).size()}


static func _register(map: Dictionary, key: int, value: int) -> void:
	if key < 0:
		return
	if not map.has(key):
		map[key] = PackedInt32Array()
	map[key].append(value)


static func _hash(text: String) -> int:
	return absi(text.hash())


# --- Construction ------------------------------------------------------------------


func _build_model(i: int, entry: Dictionary) -> void:
	if _build_landmark(i, entry):
		return
	var model := ModelLibrary.settlement_model(str(entry["kind"]), _hash(str(entry["id"])) / 7)
	if model == null:
		_models.append(null)
		_model_radius[i] = 2.0
		_model_top[i] = 2.0
		return
	var holder := Node3D.new()
	holder.name = str(entry["id"])
	var px: Vector2 = entry["px"]
	holder.position = Vector3(px.x, 0.0, px.y)
	holder.rotation.y = float(_hash(str(entry["id"]) + "yaw") % 628) / 100.0
	holder.add_child(model)
	var aabb := _model_aabb(model)
	_model_radius[i] = maxf(aabb.size.x, aabb.size.z) * 0.5
	_model_top[i] = aabb.end.y
	for geometry in model.find_children("*", "GeometryInstance3D", true, false):
		var g := geometry as GeometryInstance3D
		g.visibility_range_end = tiers.model_range
		g.visibility_range_end_margin = tiers.model_range * 0.15
	_models_root.add_child(holder)
	_models.append(holder)
	_ground_model(i)


## Ville emblématique (lot L1) : maquette dédiée à la place de la maquette générique.
func _build_landmark(i: int, entry: Dictionary) -> bool:
	var plan := LandmarkLibrary.for_settlement(str(entry["id"]))
	if plan.is_empty():
		return false
	var landmark := LandmarkModel.create(plan, terrain)
	if landmark == null:
		return false
	landmark.set_year(1337)
	_landmarks_root.add_child(landmark)
	_landmarks[i] = landmark
	_models.append(landmark)
	_model_radius[i] = landmark.core_radius
	_model_top[i] = 1.1
	return true


func _is_landmark(i: int) -> bool:
	return _landmarks.has(i)


## Hauteur de base d'une maquette (les villes emblématiques sont drapées par leur shader).
func _model_base_y(i: int) -> float:
	if _landmarks.has(i):
		return (_landmarks[i] as LandmarkModel).ground_height()
	return (_models[i] as Node3D).position.y


## Cercles (x, z, rayon) des villes emblématiques, pour le zoom rapproché de la caméra.
func landmark_zones() -> PackedVector3Array:
	var zones := PackedVector3Array()
	for landmark in _landmarks.values():
		var center: Vector3 = (landmark as LandmarkModel).position
		zones.append(Vector3(center.x, center.z, (landmark as LandmarkModel).zone_radius))
	return zones


## Vrai si un point carte est couvert par une ville emblématique (hameaux, végétation).
func covered_by_landmark(px: Vector2) -> bool:
	for landmark in _landmarks.values():
		if (landmark as LandmarkModel).covers(px):
			return true
	return false


## Réduit les maquettes trop proches d'une voisine (Paris / Vincennes / Saint-Denis) : l'écart
## entre deux colonies est partagé au prorata du poids du type, sans descendre sous
## `MIN_FIT_SCALE`. Étiquettes et picking utilisent le rayon réduit.
func _fit_models() -> void:
	for i in data.settlements.size():
		var holder: Node3D = _models[i]
		if holder == null or _landmarks.has(i):
			continue
		var entry: Dictionary = data.settlements[i]
		var px: Vector2 = entry["px"]
		var weight: float = FIT_WEIGHT.get(str(entry["kind"]), 1.0)
		var allowed := INF
		var index := terrain.chunk_index_at(px.x, px.y)
		for dy in [-1, 0, 1]:
			for dx in [-1, 0, 1]:
				var neighbor: int = index + dy * TerrainBuilder.CHUNKS + dx
				for j in _settlements_by_chunk.get(neighbor, PackedInt32Array()):
					if j == i:
						continue
					var other: Dictionary = data.settlements[j]
					var d := px.distance_to(other["px"])
					var other_weight: float = FIT_WEIGHT.get(str(other["kind"]), 1.0)
					allowed = minf(allowed, d * weight / (weight + other_weight))
		if allowed < _model_radius[i]:
			var factor := maxf(allowed / _model_radius[i], MIN_FIT_SCALE)
			var model := holder.get_child(0) as Node3D
			model.scale *= factor
			_model_radius[i] *= factor
			_model_top[i] *= factor
			_ground_model(i)


static func _model_aabb(root: Node3D) -> AABB:
	var result := AABB()
	var first := true
	for child in root.find_children("*", "MeshInstance3D", true, false):
		var mesh_instance := child as MeshInstance3D
		if mesh_instance.mesh == null:
			continue
		var local := mesh_instance.mesh.get_aabb()
		var xform := root.transform * _relative_transform(root, mesh_instance)
		var box := xform * local
		result = box if first else result.merge(box)
		first = false
	return result


static func _relative_transform(root: Node3D, node: Node3D) -> Transform3D:
	var xform := Transform3D.IDENTITY
	var current: Node = node
	while current != null and current != root:
		if current is Node3D:
			xform = (current as Node3D).transform * xform
		current = current.get_parent()
	return xform


## Pose la maquette sur la surface affichée : point le plus bas de l'emprise (les fondations des
## modèles descendent sous z = 0, rien ne flotte sur une pente).
func _ground_model(i: int) -> void:
	var holder: Node3D = _models[i]
	if holder == null or _landmarks.has(i):
		return
	var px := model_px(i)
	var radius := _model_radius[i] * 0.7
	var low := terrain.surface_height_at(px.x, px.y)
	for k in 8:
		var angle := k * TAU / 8.0
		low = minf(low, terrain.surface_height_at(px.x + cos(angle) * radius, px.y + sin(angle) * radius))
	holder.position.y = low - 0.03


func _build_label(i: int, entry: Dictionary) -> void:
	var label := Label3D.new()
	var kind := str(entry["kind"])
	label.name = "Label_%d" % i
	label.text = str(entry["name"])
	label.font_size = LABEL_FONT.get(kind, 15)
	label.outline_size = 7
	label.modulate = label_color
	label.outline_modulate = label_outline
	label.billboard = BaseMaterial3D.BILLBOARD_ENABLED
	label.fixed_size = true
	label.pixel_size = 0.0011
	label.no_depth_test = true
	label.render_priority = 3
	label.outline_render_priority = 2
	label.visible = false
	var px: Vector2 = entry["px"]
	label.position = Vector3(px.x, map_data.surface_world_at(px.x, px.y), px.y)
	_labels_root.add_child(label)
	_labels.append(label)


## Instance du `MultiMesh` d'une colonie : ordre inverse de la priorité, pour que les lieux de
## rang élevé (cités, triées en tête) soient dessinés par-dessus les petits.
func _icon_instance(i: int) -> int:
	return data.settlements.size() - 1 - i


func _build_icons() -> void:
	markers = SettlementMarkers.load_default()
	var count := data.settlements.size()
	_marker_rank.resize(count)
	_marker_size.resize(count)
	_marker_until.resize(count)
	_marker_holder.resize(count)
	var quad := QuadMesh.new()
	quad.size = Vector2.ONE
	var multimesh := MultiMesh.new()
	multimesh.transform_format = MultiMesh.TRANSFORM_3D
	multimesh.use_colors = true
	multimesh.use_custom_data = true
	multimesh.mesh = quad
	multimesh.instance_count = count
	for i in count:
		var entry: Dictionary = data.settlements[i]
		var px: Vector2 = entry["px"]
		var kind := str(entry["kind"])
		var rank := markers.rank_of(entry)
		_marker_rank[i] = rank
		_marker_size[i] = markers.size_px(kind, rank)
		_marker_until[i] = markers.visible_until(kind, rank)
		_marker_holder[i] = ""
		var cell := markers.cell_of(markers.pictogram_for(kind, rank))
		var k := _icon_instance(i)
		multimesh.set_instance_transform(k, Transform3D(Basis.IDENTITY, Vector3(px.x, map_data.surface_world_at(px.x, px.y) + 0.5, px.y)))
		multimesh.set_instance_color(k, Color(-1.0, 1.0 if bool(entry.get("port", false)) else 0.0, 0.0, 1.0))
		multimesh.set_instance_custom_data(k, Color(cell, 0.0, _marker_size[i], _marker_until[i] / 100.0))
	_icon_material = ShaderMaterial.new()
	_icon_material.shader = preload("res://shaders/settlement_icon.gdshader")
	_icon_material.set_shader_parameter("atlas", markers.atlas)
	_icon_material.set_shader_parameter("atlas_grid", Vector2(markers.atlas_columns, markers.atlas_rows))
	_icon_material.set_shader_parameter("port_cell", float(markers.port_cell()))
	_icon_material.set_shader_parameter("shield_place", markers.placement("shield"))
	_icon_material.set_shader_parameter("badge_place", markers.placement("badge"))
	_icon_material.set_shader_parameter("fade_distance", markers.fade_distance())
	_icon_material.set_shader_parameter("size_scale", icon_size_scale)
	_icon_material.render_priority = 2
	_icons = MultiMeshInstance3D.new()
	_icons.name = "Icons"
	_icons.multimesh = multimesh
	_icons.material_override = _icon_material
	_icons.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	_icons.extra_cull_margin = 64.0
	add_child(_icons)


## Lot DA3 : écus des détenteurs. L'atlas d'écus est recomposé quand une faction nouvelle
## apparaît (rare : révolte, succession), sinon seules les instances changées sont réécrites.
func _refresh_shields() -> void:
	if _icons == null or markers == null:
		return
	var factions: Array = []
	var missing := markers.shield_atlas == null
	for entry in data.settlements:
		var controller := str(entry["controller"])
		if controller != "" and not factions.has(controller):
			factions.append(controller)
			if not markers.shield_index.has(controller):
				missing = true
	if missing:
		factions.sort()
		markers.build_shield_atlas(factions)
		_icon_material.set_shader_parameter("shields", markers.shield_atlas)
		_icon_material.set_shader_parameter("shield_grid", Vector2(markers.shield_columns, markers.shield_rows))
		_marker_holder.fill("?")
	var multimesh := _icons.multimesh
	for i in data.settlements.size():
		var controller := str(data.settlements[i]["controller"])
		if controller == _marker_holder[i]:
			continue
		_marker_holder[i] = controller
		var k := _icon_instance(i)
		var color := multimesh.get_instance_color(k)
		color.r = float(markers.shield_of(controller))
		multimesh.set_instance_color(k, color)


## Taille écran (px) du marqueur d'une colonie (légende, étiquettes, picking).
func marker_size(i: int) -> float:
	return _marker_size[i] * icon_size_scale if i >= 0 and i < _marker_size.size() else 24.0


## Vrai si le marqueur de la colonie `i` est affiché à la distance caméra courante.
func marker_visible(i: int) -> bool:
	return i >= 0 and i < _marker_until.size() and _camera_distance < _marker_until[i] and _weights.x < 0.65


func _build_selection_ring() -> void:
	var torus := TorusMesh.new()
	torus.inner_radius = 0.92
	torus.outer_radius = 1.0
	torus.rings = 48
	torus.ring_segments = 6
	var material := StandardMaterial3D.new()
	material.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	material.albedo_color = Color(1.0, 0.84, 0.3)
	material.no_depth_test = true
	material.render_priority = 2
	_selection_ring = MeshInstance3D.new()
	_selection_ring.name = "SelectionRing"
	_selection_ring.mesh = torus
	_selection_ring.material_override = material
	_selection_ring.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	_selection_ring.visible = false
	add_child(_selection_ring)


# --- État de la simulation ---------------------------------------------------------


## Couleurs des contrôleurs et dévastation des provinces. `color_of(faction_id) -> Color`.
func refresh(sim: Object, color_of: Callable) -> void:
	if data == null:
		return
	data.apply_live(sim)
	if not _landmarks.is_empty() and sim != null and sim.has_method("get_date_label"):
		var year := LandmarkModel.year_of(str(sim.call("get_date_label")))
		if year > 0:
			for landmark in _landmarks.values():
				(landmark as LandmarkModel).set_year(year)
	_refresh_shields()
	for i in data.settlements.size():
		var entry: Dictionary = data.settlements[i]
		var controller := str(entry["controller"])
		var color := Color(0.62, 0.6, 0.55)
		if controller != "" and color_of.is_valid():
			color = color_of.call(controller)
			color.a = 1.0
		if color != _colors[i]:
			_colors[i] = color
			if _models[i] != null:
				ModelLibrary.tint_banner(_models[i], color)
	var devastation := {}
	if sim != null and sim.has_method("get_province_state"):
		var provinces := {}
		for hamlet in data.hamlets:
			provinces[hamlet["province"]] = true
		for province_id in provinces:
			var state: Dictionary = sim.call("get_province_state", province_id)
			devastation[province_id] = float(state.get("devastation", 0.0))
	if devastation != _devastation:
		_devastation = devastation
		for index in _hamlet_nodes:
			_hamlet_dirty[index] = true


# --- Mise à jour par image -----------------------------------------------------------


func update_view(camera_distance: float) -> void:
	if data == null:
		return
	_camera_distance = camera_distance
	var weights := Vector3(tiers.near_weight(camera_distance), tiers.medium_weight(camera_distance), tiers.far_weight(camera_distance))
	# ZG4 : au palier « site » (~1 km, jusqu'à 200 m), les maquettes à la loupe (colonies ×3-7,
	# villes emblématiques ×3,5) dépasseraient les collines : masquées en attendant les villes à
	# l'échelle réelle (ZG6, VH4).
	var site := tiers.site_weight(camera_distance) > 0.5
	if weights != _weights or site != _site_hidden:
		_weights = weights
		_site_hidden = site
		# DA3 : marqueurs aux paliers Europe et moyen (dé-encombrés par le shader), retirés au
		# profit des maquettes au palier près.
		var icon_alpha := 1.0 - weights.x
		_icon_material.set_shader_parameter("alpha", icon_alpha)
		_icons.visible = icon_alpha > 0.01
		_models_root.visible = weights.x > 0.35 and not site and not _towns_active()
		_hamlets_root.visible = weights.x > 0.35 and not site
		_landmarks_root.visible = not site
		# Hauteurs des étiquettes : ne dépendent que du seuil près (pas des poids continus).
		var near := 1 if weights.x > 0.35 else 0
		if near != _label_near:
			_label_near = near
			_update_label_heights()
		_declutter_timer = 0.0
	if _labels_dirty:
		_labels_dirty = false
		_update_label_heights()
	if not is_equal_approx(camera_distance, _icon_distance) and _icon_material != null:
		_icon_distance = camera_distance
		_icon_material.set_shader_parameter("camera_distance", camera_distance)
	_update_towns(camera_distance)
	_update_hamlets()
	_update_selection_ring()
	_declutter_timer -= get_process_delta_time() if is_inside_tree() else 0.0
	if _declutter_timer <= 0.0:
		_declutter_timer = declutter_interval
		declutter()


func _on_chunk_surface_changed(index: int) -> void:
	for i in _settlements_by_chunk.get(index, PackedInt32Array()):
		_ground_model(i)
	if _hamlet_nodes.has(index):
		_hamlet_dirty[index] = true
	# ZG4 : hauteurs des étiquettes une fois par image (et non à chaque morceau recalé).
	_labels_dirty = true


## Hauteur des étiquettes : au-dessus de la maquette (près) ou de l'icône (moyen).
func _update_label_heights() -> void:
	var near := _weights.x > 0.35
	for i in _labels.size():
		var label := _labels[i]
		var px: Vector2 = data.settlements[i]["px"]
		if near and _models[i] != null:
			if not _landmarks.has(i):
				var model_at := model_px(i)  # ZG5b : au-dessus de la maquette ancrée
				label.position.x = model_at.x
				label.position.z = model_at.y
			if _towns_active() and not _landmarks.has(i):
				label.position.y = _model_base_y(i) + 0.12  # ZG6 : ville 1:1, pas de maquette
			else:
				label.position.y = _model_base_y(i) + _model_top[i] + 0.8
			label.offset = Vector2.ZERO
		else:
			# Palier moyen : au-dessus de l'icône (décalage en pixels écran).
			label.position = Vector3(px.x, map_data.surface_world_at(px.x, px.y) + 0.5, px.y)
			label.offset = Vector2(0.0, marker_size(i) * (0.5 + ICON_CENTER_LIFT) + label.font_size * 0.4)


## Opacité d'une étiquette selon le type et le palier (cité : moyen et près ; ville : moyen
## rapproché ; autres : près seulement).
func _label_alpha(kind: String) -> float:
	match kind:
		"city":
			return clampf(_weights.x + _weights.y, 0.0, 1.0)
		"town":
			var t := 1.0 - smoothstep(tiers.town_label_distance * 0.85, tiers.town_label_distance * 1.15, _camera_distance)
			return clampf(maxf(_weights.x, _weights.y * t), 0.0, 1.0)
		_:
			return _weights.x


## Masque les étiquettes qui en chevauchent une plus prioritaire (colonies déjà triées).
func declutter() -> void:
	var camera := get_viewport().get_camera_3d() if is_inside_tree() else null
	if camera == null:
		return
	var screen := get_viewport().get_visible_rect()
	var placed: Array[Rect2] = []
	# ZG4 : paliers vallée / site (vue rasante) : seulement les colonies proches, l'horizon ne se
	# couvre pas de noms.
	var close_w := tiers.valley_weight(_camera_distance) if tiers != null else 0.0
	var label_range := tiers.close_label_range_factor * _camera_distance if tiers != null else INF
	for i in _labels.size():
		var label := _labels[i]
		var alpha := _label_alpha(str(data.settlements[i]["kind"]))
		if close_w > 0.5 and camera.global_position.distance_to(label.global_position) > label_range:
			alpha = 0.0
		if alpha < 0.02 or camera.is_position_behind(label.global_position):
			label.visible = false
			continue
		var rect := _label_rect(label, camera, declutter_margin)
		if not screen.intersects(rect):
			label.visible = false
			continue
		var free := true
		for other in placed:
			if other.intersects(rect):
				free = false
				break
		label.visible = free
		if free:
			placed.append(rect)
			var modulate := label_color
			modulate.a = alpha
			label.modulate = modulate
			var outline := label_outline
			outline.a = alpha
			label.outline_modulate = outline


## Rectangle écran estimé d'une étiquette (taille de police et longueur du texte).
static func _label_rect(label: Label3D, camera: Camera3D, margin: float) -> Rect2:
	var center := camera.unproject_position(label.global_position) - Vector2(0.0, label.offset.y)
	var width := label.text.length() * label.font_size * 0.5 + margin * 2.0
	var height := label.font_size * 1.05 + margin * 2.0
	return Rect2(center - Vector2(width, height) * 0.5, Vector2(width, height))


## Lot UX1 : rectangles écran des noms de colonies affichés (obstacles des plaques d'armée).
func screen_label_rects(camera: Camera3D) -> Array[Rect2]:
	var rects: Array[Rect2] = []
	if camera == null:
		return rects
	var view_height := camera.get_viewport().get_visible_rect().size.y
	for label in _labels:
		if label.visible and label.modulate.a > 0.15 and not camera.is_position_behind(label.global_position):
			rects.append(LabelPlacer.label3d_screen_rect(label, camera, view_height))
	return rects


func visible_label_count() -> int:
	var count := 0
	for label in _labels:
		if label.visible:
			count += 1
	return count


# --- Hameaux -------------------------------------------------------------------------


func _update_hamlets() -> void:
	var show := _weights.x > 0.35
	var builds := 0
	for index in _hamlets_by_chunk:
		var level := terrain.chunk_level(index)
		var node: Node3D = _hamlet_nodes.get(index)
		if show and level >= 1:
			if node == null or _hamlet_dirty.has(index):
				if builds >= max_hamlet_builds_per_frame or (builds > 0 and not FrameBudget.has_time()):
					continue
				builds += 1
				_build_hamlets(index)
		elif node != null and level == 0:
			node.queue_free()
			_hamlet_nodes.erase(index)
			_hamlet_dirty.erase(index)
	stats["hamlet_chunks"] = _hamlet_nodes.size()


## Construit tout de suite hameaux et maquettes voulus (captures, tests).
func flush() -> void:
	var saved := max_hamlet_builds_per_frame
	max_hamlet_builds_per_frame = 1 << 20
	FrameBudget.unlimited = true
	_update_hamlets()
	FrameBudget.unlimited = false
	max_hamlet_builds_per_frame = saved
	for landmark: LandmarkModel in _landmarks.values():  # ZG4 : cuissons étalées terminées
		landmark.flush_bake()
	if towns != null:  # ZG6 : villes 1:1 autour de la caméra
		towns.flush()
		_update_towns(_camera_distance)
	_labels_dirty = false
	_update_label_heights()


func hamlet_instance_count() -> int:
	var total := 0
	for node in _hamlet_nodes.values():
		for mmi in (node as Node3D).get_children():
			total += (mmi as MultiMeshInstance3D).multimesh.instance_count
	return total


func _build_hamlets(index: int) -> void:
	_hamlet_dirty.erase(index)
	var previous: Node3D = _hamlet_nodes.get(index)
	if previous != null:
		previous.queue_free()
	var node := Node3D.new()
	node.name = "Hamlets_%d" % index
	_hamlets_root.add_child(node)
	_hamlet_nodes[index] = node
	var meshes := ModelLibrary.hamlet_meshes()
	if meshes.is_empty():
		return
	# Groupes : (variante, brûlé) → transformations.
	var groups := {}
	for h in _hamlets_by_chunk[index]:
		var hamlet: Dictionary = data.hamlets[h]
		var px: Vector2 = hamlet["px"]
		if not _landmarks.is_empty() and covered_by_landmark(px):
			continue
		var seed_value := _hash(str(hamlet["name"]) + str(px))
		px = hamlet_px(h)  # ZG5b : ancrage fin (tirages inchangés)
		var variant := seed_value % meshes.size()
		var devastation: float = _devastation.get(hamlet["province"], 0.0)
		var burned := devastation >= BURN_THRESHOLD and float((seed_value / 7) % 100) < devastation
		var key := variant * 2 + (1 if burned else 0)
		var yaw := float((seed_value / 13) % 628) / 100.0
		var scale := ModelLibrary.HAMLET_SCALE * (0.85 + float((seed_value / 17) % 30) / 100.0)
		var y := terrain.surface_height_at(px.x, px.y)
		for k in 4:
			var angle := k * TAU / 4.0 + yaw
			y = minf(y, terrain.surface_height_at(px.x + cos(angle) * scale * 0.4, px.y + sin(angle) * scale * 0.4))
		var basis := Basis(Vector3.UP, yaw).scaled(Vector3.ONE * scale)
		if not groups.has(key):
			groups[key] = []
		groups[key].append(Transform3D(basis, Vector3(px.x, y - 0.03, px.y)))
	for key in groups:
		var transforms: Array = groups[key]
		var multimesh := MultiMesh.new()
		multimesh.transform_format = MultiMesh.TRANSFORM_3D
		multimesh.mesh = meshes[key / 2]
		multimesh.instance_count = transforms.size()
		for t in transforms.size():
			multimesh.set_instance_transform(t, transforms[t])
		var mmi := MultiMeshInstance3D.new()
		mmi.multimesh = multimesh
		if key % 2 == 1:
			mmi.material_override = _burned_material
		mmi.visibility_range_end = tiers.hamlet_range + terrain.chunk_px
		node.add_child(mmi)


# --- Picking et sélection ------------------------------------------------------------


## Colonie sous un point écran (icône au palier moyen, maquette au palier près), "" sinon.
func pick_screen(screen_position: Vector2) -> String:
	return str(pick_screen_scored(screen_position).get("id", ""))


## Q2 : comme `pick_screen`, avec `score` (distance au centre / rayon de prise, 0 = en plein
## centre) pour départager une colonie et une armée sous le même clic ; {} si rien.
func pick_screen_scored(screen_position: Vector2) -> Dictionary:
	var camera := get_viewport().get_camera_3d() if is_inside_tree() else null
	if camera == null or data == null:
		return {}
	var near := _weights.x > 0.35
	var icons := _weights.x < 0.65
	var best := ""
	var best_score := INF
	for i in data.settlements.size():
		var px: Vector2 = data.settlements[i]["px"]
		if near and _models[i] != null:
			var holder: Node3D = _models[i]
			var center := Vector3(holder.position.x, _model_base_y(i) + _model_top[i] * 0.4, holder.position.z)
			if camera.is_position_behind(center):
				continue
			var screen_center := camera.unproject_position(center)
			var edge := camera.unproject_position(center + camera.global_transform.basis.x * _model_radius[i])
			var radius_px := maxf(screen_center.distance_to(edge), 8.0)
			var d := screen_center.distance_to(screen_position)
			if d < radius_px and d / radius_px < best_score:
				best_score = d / radius_px
				best = str(data.settlements[i]["id"])
		elif icons and marker_visible(i):
			var world := Vector3(px.x, map_data.surface_world_at(px.x, px.y) + 0.5, px.y)
			if camera.is_position_behind(world):
				continue
			var size := marker_size(i)
			var radius := size * PICK_ICON_FRACTION
			var center_px := camera.unproject_position(world) - Vector2(0.0, size * ICON_CENTER_LIFT)
			var d_icon := center_px.distance_to(screen_position)
			if d_icon < radius and d_icon / radius < best_score:
				best_score = d_icon / radius
				best = str(data.settlements[i]["id"])
	return {"id": best, "score": best_score} if best != "" else {}


## Sélectionne une colonie ("" = aucune) : surbrillance de l'icône, anneau au sol, signal.
func select(id: String) -> void:
	if _icons != null and selected_id != "" and data.index_by_id.has(selected_id):
		var old := _icon_instance(data.index_by_id[selected_id])
		var custom := _icons.multimesh.get_instance_custom_data(old)
		custom.g = 0.0
		_icons.multimesh.set_instance_custom_data(old, custom)
	selected_id = id if data.index_by_id.has(id) else ""
	if selected_id != "":
		var index: int = data.index_by_id[selected_id]
		var custom_new := _icons.multimesh.get_instance_custom_data(_icon_instance(index))
		custom_new.g = 1.0
		_icons.multimesh.set_instance_custom_data(_icon_instance(index), custom_new)
		var entry: Dictionary = data.settlements[index]
		print("SettlementLayer: selected %s (%s, %s, controller %s)" % [selected_id, entry["name"], entry["kind"], entry["controller"]])
		settlement_selected.emit(selected_id)
	_update_selection_ring()


func _update_selection_ring() -> void:
	if _selection_ring == null:
		return
	var index: int = data.index_by_id.get(selected_id, -1) if data != null else -1
	var holder: Node3D = _models[index] if index >= 0 else null
	var show := index >= 0 and holder != null and _weights.x > 0.35
	_selection_ring.visible = show
	if show:
		var radius := _model_radius[index] * 1.1
		_selection_ring.position = Vector3(holder.position.x, _model_base_y(index) + 0.15, holder.position.z)
		_selection_ring.scale = Vector3(radius, 1.0, radius)


## Position monde d'une colonie (maquette posée, sinon relief), Vector3.ZERO si inconnue.
func world_position_of(id: String) -> Vector3:
	var index: int = data.index_by_id.get(id, -1) if data != null else -1
	if index < 0:
		return Vector3.ZERO
	var px: Vector2 = data.settlements[index]["px"]
	return Vector3(px.x, terrain.surface_height_at(px.x, px.y), px.y)


## Lot ZG5b : position de rendu de la maquette `i` (ancrage fin, sinon position de règle).
func model_px(i: int) -> Vector2:
	return _anchor_px.get(i, data.settlements[i]["px"])


## Lot ZG5b : position de rendu du hameau `h` (ancrage fin, sinon `hamlets.json`).
func hamlet_px(h: int) -> Vector2:
	if h < _hamlet_anchors.size():
		var a := _hamlet_anchors[h]
		return Vector2(a.x, a.y)
	return data.hamlets[h]["px"]


## Lot ZG5b : maquettes (hors villes emblématiques) et hameaux posés aux ancrages fins de
## `fine_anchors.json` (déplacés de ≤ 300 m hors des lits et des pentes fortes), puis recalés
## sur la surface affichée. Icônes, étiquettes du palier moyen, picking et règles gardent les
## positions de `data`.
func apply_fine_anchors(store: FineGeoStore) -> void:
	_anchor_px.clear()
	for i in data.settlements.size():
		var id := str(data.settlements[i]["id"])
		if _landmarks.has(i) or not store.settlements.has(id) or _models[i] == null:
			continue
		var p: Vector2 = store.settlements[id]["px"]
		_anchor_px[i] = p
		var holder := _models[i] as Node3D
		holder.position.x = p.x
		holder.position.z = p.y
		_ground_model(i)
	_hamlet_anchors = store.hamlets if store.hamlets.size() == data.hamlets.size() else PackedVector4Array()
	for index in _hamlet_nodes:
		_hamlet_dirty[index] = true
	_update_label_heights()


## Cercles d'exclusion de la végétation (x, y, rayon en px carte) : colonies et hameaux.
func vegetation_exclusions() -> PackedVector3Array:
	var result := PackedVector3Array()
	for i in data.settlements.size():
		var px: Vector2 = data.settlements[i]["px"]
		result.append(Vector3(px.x, px.y, _model_radius[i] * 1.1 + 0.5))
	for landmark in _landmarks.values():
		var center: Vector3 = (landmark as LandmarkModel).position
		result.append(Vector3(center.x, center.z, (landmark as LandmarkModel).zone_radius))
	for hamlet in data.hamlets:
		var hpx: Vector2 = hamlet["px"]
		result.append(Vector3(hpx.x, hpx.y, ModelLibrary.HAMLET_SCALE * 0.6))
	return result


# --- Lot CV1 : accès pour la campagne vivante (croissance, fumées) -------------------------


## Support (Node3D posé sur le relief) de la maquette de la colonie `i`, null sans maquette.
func model_holder(i: int) -> Node3D:
	if _is_landmark(i):
		return null  # ville emblématique (L1) : pas de croissance ni de surcouche génériques
	return _models[i] if i >= 0 and i < _models.size() else null


## Rayon au sol et hauteur (unités monde) de la maquette de la colonie `i`.
func model_radius(i: int) -> float:
	return _model_radius[i] if i >= 0 and i < _model_radius.size() else 2.0


func model_top(i: int) -> float:
	return _model_top[i] if i >= 0 and i < _model_top.size() else 2.0


## Remplace la maquette de la colonie `i` (lot CV1 : croissance) ; `model` est déjà à l'échelle
## monde. Garde position, orientation, portée de visibilité et teinte de bannière ; l'écart
## aux voisines (`_fit_models`) est réappliqué.
func replace_model(i: int, model: Node3D) -> void:
	var holder: Node3D = model_holder(i)
	if holder == null or model == null:
		return
	for child in holder.get_children():
		holder.remove_child(child)
		child.queue_free()
	holder.add_child(model)
	var aabb := _model_aabb(model)
	_model_radius[i] = maxf(aabb.size.x, aabb.size.z) * 0.5
	_model_top[i] = aabb.end.y
	for geometry in model.find_children("*", "GeometryInstance3D", true, false):
		var g := geometry as GeometryInstance3D
		g.visibility_range_end = tiers.model_range
		g.visibility_range_end_margin = tiers.model_range * 0.15
	if i < _colors.size():
		ModelLibrary.tint_banner(holder, _colors[i])
	_ground_model(i)
	_update_label_heights()


## Hameau brûlé (même tirage que `_build_hamlets`), pour les fumées d'incendie.
func hamlet_burned(h: int) -> bool:
	var hamlet: Dictionary = data.hamlets[h]
	var px: Vector2 = hamlet["px"]
	var seed_value := _hash(str(hamlet["name"]) + str(px))
	var devastation: float = _devastation.get(hamlet["province"], 0.0)
	return devastation >= BURN_THRESHOLD and float((seed_value / 7) % 100) < devastation


## Force une dévastation affichée (captures CV1 `--devastate`) et reconstruit les hameaux.
func override_devastation(values: Dictionary) -> void:
	for province_id in values:
		_devastation[province_id] = float(values[province_id])
	for index in _hamlet_nodes:
		_hamlet_dirty[index] = true


# --- Lot ZG6 : villes ordinaires à l'échelle réelle ------------------------------------------


func _setup_towns() -> void:
	towns = TownLayer.new()
	add_child(towns)
	var ids: Array = []
	for entry in data.settlements:
		ids.append(entry["id"])
	towns.setup(map_data, terrain, tiers, ids)


## Rendu 1:1 aux paliers vallée / site. Tant qu'il est actif, les maquettes à la loupe des
## colonies ordinaires sont masquées (au loin, une ville vraie de 1340 n'est qu'une tache : les
## maquettes géantes à l'horizon disparaissent) ; les villes emblématiques restent au lot VH.
func _update_towns(camera_distance: float) -> void:
	if towns == null:
		return
	var was_active := towns.active
	towns.update_view(camera_distance)
	if towns.version == _towns_version:
		return
	_towns_version = towns.version
	if was_active != towns.active:
		_models_root.visible = _weights.x > 0.35 and not _site_hidden and not towns.active
		_update_label_heights()


func _towns_active() -> bool:
	return towns != null and towns.active
