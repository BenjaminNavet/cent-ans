class_name LifeEffects
extends Node3D

## Lot CV1 : effets vivants de la carte (rendu seulement) : fumées de cheminée au-dessus des
## colonies et des hameaux (plus fournies l'hiver), fumées d'incendie des hameaux brûlés et des
## villes assiégées, vols d'oiseaux, bateaux. Instanciés par `MultiMesh` (un appel de rendu par
## famille), visibles au palier près (fumées d'incendie jusqu'au palier moyen).
## Lot SZ4 : sous le palier comté, moulins et fumées passent continûment de leur taille de carte à
## leur taille réelle (`MapPropScale`), et restent affichés jusqu'au palier site.

const SMOKE_SHADER := preload("res://shaders/life_smoke.gdshader")
const OVERLAY_SHADER := preload("res://shaders/life_overlay.gdshader")
const WINDMILL_SHADER := preload("res://shaders/life_windmill.gdshader")
## Moulins à vent par type de colonie, échelle monde, position du moyeu (repère du corps).
const WINDMILLS := {"city": 2, "town": 1, "village": 1}
const WINDMILL_SCALE := 4.6
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
## Points des panaches : [Vector2 px (courant), hauteur au-dessus du sol, graine, colonie (-1 :
## hameau), décalage au centre à l'échelle de la carte, sol à la pose de carte, sol à la pose réelle]
## (lot SZ4b : les points d'une colonie suivent l'échelle de sa maquette, `_settlement_pose`).
var _chimney_points: Array = []
var _fire_points: Array = []
var _reground_timer := -1.0
var _reground_chunks: Dictionary = {}
var _last_rebuild_key := ""
var _windmill_bodies: MultiMeshInstance3D
var _windmill_sails: MultiMeshInstance3D
## Moulins : [Vector2 px (courant), lacet, graine, tourne (bool), hauteur du sol courante (SZ4),
## colonie, décalage à l'échelle de la carte, sol à la pose de carte, sol à la pose réelle (SZ4b)].
var _windmill_points: Array = []
var _overlay: ShaderMaterial
var _overlay_snowy := false
var _ruin_overlays: Dictionary = {}
## Ruine (0-1) par indice de colonie.
var _ruin: Dictionary = {}
var _season_boost := 1.0
var _snow := 0.0
## SZ4 : échelles appliquées (moulins : instances réécrites ; fumées : paramètre du matériau).
var _windmill_scale := 1.0
var _smoke_scales := Vector2(-1.0, -1.0)
## SZ4b : échelle de référence des maquettes appliquée aux positions (pas de `rewrite_step`).
var _settlement_ref := 1.0
var _camera_distance := 1000.0
## RS-K : copies processeur des tampons MultiMesh (panaches : 12 + 4 flottants par instance ; corps
## de moulin : 12 ; ailes : 12 + 4). Lire `MultiMesh.buffer` relit le tampon du GPU de façon
## synchrone (jusqu'à 80 ms par réécriture pendant un zoom) : on écrit depuis ces copies.
var _cpu_buffers: Dictionary = {}  # MultiMeshInstance3D → PackedFloat32Array


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
	# PB1 : `CampaignLife` rappelle `rebuild` dès qu'un palier de population change ; les effets
	# ne dépendent que de la dévastation, des sièges, de la taille des maquettes et des hameaux
	# brûlés. Mêmes entrées → mêmes effets : rien à reconstruire.
	var key := _rebuild_key(province_states)
	if key == _last_rebuild_key:
		return
	_last_rebuild_key = key
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
			_chimney_points.append(_settlement_point(i, Vector2(cos(angle), sin(angle)) * r, top, float((seed_value / (k + 11)) % 1000) / 1000.0))
		# Ville assiégée : incendies dans les faubourgs.
		if bool(state.get("siege", false)) and (kind == "city" or kind == "town"):
			for k in 3:
				var angle_f := float((seed_value / (k + 5)) % 628) / 100.0
				_fire_points.append(_settlement_point(i, Vector2(cos(angle_f), sin(angle_f)) * _layer.model_radius(i) * 0.9, 0.2, float(k) / 3.0))
	for h in data.hamlets.size():
		var hamlet: Dictionary = data.hamlets[h]
		var hpx: Vector2 = hamlet["px"]
		var hseed := absi((str(hamlet["name"]) + str(hpx)).hash())
		hpx = _layer.hamlet_px(h)  # SZ4b : pose de rendu du hameau (ancrage fin ZG5b)
		var devastation := float(province_states.get(str(hamlet["province"]), {}).get("devastation", 0.0))
		if _layer.hamlet_burned(h):
			if devastation >= FIRE_MIN_DEVASTATION and hseed % 3 == 0:
				_fire_points.append(_fixed_point(hpx, 0.1, float(hseed % 1000) / 1000.0))
		elif hseed % 2 == 0:
			_chimney_points.append(_fixed_point(hpx, 0.35, float(hseed % 1000) / 1000.0))
	_settlement_ref = _reference_scale()
	_fill(_chimneys, _chimney_points, CHIMNEY_SIZE, 0.0)
	_fill(_fires, _fire_points, FIRE_SIZE, 1.0)
	_build_windmills(province_states)
	_apply_ruins(province_states)
	stats = {"chimneys": _chimney_points.size(), "fires": _fire_points.size(), "windmills": _windmill_points.size(), "ruins": _ruin.size()}


## Entrées effectives de `rebuild` : sièges, seuils de dévastation (feux, moulins, ruines),
## palier de ruine par colonie (seul `_overlay_for` le lit, par paliers de 0,2), taille et
## maquette affichée des colonies, hameaux brûlés.
func _rebuild_key(province_states: Dictionary) -> String:
	var data := _layer.data
	var parts := PackedStringArray()
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
		var siege := bool(state.get("siege", false))
		if siege:
			amount = maxf(amount, 0.3)
		var holder := _layer.model_holder(i)
		var model_id := holder.get_child(0).get_instance_id() if holder != null and holder.get_child_count() > 0 else 0
		parts.append("%d%d%d%d%d:%s:%s:%d" % [int(siege), int(devastation >= FIRE_MIN_DEVASTATION),
			int(devastation >= RUIN_MIN_DEVASTATION), int(amount > 0.0), int(round(amount * 5.0)),
			_layer.model_radius(i), _layer.model_top(i), model_id])
	var burned := PackedByteArray()
	burned.resize(data.hamlets.size())
	for h in data.hamlets.size():
		var devastation := float(province_states.get(str(data.hamlets[h]["province"]), {}).get("devastation", 0.0))
		burned[h] = (2 if _layer.hamlet_burned(h) else 0) + (1 if devastation >= FIRE_MIN_DEVASTATION else 0)
	return "%s|%s" % [",".join(parts), Marshalls.raw_to_base64(burned)]


## Moulins à vent sur la couronne de champs des colonies ; ailes arrêtées en pays dévasté.
func _build_windmills(province_states: Dictionary) -> void:
	_points_version += 1
	_windmill_points.clear()
	var data := _layer.data
	for i in data.settlements.size():
		var entry: Dictionary = data.settlements[i]
		var count := int(WINDMILLS.get(str(entry["kind"]), 0))
		var seed_value := absi((str(entry["id"]) + "mill").hash())
		var devastation := float(province_states.get(str(entry["province"]), {}).get("devastation", 0.0))
		for k in count:
			var angle := float((seed_value / (k + 2)) % 628) / 100.0
			var distance := _layer.model_radius(i) * (1.35 + float((seed_value / (k + 5)) % 60) / 100.0)
			var pose := _settlement_point(i, Vector2(cos(angle), sin(angle)) * distance, 0.0, 0.0)
			_windmill_points.append([pose[0], float((seed_value / (k + 9)) % 628) / 100.0, float(seed_value % 1000) / 1000.0,
				devastation < RUIN_MIN_DEVASTATION, _pose_y(pose), i, pose[4], pose[5], pose[6]])
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
	var body_buffer := PackedFloat32Array()
	body_buffer.resize(_windmill_points.size() * 12)
	var sail_buffer := PackedFloat32Array()
	sail_buffer.resize(_windmill_points.size() * 16)
	for n in _windmill_points.size():
		var point: Array = _windmill_points[n]
		var xforms := _windmill_transforms(point)
		_write_transform(body_buffer, n * 12, xforms[0])
		_write_transform(sail_buffer, n * 16, xforms[1])
		_write_custom(sail_buffer, n * 16 + 12, Color(float(point[2]), 0.9 if bool(point[3]) else 0.0, 0.0, 0.0))
	bodies.buffer = body_buffer
	sails.buffer = sail_buffer
	_windmill_bodies.multimesh = bodies
	_windmill_sails.multimesh = sails
	_cpu_buffers[_windmill_bodies] = body_buffer
	_cpu_buffers[_windmill_sails] = sail_buffer


## RS-K : transformation au format du tampon MultiMesh (3 lignes de base + origine).
static func _write_transform(buffer: PackedFloat32Array, o: int, t: Transform3D) -> void:
	buffer[o] = t.basis.x.x
	buffer[o + 1] = t.basis.y.x
	buffer[o + 2] = t.basis.z.x
	buffer[o + 3] = t.origin.x
	buffer[o + 4] = t.basis.x.y
	buffer[o + 5] = t.basis.y.y
	buffer[o + 6] = t.basis.z.y
	buffer[o + 7] = t.origin.y
	buffer[o + 8] = t.basis.x.z
	buffer[o + 9] = t.basis.y.z
	buffer[o + 10] = t.basis.z.z
	buffer[o + 11] = t.origin.z


static func _write_custom(buffer: PackedFloat32Array, o: int, c: Color) -> void:
	buffer[o] = c.r
	buffer[o + 1] = c.g
	buffer[o + 2] = c.b
	buffer[o + 3] = c.a


## RS-K : moulins `indices` réécrits dans les copies processeur, puis envoyés en un bloc.
func _write_windmills(indices: Array) -> void:
	var body_buffer: PackedFloat32Array = _cpu_buffers.get(_windmill_bodies, PackedFloat32Array())
	var sail_buffer: PackedFloat32Array = _cpu_buffers.get(_windmill_sails, PackedFloat32Array())
	if body_buffer.size() != _windmill_points.size() * 12 or sail_buffer.size() != _windmill_points.size() * 16:
		for n: int in indices:
			var xforms := _windmill_transforms(_windmill_points[n])
			_windmill_bodies.multimesh.set_instance_transform(n, xforms[0])
			_windmill_sails.multimesh.set_instance_transform(n, xforms[1])
		return
	for n: int in indices:
		var xforms := _windmill_transforms(_windmill_points[n])
		_write_transform(body_buffer, n * 12, xforms[0])
		_write_transform(sail_buffer, n * 16, xforms[1])
	_windmill_bodies.multimesh.buffer = body_buffer
	_windmill_sails.multimesh.buffer = sail_buffer
	_cpu_buffers[_windmill_bodies] = body_buffer
	_cpu_buffers[_windmill_sails] = sail_buffer


func _surface_y(px: Vector2) -> float:
	return _terrain.surface_height_at(px.x, px.y) if _terrain != null else 0.0


## SZ4b : point lié à la colonie `i` : décalage `offset` (échelle de la carte) autour de la maquette
## ancrée, mis à l'échelle de la maquette ; sol gardé aux deux poses extrêmes.
func _settlement_point(i: int, offset: Vector2, lift: float, seed_value: float) -> Array:
	var center := _layer.model_px(i)
	var ratio := _layer.model_scale_at(i, 0.0)
	var point := [center, lift, seed_value, i, offset, _surface_y(center + offset), _surface_y(center + offset * ratio)]
	_settlement_pose(point)
	return point


## Point fixe (hameau) au format des points de colonie.
func _fixed_point(px: Vector2, lift: float, seed_value: float) -> Array:
	var y := _surface_y(px)
	return [px, lift, seed_value, -1, Vector2.ZERO, y, y]


## SZ4b : position courante (px) d'un point de colonie à l'échelle de sa maquette.
func _settlement_pose(point: Array) -> void:
	var i: int = point[3]
	if i < 0:
		return
	var s := _layer.model_scale_at(i, _camera_distance)
	point[0] = _layer.model_px(i) + (point[4] as Vector2) * s


## SZ4b : sol courant d'un point (interpolé entre la pose réelle et la pose de carte).
func _pose_y(point: Array) -> float:
	var i: int = point[3]
	if i < 0:
		return point[5]
	var s := _layer.model_scale_at(i, _camera_distance)
	var ratio := _layer.model_scale_at(i, 0.0)
	var along := clampf((s - ratio) / maxf(1.0 - ratio, 1e-4), 0.0, 1.0)
	return lerpf(float(point[6]), float(point[5]), along)


## SZ4b : exagération commune à la distance courante (réécriture des positions par pas).
func _reference_scale() -> float:
	var props := MapPropScale.shared()
	return props.exaggeration(_camera_distance)  # exagération commune (lot SZ4b)


func _windmill_transforms(point: Array) -> Array:
	var px: Vector2 = point[0]
	var y: float = point[4]
	var basis := Basis(Vector3.UP, float(point[1])).scaled(Vector3.ONE * WINDMILL_SCALE * _windmill_scale)
	var body := Transform3D(basis, Vector3(px.x, y - 0.05 * _windmill_scale, px.y))
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
				_fire_points.append(_settlement_point(i, Vector2.ZERO, 0.3, float(i % 97) / 97.0))
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


## Pose la surcouche (neige, suie) sur les maquettes des colonies, seulement là où elle sert
## (hiver, ruine) : c'est une passe de rendu de plus par maquette. Les hameaux (petits, nombreux,
## reconstruits par tuile) n'en ont pas. Rappelée quand l'hiver arrive ou s'en va.
func _update_overlays() -> void:
	if _layer == null or _layer.data == null:
		return
	var snowy := _snow > 0.01
	_overlay_snowy = snowy
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


func _fill(mmi: MultiMeshInstance3D, points: Array, size: Vector2, darkness: float) -> void:
	_points_version += 1
	var quad := QuadMesh.new()
	quad.size = Vector2.ONE
	quad.center_offset = Vector3(0.0, 0.5, 0.0)
	var multimesh := MultiMesh.new()
	multimesh.transform_format = MultiMesh.TRANSFORM_3D
	multimesh.use_custom_data = true
	multimesh.mesh = quad
	multimesh.instance_count = points.size()
	var buffer := PackedFloat32Array()
	buffer.resize(points.size() * 16)
	for n in points.size():
		var point: Array = points[n]
		var seed_value: float = point[2]
		var s := 0.8 + 0.4 * seed_value
		# SZ4 : origine au sol, levée dans la colonne z (lue et mise à l'échelle par le shader).
		var basis := Basis(Vector3(size.x * s, 0.0, 0.0), Vector3(0.0, size.y * s, 0.0), Vector3(0.0, float(point[1]), 1.0))
		_write_transform(buffer, n * 16, Transform3D(basis, _origin(point)))
		_write_custom(buffer, n * 16 + 12, Color(seed_value, darkness, 0.75 + 0.25 * fposmod(seed_value * 7.0, 1.0), fposmod(seed_value * 13.0, 1.0)))
	multimesh.buffer = buffer
	mmi.multimesh = multimesh
	_cpu_buffers[mmi] = buffer


## Pied d'un panache (au sol ; la levée `point[1]` est portée par la base d'instance, SZ4) ;
## SZ4b : à la pose courante de sa colonie.
func _origin(point: Array) -> Vector3:
	var px: Vector2 = point[0]
	return Vector3(px.x, _pose_y(point), px.y)


## Relit le sol des deux poses d'un point (surface affichée changée).
func _reground_point(point: Array) -> void:
	var i: int = point[3]
	if i < 0:
		point[5] = _surface_y(point[0])
		point[6] = point[5]
		return
	var center := _layer.model_px(i)
	var offset: Vector2 = point[4]
	point[5] = _surface_y(center + offset)
	point[6] = _surface_y(center + offset * _layer.model_scale_at(i, 0.0))


## PB1 : seuls les points des tuiles dont la surface a changé (`_reground_chunks`) sont recalés :
## la hauteur d'un point ne dépend que de sa tuile. SZ6 : points indexés par tuile (plus de
## parcours de tous les points à chaque recalage), tampon lu et écrit seulement si un point bouge.
func _reground() -> void:
	var changed := _reground_chunks
	_reground_chunks = {}
	if _windmill_bodies.multimesh != null and _windmill_sails.multimesh != null:
		var mills := _changed_points("windmills", _windmill_points, changed)
		for n: int in mills:
			_reground_windmill(_windmill_points[n])
		if not mills.is_empty():
			_write_windmills(mills)
	for triple in [[_chimneys, _chimney_points, "chimneys"], [_fires, _fire_points, "fires"]]:
		var mmi: MultiMeshInstance3D = triple[0]
		var points: Array = triple[1]
		if mmi.multimesh == null:
			continue
		var todo := _changed_points(triple[2], points, changed)
		if todo.is_empty():
			continue
		# Tampon complet écrit une fois (12 flottants de transformation + 4 de données
		# personnalisées par instance ; hauteur de l'origine au rang 7) plutôt que deux appels au
		# serveur de rendu par point. RS-K : depuis la copie processeur (pas de relecture du GPU).
		var buffer: PackedFloat32Array = _cpu_buffers.get(mmi, PackedFloat32Array())
		var stride := 16
		if buffer.size() != points.size() * stride:
			# Tampon absent (maillage reconstruit ailleurs) : repli point par point.
			for n: int in todo:
				_reground_point(points[n])
				var xform := mmi.multimesh.get_instance_transform(n)
				xform.origin = _origin(points[n])
				mmi.multimesh.set_instance_transform(n, xform)
			continue
		for n: int in todo:
			_reground_point(points[n])
			buffer[n * stride + 7] = _pose_y(points[n])
		mmi.multimesh.buffer = buffer
		_cpu_buffers[mmi] = buffer


## SZ6 : index par tuile des points (`_windmill_points`, `_chimney_points`, `_fire_points`),
## refait quand les listes sont reconstruites (`_points_version`).
var _points_version := 0
var _points_by_chunk: Dictionary = {}


## Indices croissants des points de `points` situés dans une tuile de `changed` (tous sans terrain).
func _changed_points(list_name: String, points: Array, changed: Dictionary) -> Array:
	if _terrain == null:
		return range(points.size())
	var cached: Dictionary = _points_by_chunk.get(list_name, {})
	if int(cached.get("version", -1)) != _points_version or int(cached.get("size", -1)) != points.size():
		var index_of: Dictionary = {}
		for n in points.size():
			var px := _anchor_of(points[n])
			var index := _terrain.chunk_index_at(px.x, px.y)
			if not index_of.has(index):
				index_of[index] = []
			(index_of[index] as Array).append(n)
		cached = {"version": _points_version, "size": points.size(), "by_chunk": index_of}
		_points_by_chunk[list_name] = cached
	var by_chunk: Dictionary = cached["by_chunk"]
	var result: Array = []
	for index: int in changed:
		result.append_array(by_chunk.get(index, []))
	result.sort()
	return result


## SZ4b : point de rattachement d'un point à la tuile (centre de la colonie, fixe quand l'échelle
## de la maquette varie ; sinon le point lui-même).
func _anchor_of(point: Array) -> Vector2:
	var i := -1
	if point.size() >= 9:
		i = point[5]  # moulin : colonie au rang 5
	elif point.size() >= 7:
		i = point[3]  # panache
	return _layer.model_px(i) if i >= 0 else point[0]


## SZ4b : moulin au format des points de colonie (px, -, -, colonie, décalage, sols).
static func _mill_pose(point: Array) -> Array:
	return [point[0], 0.0, 0.0, point[5], point[6], point[7], point[8]]


## Moulin : sol des deux poses relu, sol courant recalculé.
func _reground_windmill(point: Array) -> void:
	var pose := _mill_pose(point)
	_reground_point(pose)
	point[7] = pose[5]
	point[8] = pose[6]
	point[4] = _pose_y(pose)


func _on_surface_changed(index: int) -> void:
	_reground_chunks[index] = true
	if _reground_timer < 0.0:
		_reground_timer = 0.4


## Plus de feux de cheminée l'hiver et à l'automne (poids de saison x, y, z, w).
func set_season(weights: Vector4) -> void:
	_snow = weights.w
	_season_boost = 0.55 * weights.y + 0.8 * weights.x + 0.95 * weights.z + 1.15 * weights.w


## SZ4 : échelle des panaches (paramètre des matériaux) et des moulins (instances réécrites par
## pas de `MapPropScale.rewrite_step`).
func _apply_prop_scale(camera_distance: float) -> void:
	var props := MapPropScale.shared()
	var smoke := Vector2(props.chimney_scale(camera_distance), props.fire_scale(camera_distance))
	if not smoke.is_equal_approx(_smoke_scales):
		_smoke_scales = smoke
		_chimney_material.set_shader_parameter("prop_scale", smoke.x)
		_fire_material.set_shader_parameter("prop_scale", smoke.y)
	_camera_distance = camera_distance
	var mill := props.windmill_scale(camera_distance)
	var reference := _reference_scale()
	var moved := props.needs_rewrite(_settlement_ref, reference)
	var tp := Time.get_ticks_usec()  # RS-K : sous-sections du banc `--bench-probe`
	if moved:
		# SZ4b : moulins et panaches suivent l'échelle de la maquette de leur colonie.
		_settlement_ref = reference
		_rewrite_smoke_positions()
	tp = PerfProbe.lap("life/smoke_rewrite", tp)
	if moved or props.needs_rewrite(_windmill_scale, mill):
		_windmill_scale = mill
		_rewrite_windmills()
	PerfProbe.lap("life/mill_rewrite", tp)


func _rewrite_windmills() -> void:
	if _windmill_bodies.multimesh == null or _windmill_sails.multimesh == null:
		return
	if _windmill_bodies.multimesh.instance_count != _windmill_points.size():
		return
	for n in _windmill_points.size():
		var point: Array = _windmill_points[n]
		var pose := _mill_pose(point)
		_settlement_pose(pose)
		point[0] = pose[0]
		point[4] = _pose_y(pose)
	_write_windmills(range(_windmill_points.size()))


## SZ4b : origines des panaches des colonies à l'échelle courante de leur maquette (tampon complet
## écrit une fois depuis sa copie processeur, RS-K ; repli point par point sans copie).
func _rewrite_smoke_positions() -> void:
	for pair in [[_chimneys, _chimney_points], [_fires, _fire_points]]:
		var mmi: MultiMeshInstance3D = pair[0]
		var points: Array = pair[1]
		if mmi.multimesh == null or mmi.multimesh.instance_count != points.size():
			continue
		var buffer: PackedFloat32Array = _cpu_buffers.get(mmi, PackedFloat32Array())
		var stride := 16
		var readable := buffer.size() == points.size() * stride
		var touched := false
		for n in points.size():
			var point: Array = points[n]
			if int(point[3]) < 0:
				continue
			_settlement_pose(point)
			var origin := _origin(point)
			if readable:
				buffer[n * stride + 3] = origin.x
				buffer[n * stride + 7] = origin.y
				buffer[n * stride + 11] = origin.z
				touched = true
			else:
				var xform := mmi.multimesh.get_instance_transform(n)
				xform.origin = origin
				mmi.multimesh.set_instance_transform(n, xform)
		if touched:
			mmi.multimesh.buffer = buffer
			_cpu_buffers[mmi] = buffer


func update_view(camera_distance: float, tiers: ZoomTiers) -> void:
	near_weight = tiers.near_weight(camera_distance) if tiers != null else 1.0
	var medium := tiers.medium_weight(camera_distance) if tiers != null else 0.0
	# SZ4 : fumées, feux et moulins à leur taille réelle sous le palier comté (ZG4 les masquait au
	# palier site, à leur taille de carte : des colonnes de plusieurs kilomètres).
	_apply_prop_scale(camera_distance)
	var chimney_alpha := near_weight * _season_boost * 0.85 * MapPropScale.shared().chimney_alpha(camera_distance)
	_chimneys.visible = chimney_alpha > 0.02
	_chimney_material.set_shader_parameter("fade", chimney_alpha)
	var fire_alpha := clampf(near_weight + medium * 0.8, 0.0, 1.0) * 0.9
	_fires.visible = fire_alpha > 0.02
	_fire_material.set_shader_parameter("fade", fire_alpha)
	var show_models := near_weight > 0.35
	_windmill_bodies.visible = show_models
	_windmill_sails.visible = show_models
	if (_snow > 0.01) != _overlay_snowy:
		_update_overlays()
	if _reground_timer >= 0.0:
		_reground_timer -= get_process_delta_time()
		if _reground_timer < 0.0:
			var tp := Time.get_ticks_usec()
			_reground()
			PerfProbe.lap("life/reground", tp)  # RS-K
