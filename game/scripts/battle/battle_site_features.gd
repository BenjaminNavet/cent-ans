class_name BattleSiteFeatures
extends Node3D

## Lot B5 : ce que le site de campagne pose sur le champ en plus du relief et des bois, d'après
## `BattleSim.get_terrain()` : clôtures de plessis, palissades de camp, mares du marais (eau stagnante) et roselières, mer le long d'un
## flanc côtier. Les haies sont semées avec les arbres (`BattleTerrain._build_trees`), les fossés,
## la cour et la plage dans la splatmap. Rendu seulement : positions et emprises viennent de la
## simulation. Les maisons des hameaux viennent du décor EP6 (`BattleDecor`), plus du site.

const WATER_SHADER := preload("res://shaders/battle_water.gdshader")
const SEA_SHADER := preload("res://shaders/battle_sea.gdshader")
## Niveau de la mer (m) ; le sol plonge sous ce niveau au-delà de la ligne de rivage.
const SEA_LEVEL := 0.0

var _terrain: BattleTerrain
var _snowy := false
var _mats: Dictionary = {}
var reed_count: int = 0


func build(terrain: BattleTerrain, data: Dictionary, weather: String) -> void:
	_terrain = terrain
	_snowy = terrain.ground_key == "snowy" or weather == "snow"
	_build_fences(data.get("obstacles", []))
	_build_palisades(data.get("obstacles", []))
	_build_pools(data.get("pools", []), weather)
	_build_reeds(data)
	if data.has("coast"):
		_build_sea(data["coast"], weather)


# --- Matières -----------------------------------------------------------------------------


func _mat(key: String) -> StandardMaterial3D:
	if _mats.is_empty():
		_mats = {
			"beam": BattleSiege._textured("wood", Color(0.33, 0.24, 0.17)),
			"wattle": BattleSiege._textured("wood", Color(0.58, 0.5, 0.4)),
		}
	return _mats[key]


# --- Clôtures de plessis ------------------------------------------------------------------


## Clôtures (`kind == "fence"`) : poteaux tous les 2 m et panneau de plessis qui suit le sol ;
## un seul maillage fusionné pour toutes les clôtures.
func _build_fences(obstacles: Array) -> void:
	var st := SurfaceTool.new()
	st.begin(Mesh.PRIMITIVE_TRIANGLES)
	var count := 0
	for o in obstacles:
		if str(o["kind"]) != "fence":
			continue
		var a: Vector2 = o["a"]
		var b: Vector2 = o["b"]
		var length := a.distance_to(b)
		var steps := maxi(int(length / 2.0), 1)
		var dir := (b - a) / length
		var yaw := -atan2(dir.y, dir.x)
		for k in steps + 1:
			var p := a.lerp(b, float(k) / float(steps))
			var y := _terrain.height_at(p.x, p.y)
			_add_box(st, Vector3(p.x, y + 0.55, p.y), Vector3(0.12, 1.3, 0.12), yaw)
			if k < steps:
				var q := a.lerp(b, (float(k) + 0.5) / float(steps))
				var yq := _terrain.height_at(q.x, q.y)
				_add_box(st, Vector3(q.x, yq + 0.5, q.y), Vector3(length / float(steps), 0.75, 0.07), yaw)
		count += 1
	if count == 0:
		return
	st.generate_normals()
	var mi := MeshInstance3D.new()
	mi.name = "Fences"
	mi.mesh = st.commit()
	mi.material_override = _mat("wattle")
	mi.visibility_range_end = 900.0
	add_child(mi)


## CV3-2 : palissade basse d'un camp retranché (`kind == "palisade"`) : pieux de 1,7 m serrés
## tous les 0,45 m, un peu penchés vers l'ennemi, liés par deux traverses ; un seul maillage.
func _build_palisades(obstacles: Array) -> void:
	if _ga3_palisades(obstacles):
		return
	var st := SurfaceTool.new()
	st.begin(Mesh.PRIMITIVE_TRIANGLES)
	var count := 0
	for o in obstacles:
		if str(o["kind"]) != "palisade":
			continue
		var a: Vector2 = o["a"]
		var b: Vector2 = o["b"]
		var length := a.distance_to(b)
		if length < 0.1:
			continue
		var dir := (b - a) / length
		var yaw := -atan2(dir.y, dir.x)
		var steps := maxi(int(length / 0.45), 1)
		for k in steps + 1:
			var p := a.lerp(b, float(k) / float(steps))
			var y := _terrain.height_at(p.x, p.y)
			_add_box(st, Vector3(p.x, y + 0.8, p.y), Vector3(0.16, 1.7, 0.16), yaw)
		var m := a.lerp(b, 0.5)
		var ym := _terrain.height_at(m.x, m.y)
		for h in [0.5, 1.2]:
			_add_box(st, Vector3(m.x, ym + h, m.y), Vector3(length, 0.1, 0.1), yaw)
		count += 1
	if count == 0:
		return
	st.generate_normals()
	var mi := MeshInstance3D.new()
	mi.name = "Palisades"
	mi.mesh = st.commit()
	mi.material_override = _mat("beam")
	mi.visibility_range_end = 1200.0
	add_child(mi)


## Lot GA3-L1 : palissade en segments générés (`Ga3Kit`, variante `palisade`) : chaque tronçon
## de la simulation reçoit un nombre entier de segments étirés en longueur (±25 %), posés sur le
## sol. Faux (procédural gardé) sans variante, ou sous la neige.
func _ga3_palisades(obstacles: Array) -> bool:
	if not Ga3Kit.active or _snowy:
		return false
	var variants := Ga3Kit.variants_of("palisade")
	if variants.is_empty():
		return false
	var model: String = variants[0]
	var segment := float((Ga3Kit.manifest()[model] as Dictionary)["length"])
	var batch := Ga3Kit.Batch.new(1200.0 * RenderQuality.battle_lod_scale)
	for o in obstacles:
		if str(o["kind"]) != "palisade":
			continue
		var a: Vector2 = o["a"]
		var b: Vector2 = o["b"]
		var length := a.distance_to(b)
		if length < 0.1:
			continue
		var dir := (b - a) / length
		var n := maxi(int(round(length / segment)), 1)
		var stretch := clampf(length / (n * segment), 0.75, 1.25)
		for k in n:
			var p := a.lerp(b, (k + 0.5) / float(n))
			var basis := Basis(Vector3.UP, -atan2(dir.y, dir.x)) * Basis.from_scale(Vector3(stretch, 1.0, 1.0))
			batch.add(model, Transform3D(basis, Vector3(p.x, _terrain.height_at(p.x, p.y) - 0.08, p.y)))
	if batch.count() == 0:
		return false
	var root := Node3D.new()
	root.name = "Palisades"
	add_child(root)
	batch.build(root)
	return true


## Boîte orientée (lacet `yaw` autour de y) ajoutée au SurfaceTool (faces à plat, normales générées).
static func _add_box(st: SurfaceTool, center: Vector3, size: Vector3, yaw: float) -> void:
	var basis := Basis(Vector3.UP, yaw)
	var h := size * 0.5
	var c := []
	for i in 8:
		var v := Vector3(h.x if i & 1 else -h.x, h.y if i & 2 else -h.y, h.z if i & 4 else -h.z)
		c.append(center + basis * v)
	var faces := [[0, 1, 3, 2], [4, 6, 7, 5], [0, 4, 5, 1], [2, 3, 7, 6], [0, 2, 6, 4], [1, 5, 7, 3]]
	for f in faces:
		st.add_vertex(c[f[0]])
		st.add_vertex(c[f[1]])
		st.add_vertex(c[f[2]])
		st.add_vertex(c[f[0]])
		st.add_vertex(c[f[2]])
		st.add_vertex(c[f[3]])


# --- Mares et roseaux ---------------------------------------------------------------------


## Mares : disques d'eau stagnante (shader de la rivière sans courant, eau brune et verte) au
## niveau moyen du sol, que le sol rendu creuse de `BattleTerrain.POOL_CARVE`.
func _build_pools(pools: Array, weather: String) -> void:
	for pool in pools:
		var c := Vector2(float(pool["x"]), float(pool["z"]))
		var r := float(pool["radius"])
		var level := _terrain.pool_level(c, r)
		var st := SurfaceTool.new()
		st.begin(Mesh.PRIMITIVE_TRIANGLES)
		var rings := 4
		var segs := 28
		var prev := []
		for ri in rings + 1:
			var rr := r * (1.0 + 0.12) * float(ri) / float(rings)
			var ring := []
			for s in segs:
				var ang := TAU * float(s) / float(segs)
				var wobble := 1.0 + 0.1 * sin(ang * 3.0 + c.x * 0.1) + 0.06 * sin(ang * 5.0 + c.y * 0.07)
				ring.append(Vector3(c.x + cos(ang) * rr * wobble, level, c.y + sin(ang) * rr * wobble))
			if ri > 0:
				var u0 := 0.5 - 0.5 * float(ri - 1) / float(rings)
				var u1 := 0.5 - 0.5 * float(ri) / float(rings)
				for s in segs:
					var s1 := (s + 1) % segs
					var v0 := float(s) / float(segs) * TAU * r
					var v1 := float(s + 1) / float(segs) * TAU * r
					var quad := [[prev[s], u0, v0], [ring[s], u1, v0], [ring[s1], u1, v1], [prev[s1], u0, v1]]
					for idx in [0, 1, 2, 0, 2, 3]:
						st.set_normal(Vector3.UP)
						st.set_uv(Vector2(quad[idx][1], quad[idx][2]))
						st.add_vertex(quad[idx][0])
			prev = ring
		var mat := ShaderMaterial.new()
		mat.shader = WATER_SHADER
		mat.set_shader_parameter("river_width", r * 2.0)
		mat.set_shader_parameter("flow_speed", 0.0)
		mat.set_shader_parameter("shallow_color", Color(0.28, 0.3, 0.18))
		mat.set_shader_parameter("deep_color", Color(0.07, 0.09, 0.05))
		mat.set_shader_parameter("turbidity", 0.85)
		mat.set_shader_parameter("clarity", 0.4)
		mat.set_shader_parameter("macro_noise", _terrain.macro_noise)
		mat.set_shader_parameter("wave_normal", _terrain.water_waves())
		mat.set_shader_parameter("sky_color", _terrain.sky_reflection().darkened(0.35))
		if weather == "rain":
			mat.set_shader_parameter("ripple", 1.0)
		var mi := MeshInstance3D.new()
		mi.name = "Pool"
		mi.mesh = st.commit()
		mi.material_override = mat
		mi.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		add_child(mi)


## Roselières : couronnes de roseaux autour des mares, touffes dans les boues d'un marais et le
## long de la rivière en terrain de marais. Un MultiMesh par tuile de 160 m (écarté hors champ).
func _build_reeds(data: Dictionary) -> void:
	var rng := RandomNumberGenerator.new()
	rng.seed = 5150
	var spots: Array[Vector2] = []
	for pool in data.get("pools", []):
		var c := Vector2(float(pool["x"]), float(pool["z"]))
		var r := float(pool["radius"])
		for _i in int(TAU * r / 0.45):
			var ang := rng.randf() * TAU
			spots.append(c + Vector2(cos(ang), sin(ang)) * r * rng.randf_range(0.8, 1.35))
	# EP3 : roselières le long des berges marécageuses de la rivière, touffes au bord des ruisseaux.
	for bank in data.get("river", {}).get("banks", []):
		if str(bank["kind"]) != "marsh":
			continue
		var side := 1.0 if bool(bank["north"]) else -1.0
		var x := float(bank["x0"])
		while x < float(bank["x1"]):
			var zc := _terrain.river_center_z(x)
			var w := _terrain.river_width_at(x)
			for _k in 3:
				spots.append(Vector2(x + rng.randf_range(-1.0, 1.0), zc + side * (w * 0.5 + rng.randf_range(-1.5, 7.0))))
			x += 1.2
	for stream in data.get("streams", []):
		var pts: PackedVector2Array = stream["points"]
		var w := float(stream["width"])
		for i in range(pts.size() - 1):
			if rng.randf() < 0.55:
				continue
			var dir := (pts[i + 1] - pts[i]).normalized()
			var n := Vector2(-dir.y, dir.x) * (1.0 if rng.randf() < 0.5 else -1.0)
			for _k in 6:
				spots.append(pts[i].lerp(pts[i + 1], rng.randf()) + n * (w * 0.5 + rng.randf_range(-0.3, 1.2)))
	var marsh := str(data.get("terrain", "")) == "marsh"
	if marsh:
		for zone in data.get("mud", []):
			var c := Vector2(float(zone["x"]), float(zone["z"]))
			var r := float(zone["radius"])
			for _i in int(PI * r * r / 30.0):
				var ang := rng.randf() * TAU
				var d := sqrt(rng.randf()) * r
				spots.append(c + Vector2(cos(ang), sin(ang)) * d)
	if spots.is_empty():
		return
	var tiles := {}
	for p in spots:
		var key := Vector2i(floori(p.x / 160.0), floori(p.y / 160.0))
		if not tiles.has(key):
			tiles[key] = []
		(tiles[key] as Array).append(p)
	var mesh := _reed_mesh()
	var mat := StandardMaterial3D.new()
	mat.vertex_color_use_as_albedo = true
	mat.vertex_color_is_srgb = true
	mat.cull_mode = BaseMaterial3D.CULL_DISABLED
	mat.roughness = 0.9
	if _snowy:
		mat.albedo_color = Color(1.1, 1.0, 0.85)
	for key in tiles:
		var members: Array = tiles[key]
		var mm := MultiMesh.new()
		mm.transform_format = MultiMesh.TRANSFORM_3D
		mm.mesh = mesh
		mm.instance_count = members.size()
		for k in members.size():
			var p: Vector2 = members[k]
			var s := rng.randf_range(0.7, 1.3)
			var basis := Basis(Vector3.UP, rng.randf() * TAU).scaled(Vector3(s, s * rng.randf_range(0.8, 1.2), s))
			mm.set_instance_transform(k, Transform3D(basis, Vector3(p.x, _terrain.height_at(p.x, p.y) - 0.1, p.y)))
		var mi := MultiMeshInstance3D.new()
		mi.name = "Reeds_%d_%d" % [key.x, key.y]
		mi.multimesh = mm
		mi.material_override = mat
		mi.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		mi.visibility_range_end = 600.0
		add_child(mi)
		reed_count += members.size()


## Touffe de roseaux : 9 tiges effilées (1,3-2,2 m), quelques massettes brunes ; couleurs par
## sommet (vert-jaune à la base, paille en haut).
static func _reed_mesh() -> ArrayMesh:
	var st := SurfaceTool.new()
	st.begin(Mesh.PRIMITIVE_TRIANGLES)
	var rng := RandomNumberGenerator.new()
	rng.seed = 77
	for k in 9:
		var ang := rng.randf() * TAU
		var base := Vector3(cos(ang), 0, sin(ang)) * rng.randf_range(0.0, 0.35)
		var h := rng.randf_range(1.3, 2.2)
		var lean := Vector3(rng.randf_range(-0.3, 0.3), 0, rng.randf_range(-0.3, 0.3))
		var side := Vector3(cos(ang + 1.3), 0, sin(ang + 1.3)) * 0.06
		var top := base + Vector3(0, h, 0) + lean
		var low := Color(0.2, 0.26, 0.1)
		var tip := Color(0.5, 0.47, 0.26)
		st.set_color(low)
		st.add_vertex(base - side)
		st.set_color(low)
		st.add_vertex(base + side)
		st.set_color(tip)
		st.add_vertex(top)
		if k % 3 == 0:
			# Massette : petit fuseau brun sous la pointe.
			var m := base + (top - base) * 0.8
			var brown := Color(0.3, 0.2, 0.12)
			for q in [[m - side * 2.0, m + side * 2.0, m + (top - base) * 0.12]]:
				for v in q:
					st.set_color(brown)
					st.add_vertex(v)
	st.generate_normals()
	return st.commit()


# --- Mer ----------------------------------------------------------------------------------


## Mer le long du flanc côtier : un plan d'eau (`battle_sea.gdshader`) du rivage jusqu'à l'horizon,
## commencé un peu dans les terres : la ligne d'eau est là où le sol plonge sous `SEA_LEVEL`.
func _build_sea(coast: Dictionary, weather: String) -> void:
	var west := str(coast.get("flank", "west")) == "west"
	var shore := float(coast.get("shore_x", 0.0))
	var inland := shore + (40.0 if west else -40.0)
	var x_far := shore - 9000.0 if west else shore + 9000.0
	var x0 := minf(inland, x_far)
	var x1 := maxf(inland, x_far)
	var plane := PlaneMesh.new()
	plane.size = Vector2(x1 - x0, 17000.0)
	plane.subdivide_width = 8
	plane.subdivide_depth = 8
	var mat := ShaderMaterial.new()
	mat.shader = SEA_SHADER
	mat.set_shader_parameter("shore_x", shore)
	mat.set_shader_parameter("flank", -1.0 if west else 1.0)
	mat.set_shader_parameter("macro_noise", _terrain.macro_noise)
	mat.set_shader_parameter("wave_normal", _terrain.water_waves())
	mat.set_shader_parameter("sky_color", _terrain.sky_reflection())
	if weather == "rain":
		mat.set_shader_parameter("ripple", 1.0)
	var mi := MeshInstance3D.new()
	mi.name = "Sea"
	mi.mesh = plane
	mi.position = Vector3((x0 + x1) * 0.5, SEA_LEVEL, _terrain.FIELD_D * 0.5)
	mi.material_override = mat
	mi.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	add_child(mi)
