class_name LifeEffects
extends Node3D

## Lot CV1 : effets vivants de la carte (rendu seulement) : fumées de cheminée au-dessus des
## colonies et des hameaux (plus fournies l'hiver), fumées d'incendie des hameaux brûlés et des
## villes assiégées, vols d'oiseaux, bateaux. Instanciés par `MultiMesh` (un appel de rendu par
## famille), visibles au palier près (fumées d'incendie jusqu'au palier moyen).

const SMOKE_SHADER := preload("res://shaders/life_smoke.gdshader")
const OVERLAY_SHADER := preload("res://shaders/life_overlay.gdshader")
const WINDMILL_SHADER := preload("res://shaders/life_windmill.gdshader")
## Moulins à vent par type de colonie, échelle monde, position du moyeu (repère du corps).
const WINDMILLS := {"city": 2, "town": 1, "village": 1}
const WINDMILL_SCALE := 6.0
const WINDMILL_HUB := Vector3(0.0, 0.3, 0.08)
## Dévastation (%) à partir de laquelle villages et bourgs sont en ruine, et ruine maximale.
const RUIN_MIN_DEVASTATION := 45.0
## Panaches de cheminée par type de colonie.
const CHIMNEYS := {"city": 5, "town": 3, "village": 2, "abbey": 2, "castle": 1}
## Taille d'un panache de cheminée (largeur, hauteur) et d'incendie (unités monde).
const CHIMNEY_SIZE := Vector2(1.3, 4.0)
const FIRE_SIZE := Vector2(3.0, 13.0)
## Dévastation (%) au-delà de laquelle les hameaux brûlés fument encore.
const FIRE_MIN_DEVASTATION := 25.0

var stats: Dictionary = {}
var near_weight: float = 0.0

var _layer: SettlementLayer = null
var _terrain: TerrainBuilder = null
var _chimneys: MultiMeshInstance3D
var _fires: MultiMeshInstance3D
var _chimney_material: ShaderMaterial
var _fire_material: ShaderMaterial
## Points des panaches : [Vector2 px, hauteur au-dessus du sol, graine].
var _chimney_points: Array = []
var _fire_points: Array = []
var _reground_timer := -1.0
var _windmill_bodies: MultiMeshInstance3D
var _windmill_sails: MultiMeshInstance3D
## Moulins : [Vector2 px, lacet, graine, tourne (bool)].
var _windmill_points: Array = []
var _overlay: ShaderMaterial
var _overlay_timer := 0.0
var _ruin_overlays: Dictionary = {}
## Ruine (0-1) par indice de colonie.
var _ruin: Dictionary = {}
var _season_boost := 1.0
var _snow := 0.0


func setup(layer: SettlementLayer, terrain: TerrainBuilder) -> void:
	_layer = layer
	_terrain = terrain
	_chimney_material = _smoke_material(0.55)
	_fire_material = _smoke_material(0.9)
	_chimneys = _make_instance("Chimneys", _chimney_material)
	_fires = _make_instance("Fires", _fire_material)
	_windmill_bodies = _make_instance("WindmillBodies", null)
	_windmill_bodies.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_ON
	var sails_material := ShaderMaterial.new()
	sails_material.shader = WINDMILL_SHADER
	_windmill_sails = _make_instance("WindmillSails", sails_material)
	_windmill_sails.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_ON
	_overlay = ShaderMaterial.new()
	_overlay.shader = OVERLAY_SHADER
	if terrain != null and not terrain.chunk_surface_changed.is_connected(_on_surface_changed):
		terrain.chunk_surface_changed.connect(_on_surface_changed)


func _smoke_material(opacity: float) -> ShaderMaterial:
	var material := ShaderMaterial.new()
	material.shader = SMOKE_SHADER
	material.set_shader_parameter("fade", opacity)
	material.render_priority = 1
	return material


func _make_instance(node_name: String, material: ShaderMaterial) -> MultiMeshInstance3D:
	var mmi := MultiMeshInstance3D.new()
	mmi.name = node_name
	if material != null:
		mmi.material_override = material
	mmi.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	mmi.extra_cull_margin = 16.0
	add_child(mmi)
	return mmi


## Recalcule les panaches : `province_states` id → {devastation, siege}.
func rebuild(province_states: Dictionary) -> void:
	if _layer == null or _layer.data == null:
		return
	_chimney_points.clear()
	_fire_points.clear()
	var data := _layer.data
	for i in data.settlements.size():
		var entry: Dictionary = data.settlements[i]
		var kind := str(entry["kind"])
		var px: Vector2 = entry["px"]
		var seed_value := absi(str(entry["id"]).hash())
		var state: Dictionary = province_states.get(str(entry["province"]), {})
		var radius := _layer.model_radius(i) * 0.55
		var top := _layer.model_top(i) * 0.45
		for k in int(CHIMNEYS.get(kind, 1)):
			var angle := float((seed_value / (k + 3)) % 628) / 100.0
			var r := radius * float((seed_value / (k + 7)) % 100) / 100.0
			_chimney_points.append([px + Vector2(cos(angle), sin(angle)) * r, top, float((seed_value / (k + 11)) % 1000) / 1000.0])
		# Ville assiégée : incendies dans les faubourgs.
		if bool(state.get("siege", false)) and (kind == "city" or kind == "town"):
			for k in 3:
				var angle_f := float((seed_value / (k + 5)) % 628) / 100.0
				_fire_points.append([px + Vector2(cos(angle_f), sin(angle_f)) * _layer.model_radius(i) * 0.9, 0.2, float(k) / 3.0])
	for h in data.hamlets.size():
		var hamlet: Dictionary = data.hamlets[h]
		var hpx: Vector2 = hamlet["px"]
		var hseed := absi((str(hamlet["name"]) + str(hpx)).hash())
		var devastation := float(province_states.get(str(hamlet["province"]), {}).get("devastation", 0.0))
		if _layer.hamlet_burned(h):
			if devastation >= FIRE_MIN_DEVASTATION and hseed % 3 == 0:
				_fire_points.append([hpx, 0.1, float(hseed % 1000) / 1000.0])
		elif hseed % 2 == 0:
			_chimney_points.append([hpx, 0.35, float(hseed % 1000) / 1000.0])
	_fill(_chimneys, _chimney_points, CHIMNEY_SIZE, 0.0)
	_fill(_fires, _fire_points, FIRE_SIZE, 1.0)
	_build_windmills(province_states)
	_apply_ruins(province_states)
	stats = {"chimneys": _chimney_points.size(), "fires": _fire_points.size(), "windmills": _windmill_points.size(), "ruins": _ruin.size()}


## Moulins à vent sur la couronne de champs des colonies ; ailes arrêtées en pays dévasté.
func _build_windmills(province_states: Dictionary) -> void:
	_windmill_points.clear()
	var data := _layer.data
	for i in data.settlements.size():
		var entry: Dictionary = data.settlements[i]
		var count := int(WINDMILLS.get(str(entry["kind"]), 0))
		var px: Vector2 = entry["px"]
		var seed_value := absi((str(entry["id"]) + "mill").hash())
		var devastation := float(province_states.get(str(entry["province"]), {}).get("devastation", 0.0))
		for k in count:
			var angle := float((seed_value / (k + 2)) % 628) / 100.0
			var distance := _layer.model_radius(i) * (1.35 + float((seed_value / (k + 5)) % 60) / 100.0)
			_windmill_points.append([px + Vector2(cos(angle), sin(angle)) * distance, float((seed_value / (k + 9)) % 628) / 100.0, float(seed_value % 1000) / 1000.0, devastation < RUIN_MIN_DEVASTATION])
	var body_mesh := _first_mesh("settlements/windmill_body")
	var sails_mesh := _first_mesh("settlements/windmill_sails")
	if body_mesh == null or sails_mesh == null:
		return
	var bodies := MultiMesh.new()
	bodies.transform_format = MultiMesh.TRANSFORM_3D
	bodies.mesh = body_mesh
	bodies.instance_count = _windmill_points.size()
	var sails := MultiMesh.new()
	sails.transform_format = MultiMesh.TRANSFORM_3D
	sails.use_custom_data = true
	sails.mesh = sails_mesh
	sails.instance_count = _windmill_points.size()
	for n in _windmill_points.size():
		var point: Array = _windmill_points[n]
		var xforms := _windmill_transforms(point)
		bodies.set_instance_transform(n, xforms[0])
		sails.set_instance_transform(n, xforms[1])
		sails.set_instance_custom_data(n, Color(float(point[2]), 0.9 if bool(point[3]) else 0.0, 0.0, 0.0))
	_windmill_bodies.multimesh = bodies
	_windmill_sails.multimesh = sails


func _windmill_transforms(point: Array) -> Array:
	var px: Vector2 = point[0]
	var y := _terrain.surface_height_at(px.x, px.y) if _terrain != null else 0.0
	var basis := Basis(Vector3.UP, float(point[1])).scaled(Vector3.ONE * WINDMILL_SCALE)
	var body := Transform3D(basis, Vector3(px.x, y - 0.05, px.y))
	return [body, Transform3D(basis, body * WINDMILL_HUB)]


static func _first_mesh(model_name: String) -> Mesh:
	var scene := ModelLibrary.get_scene(model_name)
	if scene == null:
		return null
	var root := scene.instantiate()
	var meshes := root.find_children("*", "MeshInstance3D", true, false)
	var mesh: Mesh = (meshes[0] as MeshInstance3D).mesh if not meshes.is_empty() else null
	root.free()
	return mesh


## Ruines : villages et bourgs des provinces dévastées (suie, toits effondrés), un peu de
## suie sur les villes assiégées. Surcouche partagée (neige l'hiver) sur toutes les maquettes.
func _apply_ruins(province_states: Dictionary) -> void:
	_ruin.clear()
	var data := _layer.data
	for i in data.settlements.size():
		var entry: Dictionary = data.settlements[i]
		var state: Dictionary = province_states.get(str(entry["province"]), {})
		var devastation := float(state.get("devastation", 0.0))
		var kind := str(entry["kind"])
		var amount := 0.0
		if kind == "village" or kind == "abbey":
			amount = clampf((devastation - RUIN_MIN_DEVASTATION + 10.0) / 50.0, 0.0, 0.85)
		elif devastation >= RUIN_MIN_DEVASTATION:
			amount = clampf((devastation - RUIN_MIN_DEVASTATION) / 120.0, 0.0, 0.35)
		if bool(state.get("siege", false)):
			amount = maxf(amount, 0.3)
		if amount > 0.0:
			_ruin[i] = amount
			if kind == "village":
				# Village en ruine : fumée d'incendie au-dessus.
				_fire_points.append([entry["px"], 0.3, float(i % 97) / 97.0])
	_fill(_fires, _fire_points, FIRE_SIZE, 1.0)
	_update_overlays()


## Surcouche pour un degré de ruine (paliers de 0,2 : peu de matériaux distincts).
func _overlay_for(amount: float) -> ShaderMaterial:
	var step := int(round(amount * 5.0))
	if step == 0:
		return _overlay
	if not _ruin_overlays.has(step):
		var material := ShaderMaterial.new()
		material.shader = OVERLAY_SHADER
		material.set_shader_parameter("ruin", step / 5.0)
		_ruin_overlays[step] = material
	return _ruin_overlays[step]


## Pose la surcouche (neige, suie) sur les maquettes et les hameaux, y compris ceux reconstruits
## depuis (appelé périodiquement). Seulement là où elle sert (hiver, ruine) : c'est une passe de
## rendu de plus par maquette.
func _update_overlays() -> void:
	if _layer == null or _layer.data == null:
		return
	var snowy := _snow > 0.01
	for i in _layer.data.settlements.size():
		var holder := _layer.model_holder(i)
		if holder == null:
			continue
		var amount: float = _ruin.get(i, 0.0)
		var wanted: ShaderMaterial = _overlay_for(amount) if snowy or amount > 0.0 else null
		for geometry in holder.find_children("*", "GeometryInstance3D", true, false):
			var g := geometry as GeometryInstance3D
			if g.material_overlay != wanted:
				g.material_overlay = wanted
	var hamlets := _layer.get_node_or_null("Hamlets")
	if hamlets != null:
		var wanted_h: ShaderMaterial = _overlay if snowy else null
		for node in hamlets.get_children():
			for mmi in node.get_children():
				var g_h := mmi as GeometryInstance3D
				if g_h != null and g_h.material_overlay != wanted_h:
					g_h.material_overlay = wanted_h


func _fill(mmi: MultiMeshInstance3D, points: Array, size: Vector2, darkness: float) -> void:
	var quad := QuadMesh.new()
	quad.size = Vector2.ONE
	quad.center_offset = Vector3(0.0, 0.5, 0.0)
	var multimesh := MultiMesh.new()
	multimesh.transform_format = MultiMesh.TRANSFORM_3D
	multimesh.use_custom_data = true
	multimesh.mesh = quad
	multimesh.instance_count = points.size()
	for n in points.size():
		var point: Array = points[n]
		var seed_value: float = point[2]
		var s := 0.8 + 0.4 * seed_value
		var basis := Basis.from_scale(Vector3(size.x * s, size.y * s, 1.0))
		multimesh.set_instance_transform(n, Transform3D(basis, _ground(point)))
		multimesh.set_instance_custom_data(n, Color(seed_value, darkness, 0.75 + 0.25 * fposmod(seed_value * 7.0, 1.0), fposmod(seed_value * 13.0, 1.0)))
	mmi.multimesh = multimesh


func _ground(point: Array) -> Vector3:
	var px: Vector2 = point[0]
	var y := _terrain.surface_height_at(px.x, px.y) if _terrain != null else 0.0
	return Vector3(px.x, y + float(point[1]), px.y)


func _reground() -> void:
	if _windmill_bodies.multimesh != null and _windmill_sails.multimesh != null:
		for n in _windmill_points.size():
			var xforms := _windmill_transforms(_windmill_points[n])
			_windmill_bodies.multimesh.set_instance_transform(n, xforms[0])
			_windmill_sails.multimesh.set_instance_transform(n, xforms[1])
	for pair in [[_chimneys, _chimney_points], [_fires, _fire_points]]:
		var mmi: MultiMeshInstance3D = pair[0]
		var points: Array = pair[1]
		if mmi.multimesh == null:
			continue
		for n in points.size():
			var xform := mmi.multimesh.get_instance_transform(n)
			xform.origin = _ground(points[n])
			mmi.multimesh.set_instance_transform(n, xform)


func _on_surface_changed(_index: int) -> void:
	if _reground_timer < 0.0:
		_reground_timer = 0.4


## Plus de feux de cheminée l'hiver et à l'automne (poids de saison x, y, z, w).
func set_season(weights: Vector4) -> void:
	_snow = weights.w
	_season_boost = 0.55 * weights.y + 0.8 * weights.x + 0.95 * weights.z + 1.15 * weights.w


func update_view(camera_distance: float, tiers: ZoomTiers) -> void:
	near_weight = tiers.near_weight(camera_distance) if tiers != null else 1.0
	var medium := tiers.medium_weight(camera_distance) if tiers != null else 0.0
	var chimney_alpha := near_weight * _season_boost * 0.85
	_chimneys.visible = chimney_alpha > 0.02
	_chimney_material.set_shader_parameter("fade", chimney_alpha)
	var fire_alpha := clampf(near_weight + medium * 0.8, 0.0, 1.0) * 0.9
	_fires.visible = fire_alpha > 0.02
	_fire_material.set_shader_parameter("fade", fire_alpha)
	var show_models := near_weight > 0.35
	_windmill_bodies.visible = show_models
	_windmill_sails.visible = show_models
	_overlay_timer -= get_process_delta_time()
	if show_models and _overlay_timer <= 0.0:
		_overlay_timer = 0.5
		_update_overlays()
	if _reground_timer >= 0.0:
		_reground_timer -= get_process_delta_time()
		if _reground_timer < 0.0:
			_reground()
