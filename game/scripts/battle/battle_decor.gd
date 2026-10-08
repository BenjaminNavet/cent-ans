class_name BattleDecor
extends Node3D

## Lot EP6 : décor du champ de bataille posé par le cœur (`get_terrain().decor`) : bâtiments des
## hameaux, fermes, moulins, église, manoir (kit Blender BR1), murs de cimetière et porche,
## tombes, fossé en eau du manoir, rangs de vigne, meules, charrettes, puits, bûchers ; camp de
## chaque armée (tentes, pavillons, chariots, feux, chevaux au piquet) et convoi de bagages. Les
## vergers sont plantés avec les arbres (`BattleTerrain._plant_orchards`), les labours, prés et
## sols des vignes peints dans le sol (`BattleTerrain.decor_fields`). Rendu seulement : positions,
## emprises et orientations viennent de la simulation.
##
## Performance : un `MultiMesh` par modèle et par lot (bâtiments, accessoires, camp, vigne), portées
## de visibilité (`visibility_range_end`) par taille ; les rangs de vigne ont deux niveaux de détail
## (modèle du kit de près, simple haie verte au-delà de `VINE_NEAR`).

const WATER_SHADER := preload("res://shaders/battle_water.gdshader")
const FLAME_SHADER := preload("res://shaders/battle_camp_fire.gdshader")
## Portées de visibilité (m).
const LANDMARK_RANGE := 2800.0
const HOUSE_RANGE := 1900.0
const PROP_RANGE := 750.0
const CAMP_RANGE := 1300.0
const HORSE_RANGE := 380.0
const VINE_NEAR := 170.0
const VINE_FAR := 900.0
const WALL_RANGE := 900.0
const FIRE_RANGE := 900.0

var _terrain: BattleTerrain
var _snowy := false
var _decor: Dictionary = {}
var _sim: Object = null
var _camp_timer := 0.0
## camp -> {batch, tents: [poignées], fires: [Vector3], looted}
var _camps: Dictionary = {}
var _smoke_root: Node3D = null
## EP8 : scène qui porte les sources de fumée durables (`add_smoke_source`), null sans EP8.
var _smoke_scene: Node = null
var smoke_sources := 0
var building_count := 0
var prop_count := 0
var vine_segments := 0
var horse_count := 0
var tent_count := 0
## GA3-L1 : instances remplacées par une variante générée (`Ga3Kit`), camps compris.
var ga3_count := 0


func build(terrain: BattleTerrain, decor: Dictionary, weather: String) -> void:
	_terrain = terrain
	_decor = decor
	_snowy = terrain.snowy() or weather == "snow"
	if decor.is_empty() or not BuildingKit.available():
		return
	var variant := "snow" if _snowy else ""
	var landmarks := BuildingKit.Batch.new(variant, LANDMARK_RANGE * RenderQuality.battle_lod_scale)
	var houses := BuildingKit.Batch.new(variant, HOUSE_RANGE * RenderQuality.battle_lod_scale)
	var props := BuildingKit.Batch.new("", PROP_RANGE * RenderQuality.battle_lod_scale)
	var walls := BuildingKit.Batch.new(variant, WALL_RANGE * RenderQuality.battle_lod_scale)
	_build_buildings(decor.get("buildings", []), landmarks, houses)
	_build_props(decor.get("props", []), props)
	_build_churchyards(decor.get("areas", []), walls)
	var root := Node3D.new()
	root.name = "Buildings"
	add_child(root)
	landmarks.build(root)
	houses.build(root)
	props.build(root)
	walls.build(root)
	_build_moats(decor.get("moats", []), weather)
	_build_vineyards(decor.get("areas", []), bool(decor.get("vines_leafy", true)))
	ga3_count = landmarks.ga3_count() + houses.ga3_count() + props.ga3_count()
	for camp in decor.get("camps", []):
		_build_camp(camp)
	set_process(false)
	print("BattleDecor: %s, %d buildings, %d props, %d vine segments, %d tents, %d horses, %d GA3 variants" % [
		str(decor.get("profile", "")), building_count, prop_count, vine_segments, tent_count, horse_count, ga3_count])


## Vrai si le décor pose ses propres camps (EP8 ne pose alors pas ses feux par défaut).
func has_camps() -> bool:
	return not _camps.is_empty()


## EP8 : fumée des feux de camp par `BattleScene.add_smoke_source` ; au plus
## `camp_fires_per_side` feux fumants par camp (réglage de qualité d'EP8), répartis dans le camp,
## aucun par temps de pluie ou de neige (`no_campfire_weather`).
func attach_smoke(scene: Node, staging: BattleStaging, weather: String) -> void:
	if scene == null or staging == null or staging.smoke == null or not staging.cfg.has("smoke"):
		return
	_smoke_scene = scene
	var smoke_cfg: Dictionary = staging.cfg["smoke"]
	if (smoke_cfg.get("no_campfire_weather", []) as Array).has(weather):
		return
	var per_side: Dictionary = smoke_cfg.get("camp_fires_per_side", {})
	var count := int(per_side.get(RenderQuality.current(), per_side.get("high", 2)))
	for side in _camps:
		var fires: Array = _camps[side]["fires"]
		var n := mini(count, fires.size())
		for i in n:
			var fire: Vector3 = fires[int((i + 0.5) * fires.size() / float(n))]
			var id := int(scene.call("add_smoke_source", fire + Vector3(0, 0.6, 0), 0.6 + 0.4 * BuildingKit.hash01(i, 73), "campfire"))
			if id >= 0:
				smoke_sources += 1
	print("BattleDecor: %d camp fire smoke sources" % smoke_sources)


## Relie la simulation : l'état des camps (pillage) est relu une fois par seconde.
func bind(sim: Object) -> void:
	_sim = sim
	set_process(sim != null and sim.has_method("get_camps") and not _camps.is_empty())


func _process(delta: float) -> void:
	_camp_timer += delta
	if _camp_timer < 1.0 or _sim == null:
		return
	_camp_timer = 0.0
	for state in _sim.call("get_camps"):
		var side := str(state["side"])
		if bool(state["looted"]) and _camps.has(side) and not bool(_camps[side]["looted"]):
			_loot(side)


# --- Bâtiments ----------------------------------------------------------------------------


## Type de modèle du kit pour un bâtiment du décor.
static func kit_kind(kind: String, length: float) -> String:
	match kind:
		"timbered":
			return "timber"
		"stone":
			return "stonehouse"
		"barn", "church", "windmill", "watermill", "manor":
			return kind
		_:
			return "longere" if length > 11.0 else "cottage"


## Plus bas et plus haut sol sous une emprise orientée (lacet du cœur).
func _ground_span(p: Vector2, length: float, width: float, yaw: float) -> Vector2:
	var low := INF
	var high := -INF
	for corner in [Vector2(-length, -width), Vector2(length, -width), Vector2(length, width), Vector2(-length, width), Vector2.ZERO]:
		var q: Vector2 = p + (corner * 0.5).rotated(yaw)
		var h := _terrain.height_at(q.x, q.y)
		low = minf(low, h)
		high = maxf(high, h)
	return Vector2(low, high)


func _build_buildings(buildings: Array, landmarks: BuildingKit.Batch, houses: BuildingKit.Batch) -> void:
	for b in buildings:
		var p := Vector2(float(b["x"]), float(b["z"]))
		var length := float(b["length"])
		var width := float(b["width"])
		var yaw := float(b["yaw"])
		var kind := str(b["kind"])
		var rng := RandomNumberGenerator.new()
		rng.seed = int(p.x * 31.0 + p.y * 17.0)
		var model := BuildingKit.pick(kit_kind(kind, length), length, width, rng)
		if model == "":
			continue
		var span := _ground_span(p, length, width, yaw)
		# Le cœur oriente déjà la façade (+Z du modèle) vers la rue, la cour ou l'eau.
		var basis := Basis(Vector3.UP, -yaw) * Basis.from_scale(BuildingKit.fit_scale(model, length, width))
		var xform := Transform3D(basis, Vector3(p.x, minf(span.y, span.x + 1.4) - 0.05, p.y))
		if kind in ["church", "windmill", "watermill", "manor"]:
			landmarks.add(model, xform)
			print("BattleDecor: %s at (%.0f, %.0f)" % [kind, p.x, p.y])  # repères des captures
		else:
			houses.add(model, xform)
		building_count += 1


func _build_props(props: Array, batch: BuildingKit.Batch) -> void:
	for k in props.size():
		var prop: Dictionary = props[k]
		var model := BuildingKit.prop_model(str(prop["kind"]), k)
		if model == "":
			continue
		var y := _terrain.height_at(float(prop["x"]), float(prop["z"])) - 0.03
		batch.add(model, BuildingKit.prop_transform(prop, y))
		prop_count += 1


## Murs de pierre sèche du cimetière (tronçons de 10 m du kit ajustés à chaque côté) et porche
## couvert sur le côté le plus proche d'une route.
func _build_churchyards(areas: Array, batch: BuildingKit.Batch) -> void:
	var runs := BuildingKit.models_of("wall_run")
	var gates := BuildingKit.models_of("lychgate")
	if runs.is_empty():
		return
	for area in areas:
		if str(area["kind"]) != "church":
			continue
		var c := Vector2(float(area["x"]), float(area["z"]))
		var yaw := float(area["yaw"])
		var hl := float(area["length"]) * 0.5
		var hw := float(area["width"]) * 0.5
		var corners: Array[Vector2] = [Vector2(-hl, -hw), Vector2(hl, -hw), Vector2(hl, hw), Vector2(-hl, hw)]
		# Porche : milieu du côté le plus proche d'une route (sinon la façade).
		var gate_side := 2
		var best := INF
		for s in 4:
			var mid: Vector2 = c + ((corners[s] + corners[(s + 1) % 4]) * 0.5).rotated(yaw)
			var d := _terrain.road_distance(mid)
			if d < best:
				best = d
				gate_side = s
		for s in 4:
			var a: Vector2 = c + corners[s].rotated(yaw)
			var b: Vector2 = c + corners[(s + 1) % 4].rotated(yaw)
			var seg_len := a.distance_to(b)
			var dir := (b - a).normalized()
			var pieces: Array = [[0.0, seg_len]]
			if s == gate_side and not gates.is_empty():
				var mid := seg_len * 0.5
				pieces = [[0.0, mid - 1.6], [mid + 1.6, seg_len]]
				var gp := a + dir * mid
				var gy := _terrain.height_at(gp.x, gp.y)
				var gbasis := Basis(Vector3.UP, -atan2(dir.y, dir.x))
				batch.add(gates[0], Transform3D(gbasis, Vector3(gp.x, gy - 0.05, gp.y)))
			for piece in pieces:
				var from: float = piece[0]
				var to: float = piece[1]
				var length := to - from
				if length < 1.0:
					continue
				var n := maxi(int(ceil(length / 10.0)), 1)
				var run_len := length / float(n)
				for i in n:
					var q := a + dir * (from + run_len * (float(i) + 0.5))
					var y := _terrain.height_at(q.x, q.y)
					var basis := Basis(Vector3.UP, -atan2(dir.y, dir.x)) * Basis.from_scale(Vector3(run_len / 10.0, 1.0, 1.0))
					batch.add(runs[(i + s) % runs.size()], Transform3D(basis, Vector3(q.x, y - 0.1, q.y)))


# --- Fossé du manoir ------------------------------------------------------------------------


func _build_moats(moats: Array, weather: String) -> void:
	for moat in moats:
		var c := Vector2(float(moat["x"]), float(moat["z"]))
		var yaw := float(moat["yaw"])
		var hl := float(moat["length"]) * 0.5
		var hw := float(moat["width"]) * 0.5
		var ring := float(moat["ring"])
		# Niveau d'eau : un peu sous le sol le plus bas de l'anneau (le sol y est creusé en boue).
		var level := INF
		for k in 16:
			var ang := TAU * float(k) / 16.0
			var q := c + Vector2(cos(ang) * hl, sin(ang) * hw).rotated(yaw)
			level = minf(level, _terrain.height_at(q.x, q.y))
		var st := SurfaceTool.new()
		st.begin(Mesh.PRIMITIVE_TRIANGLES)
		var outer: Array[Vector2] = [Vector2(-hl, -hw), Vector2(hl, -hw), Vector2(hl, hw), Vector2(-hl, hw)]
		var inner: Array[Vector2] = [Vector2(-hl + ring, -hw + ring), Vector2(hl - ring, -hw + ring), Vector2(hl - ring, hw - ring), Vector2(-hl + ring, hw - ring)]
		for s in 4:
			var o0: Vector2 = c + outer[s].rotated(yaw)
			var o1: Vector2 = c + outer[(s + 1) % 4].rotated(yaw)
			var i0: Vector2 = c + inner[s].rotated(yaw)
			var i1: Vector2 = c + inner[(s + 1) % 4].rotated(yaw)
			var steps := maxi(int(o0.distance_to(o1) / 4.0), 1)
			for k in steps:
				var t0 := float(k) / float(steps)
				var t1 := float(k + 1) / float(steps)
				var quad := [o0.lerp(o1, t0), o0.lerp(o1, t1), i0.lerp(i1, t1), i0.lerp(i1, t0)]
				var verts: Array[Vector3] = []
				for q in quad:
					# L'eau suit le sol (fossé creusé) sans descendre sous le niveau bas.
					var h := maxf(level, _terrain.height_at(q.x, q.y) - 0.25)
					verts.append(Vector3(q.x, h, q.y))
				for idx in [0, 1, 2, 0, 2, 3]:
					st.set_normal(Vector3.UP)
					st.set_uv(Vector2(verts[idx].x, verts[idx].z) * 0.1)
					st.add_vertex(verts[idx])
		var mat := ShaderMaterial.new()
		mat.shader = WATER_SHADER
		mat.set_shader_parameter("river_width", ring)
		mat.set_shader_parameter("flow_speed", 0.0)
		mat.set_shader_parameter("shallow_color", Color(0.24, 0.28, 0.16))
		mat.set_shader_parameter("deep_color", Color(0.06, 0.08, 0.05))
		mat.set_shader_parameter("turbidity", 0.9)
		mat.set_shader_parameter("clarity", 0.35)
		mat.set_shader_parameter("macro_noise", _terrain.macro_noise)
		mat.set_shader_parameter("wave_normal", _terrain.water_waves())
		mat.set_shader_parameter("sky_color", _terrain.sky_reflection().darkened(0.35))
		if weather == "rain":
			mat.set_shader_parameter("ripple", 1.0)
		var mi := MeshInstance3D.new()
		mi.name = "Moat"
		mi.mesh = st.commit()
		mi.material_override = mat
		mi.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		mi.visibility_range_end = HOUSE_RANGE
		add_child(mi)


# --- Vignes ------------------------------------------------------------------------------------


## Rangs de vigne tous les 2,2 m le long de la parcelle : modèle du kit (feuillu ou nu d'hiver)
## en tronçons de 10 m de près, simple haie basse au-delà (un MultiMesh par parcelle et par niveau).
func _build_vineyards(areas: Array, leafy: bool) -> void:
	var models := BuildingKit.models_of("vine_row")
	var near_model := ""
	if not models.is_empty():
		near_model = models[0] if leafy or models.size() < 2 else models[1]
	var far_mesh := _vine_far_mesh(leafy)
	var lod := RenderQuality.battle_lod_scale
	for area in areas:
		if str(area["kind"]) != "vineyard":
			continue
		var c := Vector2(float(area["x"]), float(area["z"]))
		var yaw := float(area["yaw"])
		var length := float(area["length"])
		var width := float(area["width"])
		var axis := Vector2(cos(yaw), sin(yaw))
		var across := Vector2(-sin(yaw), cos(yaw))
		var rows := maxi(int((width - 3.0) / 2.2), 1)
		var segs := maxi(int(ceil((length - 4.0) / 10.0)), 1)
		var seg_len := (length - 4.0) / float(segs)
		var xforms: Array[Transform3D] = []
		for r in rows:
			var v := -width * 0.5 + 1.5 + 2.2 * (float(r) + 0.5)
			for s in segs:
				var u := -length * 0.5 + 2.0 + seg_len * (float(s) + 0.5)
				var q := c + axis * u + across * v
				var basis := Basis(Vector3.UP, -yaw) * Basis.from_scale(Vector3(seg_len / 10.0, 1.0, 1.0))
				xforms.append(Transform3D(basis, Vector3(q.x, _terrain.height_at(q.x, q.y) - 0.05, q.y)))
		vine_segments += xforms.size()
		var node := Node3D.new()
		node.name = "Vineyard"
		add_child(node)
		if near_model != "":
			var batch := BuildingKit.Batch.new("", VINE_NEAR * lod)
			for x in xforms:
				batch.add(near_model, x)
			batch.build(node)
		var mm := MultiMesh.new()
		mm.transform_format = MultiMesh.TRANSFORM_3D
		mm.mesh = far_mesh
		mm.instance_count = xforms.size()
		for i in xforms.size():
			mm.set_instance_transform(i, xforms[i])
		var mmi := MultiMeshInstance3D.new()
		mmi.multimesh = mm
		mmi.visibility_range_begin = VINE_NEAR * lod if near_model != "" else 0.0
		mmi.visibility_range_end = VINE_FAR * lod
		mmi.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		node.add_child(mmi)


## Rang de vigne lointain : haie basse de 10 m (feuillage vert ou ceps bruns d'hiver).
static func _vine_far_mesh(leafy: bool) -> ArrayMesh:
	var st := SurfaceTool.new()
	st.begin(Mesh.PRIMITIVE_TRIANGLES)
	var color := Color(0.27, 0.36, 0.13) if leafy else Color(0.33, 0.26, 0.19)
	BattleMeshes.add_box(st, Vector3(0, 0.7, 0), Vector3(10.0, 0.9 if leafy else 0.5, 0.55 if leafy else 0.12), color)
	var mesh := st.commit()
	var mat := StandardMaterial3D.new()
	mat.vertex_color_use_as_albedo = true
	mat.roughness = 0.95
	mesh.surface_set_material(0, mat)
	return mesh


# --- Camps -------------------------------------------------------------------------------------


func _build_camp(camp: Dictionary) -> void:
	var side := str(camp["side"])
	var lod := RenderQuality.battle_lod_scale
	var batch := BuildingKit.Batch.new("snow" if _snowy else "", CAMP_RANGE * lod)
	var horses := BuildingKit.Batch.new("", HORSE_RANGE * lod)
	var horse_models := BuildingKit.models_of("horse")
	# Lot AS1 : chevaux animés (respiration, queue, tête), un MultiMesh par modèle.
	var horse_moves: Variant = {} if AnimalMotion.enabled() else null
	var tents: Array = []
	var fires: Array[Vector3] = []
	var posts := PackedVector3Array()
	var items: Array = camp.get("items", [])
	for k in items.size():
		var item: Dictionary = items[k]
		var kind := str(item["kind"])
		var p := Vector2(float(item["x"]), float(item["z"]))
		var y := _terrain.height_at(p.x, p.y) - 0.03
		match kind:
			"horse_line":
				_horse_line(item, horses, horse_models, posts, horse_moves)
			_:
				var model := BuildingKit.prop_model(kind, k)
				if model == "":
					continue
				var handle := batch.add(model, BuildingKit.prop_transform(item, y))
				if kind == "tent" or kind == "pavilion":
					tents.append(handle)
					tent_count += 1
				elif kind == "campfire":
					fires.append(Vector3(p.x, y, p.y))
	var convoy: Array = camp.get("convoy", [])
	for k in convoy.size():
		var wagon: Dictionary = convoy[k]
		var model := BuildingKit.prop_model("wagon", k + 7)
		if model != "":
			batch.add(model, BuildingKit.prop_transform(wagon, _terrain.height_at(float(wagon["x"]), float(wagon["z"])) - 0.03))
	var root := Node3D.new()
	root.name = "Camp_" + side
	add_child(root)
	batch.build(root)
	ga3_count += batch.ga3_count()
	horses.build(root)
	if horse_moves != null:
		AnimalMotion.build_camp_horses(root, horse_moves, HORSE_RANGE * lod)
	_build_posts(root, posts)
	_build_fires(root, fires)
	_camps[side] = {"batch": batch, "tents": tents, "fires": fires, "looted": false, "root": root}
	var area: Dictionary = camp.get("area", {})
	print("BattleDecor: camp %s at (%.0f, %.0f), %d fires" % [side, float(area.get("x", 0.0)), float(area.get("z", 0.0)), fires.size()])


## Chevaux au piquet des deux côtés d'une corde tendue entre deux poteaux, tête vers la corde.
## `moves` (lot AS1) : non nul, il recueille les poses par modèle pour les chevaux animés.
func _horse_line(item: Dictionary, batch: BuildingKit.Batch, models: Array, posts: PackedVector3Array, moves: Variant) -> void:
	var c := Vector2(float(item["x"]), float(item["z"]))
	var yaw := float(item["yaw"])
	var length := float(item["length"])
	var axis := Vector2(cos(yaw), sin(yaw))
	var front := Vector2(-sin(yaw), cos(yaw))
	var a := c - axis * length * 0.5
	var b := c + axis * length * 0.5
	posts.append(Vector3(a.x, _terrain.height_at(a.x, a.y), a.y))
	posts.append(Vector3(b.x, _terrain.height_at(b.x, b.y), b.y))
	if models.is_empty():
		return
	var count := int(item.get("count", 6))
	for i in count:
		var t := (float(i) + 0.5) / float(count)
		var side := 1.0 if i % 2 == 0 else -1.0
		var p := a.lerp(b, t) + front * side * 1.5
		# Le modèle a le nez vers -X : il regarde la corde (vers -side * front).
		var nose := -front * side
		var ang := atan2(-nose.y, -nose.x) + BuildingKit.hash01(i, int(c.x)) * 0.5 - 0.25
		var xform := Transform3D(Basis(Vector3.UP, -ang), Vector3(p.x, _terrain.height_at(p.x, p.y) - 0.02, p.y))
		var model: String = models[(i + int(c.x)) % models.size()]
		if moves == null:
			batch.add(model, xform)
		else:
			# Lot AS1 : posé par `AnimalMotion.build_camp_horses` (shader de sommets).
			if not moves.has(model):
				moves[model] = []
			(moves[model] as Array).append(xform)
		horse_count += 1


## Poteaux et cordes des lignes de chevaux (un seul maillage par camp).
func _build_posts(root: Node3D, posts: PackedVector3Array) -> void:
	if posts.is_empty():
		return
	var st := SurfaceTool.new()
	st.begin(Mesh.PRIMITIVE_TRIANGLES)
	var wood := Color(0.36, 0.27, 0.18)
	for i in range(0, posts.size() - 1, 2):
		var a := posts[i]
		var b := posts[i + 1]
		BattleMeshes.add_box(st, a + Vector3(0, 0.75, 0), Vector3(0.14, 1.5, 0.14), wood)
		BattleMeshes.add_box(st, b + Vector3(0, 0.75, 0), Vector3(0.14, 1.5, 0.14), wood)
		var mid := (a + b) * 0.5 + Vector3(0, 1.3, 0)
		var d := b - a
		var rope := MeshInstance3D.new()
		var box := BoxMesh.new()
		box.size = Vector3(Vector2(d.x, d.z).length(), 0.03, 0.03)
		rope.mesh = box
		rope.position = mid
		rope.rotation.y = -atan2(d.z, d.x)
		rope.visibility_range_end = HORSE_RANGE
		root.add_child(rope)
	var mi := MeshInstance3D.new()
	mi.mesh = st.commit()
	var mat := StandardMaterial3D.new()
	mat.vertex_color_use_as_albedo = true
	mat.roughness = 0.9
	mi.material_override = mat
	mi.visibility_range_end = HORSE_RANGE
	root.add_child(mi)


## Flammes des feux de camp : quads face caméra au shader animé, un seul MultiMesh par camp.
func _build_fires(root: Node3D, fires: Array[Vector3]) -> void:
	if fires.is_empty():
		return
	var quad := QuadMesh.new()
	quad.size = Vector2(1.1, 1.5)
	quad.center_offset = Vector3(0, 0.75, 0)
	var mat := ShaderMaterial.new()
	mat.shader = FLAME_SHADER
	quad.material = mat
	var mm := MultiMesh.new()
	mm.transform_format = MultiMesh.TRANSFORM_3D
	mm.use_custom_data = true
	mm.mesh = quad
	mm.instance_count = fires.size()
	for i in fires.size():
		mm.set_instance_transform(i, Transform3D(Basis(), fires[i] + Vector3(0, 0.25, 0)))
		mm.set_instance_custom_data(i, Color(BuildingKit.hash01(i, 71), 0, 0, 0))
	var mmi := MultiMeshInstance3D.new()
	mmi.name = "Fires"
	mmi.multimesh = mm
	mmi.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	mmi.visibility_range_end = FIRE_RANGE * RenderQuality.battle_lod_scale
	root.add_child(mmi)


## Camp pillé : tentes et pavillons abattus, fumée noire au-dessus des chariots.
func _loot(side: String) -> void:
	var camp: Dictionary = _camps[side]
	camp["looted"] = true
	var batch: BuildingKit.Batch = camp["batch"]
	for handle in camp["tents"]:
		var xform := batch.get_transform(handle)
		var tilt := Basis(Vector3.RIGHT, 0.35) * Basis.from_scale(Vector3(1.05, 0.28, 1.1))
		batch.set_transform(handle, Transform3D(xform.basis * tilt, xform.origin - Vector3(0, 0.05, 0)))
	var fires: Array = camp["fires"]
	var root: Node3D = camp["root"]
	for i in mini(fires.size(), 3):
		# EP8 : colonne de fumée noire partagée avec les incendies ; sinon particules locales.
		var id := -1
		if _smoke_scene != null:
			id = int(_smoke_scene.call("add_smoke_source", fires[i] + Vector3(0, 1.5, 0), 1.2, "column"))
		if id < 0:
			root.add_child(_smoke(fires[i]))
	print("BattleDecor: camp %s looted" % side)


func _smoke(at: Vector3) -> GPUParticles3D:
	var p := GPUParticles3D.new()
	p.name = "LootSmoke"
	p.amount = 24
	p.lifetime = 9.0
	p.position = at + Vector3(0, 1.0, 0)
	var pm := ParticleProcessMaterial.new()
	pm.direction = Vector3(0.2, 1, 0)
	pm.spread = 12.0
	pm.initial_velocity_min = 1.5
	pm.initial_velocity_max = 2.5
	pm.gravity = Vector3(0.3, 0.4, 0)
	pm.scale_min = 2.0
	pm.scale_max = 4.0
	pm.color = Color(0.16, 0.15, 0.14, 0.55)
	p.process_material = pm
	var quad := QuadMesh.new()
	quad.size = Vector2(3, 3)
	var mat := StandardMaterial3D.new()
	mat.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	mat.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	mat.billboard_mode = BaseMaterial3D.BILLBOARD_PARTICLES
	mat.vertex_color_use_as_albedo = true
	mat.albedo_color = Color(1, 1, 1, 0.6)
	quad.material = mat
	p.draw_pass_1 = quad
	p.visibility_aabb = AABB(Vector3(-20, -2, -20), Vector3(40, 80, 40))
	return p
