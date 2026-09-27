class_name BattleVillage
extends Node3D

## Lot B5 : ce que le site de campagne pose sur le champ en plus du relief et des bois, d'après
## `BattleSim.get_terrain()` : maisons du hameau ou de la ferme (chaume, colombage, grange, église),
## clôtures de plessis des courtils, mares du marais (eau stagnante) et roselières, mer le long d'un
## flanc côtier. Les haies sont semées avec les arbres (`BattleTerrain._build_trees`), les fossés,
## la cour et la plage dans la splatmap. Rendu seulement : positions et emprises viennent de la
## simulation. Maisons et clôtures regroupées en MultiMesh (`BattleSiegeBatcher`) : quelques appels
## de dessin quel que soit le nombre de maisons.

const WATER_SHADER := preload("res://shaders/battle_water.gdshader")
const SEA_SHADER := preload("res://shaders/battle_sea.gdshader")
## Niveau de la mer (m) ; le sol plonge sous ce niveau au-delà de la ligne de rivage.
const SEA_LEVEL := 0.0

var _terrain: BattleTerrain
var _snowy := false
var _mats: Dictionary = {}
var house_count: int = 0
var reed_count: int = 0


func build(terrain: BattleTerrain, data: Dictionary, weather: String) -> void:
	_terrain = terrain
	_snowy = terrain.ground_key == "snowy" or weather == "snow"
	var village: Dictionary = data.get("village", {})
	if not village.is_empty():
		_build_houses(village)
	_build_fences(data.get("obstacles", []))
	_build_palisades(data.get("obstacles", []))
	_build_pools(data.get("pools", []), weather)
	_build_reeds(data)
	if data.has("coast"):
		_build_sea(data["coast"], weather)


# --- Matières -----------------------------------------------------------------------------


func _mat(key: String) -> StandardMaterial3D:
	if _mats.is_empty():
		var snow_roof := Color.WHITE
		_mats = {
			"stone": BattleSiege._textured("stone", Color(0.86, 0.82, 0.74)),
			"church": BattleSiege._textured("stone", Color(0.92, 0.88, 0.8)),
			"cob0": BattleSiege._textured("plaster", Color(0.9, 0.82, 0.66)),
			"cob1": BattleSiege._textured("plaster", Color(0.84, 0.76, 0.62)),
			"lime": BattleSiege._textured("plaster", Color(0.95, 0.93, 0.87)),
			"beam": BattleSiege._textured("wood", Color(0.33, 0.24, 0.17)),
			"planks": BattleSiege._textured("wood", Color(0.52, 0.42, 0.32)),
			"door": BattleSiege._textured("wood", Color(0.42, 0.31, 0.22)),
			"wattle": BattleSiege._textured("wood", Color(0.58, 0.5, 0.4)),
			"thatch": BattleSiege._textured("thatch", snow_roof if _snowy else Color(1.08, 0.93, 0.68), 1.0),
			"thatch_old": BattleSiege._textured("thatch", snow_roof if _snowy else Color(0.86, 0.76, 0.6), 1.0),
			"tiles": BattleSiege._textured("tiles", snow_roof if _snowy else Color(0.62, 0.45, 0.38), 0.85),
			"slate": BattleSiege._textured("slate", snow_roof if _snowy else Color(0.55, 0.56, 0.6), 0.75),
		}
		if _snowy:
			# Toits sous la neige : manteau blanc uni, légèrement bleuté.
			var snow := StandardMaterial3D.new()
			snow.albedo_color = Color(0.9, 0.92, 0.97)
			snow.roughness = 0.85
			for roof in ["thatch", "thatch_old", "tiles", "slate"]:
				_mats[roof] = snow
	return _mats[key]


# --- Maisons ------------------------------------------------------------------------------


func _build_houses(village: Dictionary) -> void:
	var root := Node3D.new()
	root.name = "Houses"
	add_child(root)
	if BuildingKit.available():
		# BR1 : bâtiments réalistes du kit Blender, un MultiMesh par modèle.
		var batch := BuildingKit.Batch.new("snow" if _snowy else "", 1600.0)
		var props: Array = village.get("props", [])
		var houses: Array = village.get("houses", [])
		for i in houses.size():
			if _kit_house(batch, houses[i], _prop_side(houses[i], i, props)):
				house_count += 1
		# BR3 : mobilier du cœur (solide pour les figurines), taille réelle, dos au mur.
		for k in props.size():
			var prop: Dictionary = props[k]
			var model := BuildingKit.prop_model(str(prop["kind"]), k)
			if model != "":
				var y := _terrain.height_at(float(prop["x"]), float(prop["z"])) - 0.03
				batch.add(model, BuildingKit.prop_transform(prop, y))
		batch.build(root)
		return
	for house in village.get("houses", []):
		_house(root, house)
		house_count += 1
	BattleSiegeBatcher.batch_and_replace(root)
	for child in root.get_children():
		if child is GeometryInstance3D:
			(child as GeometryInstance3D).visibility_range_end = 1600.0


## Type de modèle du kit pour une maison de la simulation (`kind` et emprise).
static func kit_kind(kind: String, length: float, rng: RandomNumberGenerator) -> String:
	match kind:
		"church":
			return "church"
		"barn":
			return "barn"
		"timbered":
			return "stonehouse" if rng.randf() < 0.15 else "timber"
		_:
			return "longere" if length > 11.0 else "cottage"


## BR3 : côté de la façade d'une maison qui a du mobilier du cœur devant elle (+1 : face avant
## (-sin, cos) du lacet, -1 : l'autre) ; 0 sans mobilier (côté tiré au hasard).
static func _prop_side(house: Dictionary, index: int, props: Array) -> int:
	var yaw := float(house["yaw"])
	var front := Vector2(-sin(yaw), cos(yaw))
	for prop in props:
		if int(prop["house"]) == index:
			var d := Vector2(float(prop["x"]) - float(house["x"]), float(prop["z"]) - float(house["z"]))
			return 1 if d.dot(front) >= 0.0 else -1
	return 0


## Pose une maison du kit sur son emprise (lacet de la simulation, de +x vers +z) : origine au
## centre, calée sur le bas de la pente sans descendre de plus de 1,4 m sous le haut (les
## fondations du modèle comblent le reste). `side` : façade tournée vers le mobilier (BR3).
## `false` si aucun modèle ne convient.
func _kit_house(batch: BuildingKit.Batch, house: Dictionary, side: int = 0) -> bool:
	var p := Vector2(float(house["x"]), float(house["z"]))
	var length := float(house["length"])
	var width := float(house["width"])
	var yaw := float(house["yaw"])
	var rng := RandomNumberGenerator.new()
	rng.seed = int(p.x * 31.0 + p.y * 17.0)
	var model := BuildingKit.pick(kit_kind(str(house["kind"]), length, rng), length, width, rng)
	if model == "":
		return false
	var low := INF
	var high := -INF
	for corner in [Vector2(-length, -width), Vector2(length, -width), Vector2(length, width), Vector2(-length, width)]:
		var q: Vector2 = p + (corner * 0.5).rotated(yaw)
		var h := _terrain.height_at(q.x, q.y)
		low = minf(low, h)
		high = maxf(high, h)
	# Façade (+Z du modèle) tournée d'un côté ou de l'autre du faîtage (vers le mobilier s'il y en a).
	var flip := PI if rng.randf() < 0.5 else 0.0
	if side != 0:
		flip = 0.0 if side > 0 else PI
	var basis := Basis(Vector3.UP, -yaw + flip) * Basis.from_scale(BuildingKit.fit_scale(model, length, width))
	batch.add(model, Transform3D(basis, Vector3(p.x, minf(high, low + 1.4) - 0.05, p.y)))
	return true


func _box(parent: Node3D, size: Vector3, pos: Vector3, mat: StandardMaterial3D, rot: Vector3 = Vector3.ZERO) -> MeshInstance3D:
	var mi := MeshInstance3D.new()
	var box := BoxMesh.new()
	box.size = size
	mi.mesh = box
	mi.material_override = mat
	mi.position = pos
	mi.rotation = rot
	parent.add_child(mi)
	return mi


## Toit à deux pans : faîtage le long de x local, longueur `length`.
func _roof(parent: Node3D, length: float, width: float, pitch: float, base_y: float, mat: StandardMaterial3D) -> void:
	var mi := MeshInstance3D.new()
	var prism := PrismMesh.new()
	prism.size = Vector3(width + 1.1, pitch, length + 0.8)
	mi.mesh = prism
	mi.material_override = mat
	mi.position = Vector3(0, base_y + pitch * 0.5, 0)
	mi.rotation.y = PI * 0.5
	parent.add_child(mi)


## Une maison : emprise `length` × `width` (faîtage le long de `length`), lacet de la simulation
## (de +x vers +z), posée sur le point le plus bas de son emprise, soubassement de pierre.
func _house(parent: Node3D, house: Dictionary) -> void:
	var p := Vector2(float(house["x"]), float(house["z"]))
	var length := float(house["length"])
	var width := float(house["width"])
	var yaw := float(house["yaw"])
	var kind := str(house["kind"])
	var node := Node3D.new()
	var low := INF
	var high := -INF
	for corner in [Vector2(-length, -width), Vector2(length, -width), Vector2(length, width), Vector2(-length, width)]:
		var q: Vector2 = p + (corner * 0.5).rotated(yaw)
		var h := _terrain.height_at(q.x, q.y)
		low = minf(low, h)
		high = maxf(high, h)
	node.position = Vector3(p.x, low, p.y)
	node.rotation.y = -yaw
	parent.add_child(node)
	var rng := RandomNumberGenerator.new()
	rng.seed = int(p.x * 31.0 + p.y * 17.0)
	var plinth := high - low + 0.35
	_box(node, Vector3(length + 0.2, plinth + 0.6, width + 0.2), Vector3(0, (plinth - 0.6) * 0.5, 0), _mat("stone"))
	match kind:
		"church":
			_church(node, length, width, plinth, rng)
		"barn":
			var h := rng.randf_range(3.8, 4.6)
			_box(node, Vector3(length, h, width), Vector3(0, plinth + h * 0.5, 0), _mat("planks"))
			_box(node, Vector3(3.2, 3.2, 0.2), Vector3(0, plinth + 1.6, width * 0.5 + 0.05), _mat("door"))
			_roof(node, length, width, width * 0.7, plinth + h, _mat("thatch_old"))
		"timbered":
			var h := rng.randf_range(3.6, 4.6)
			_box(node, Vector3(length, h, width), Vector3(0, plinth + h * 0.5, 0), _mat("lime"))
			_timbers(node, length, width, h, plinth)
			_box(node, Vector3(1.1, 2.0, 0.16), Vector3(rng.randf_range(-length * 0.25, length * 0.25), plinth + 1.0, width * 0.5 + 0.08), _mat("door"))
			var roof := "tiles" if rng.randf() < 0.4 else "thatch"
			_roof(node, length, width, width * 0.8, plinth + h, _mat(roof))
			if rng.randf() < 0.5:
				_box(node, Vector3(0.8, width * 0.8 + 1.3, 0.8), Vector3(length * 0.3, plinth + h + width * 0.4 + 0.3, 0), _mat("stone"))
		_:
			var h := rng.randf_range(2.4, 3.0)
			_box(node, Vector3(length, h, width), Vector3(0, plinth + h * 0.5, 0), _mat("cob%d" % rng.randi_range(0, 1)))
			_box(node, Vector3(1.0, 1.8, 0.16), Vector3(rng.randf_range(-length * 0.2, length * 0.2), plinth + 0.9, width * 0.5 + 0.08), _mat("door"))
			# Chaume épais et pentu, qui descend bas sur les murs.
			_roof(node, length, width + 0.6, width * 0.95, plinth + h - 0.35, _mat("thatch"))


## Colombage : poteaux, sablière, décharges en croix de Saint-André sur les longs pans, poteaux
## d'angle sur les pignons.
func _timbers(node: Node3D, length: float, width: float, h: float, base: float) -> void:
	var beam := _mat("beam")
	var posts := maxi(int(length / 1.8), 3)
	for side in [-1.0, 1.0]:
		var z: float = side * (width * 0.5 + 0.05)
		for k in posts + 1:
			var x := -length * 0.5 + length * float(k) / float(posts)
			_box(node, Vector3(0.2, h, 0.1), Vector3(x, base + h * 0.5, z), beam)
		_box(node, Vector3(length, 0.2, 0.1), Vector3(0, base + h * 0.52, z), beam)
		_box(node, Vector3(length, 0.22, 0.1), Vector3(0, base + h - 0.11, z), beam)
		var brace_len := sqrt(pow(length / float(posts), 2.0) + pow(h * 0.5, 2.0))
		var angle := atan2(h * 0.5, length / float(posts))
		for k in posts:
			if k % 2 == 1:
				continue
			var cx := -length * 0.5 + length * (float(k) + 0.5) / float(posts)
			_box(node, Vector3(brace_len, 0.16, 0.08), Vector3(cx, base + h * 0.25, z), beam, Vector3(0, 0, angle if k % 4 == 0 else -angle))
	for side in [-1.0, 1.0]:
		var x: float = side * (length * 0.5 + 0.05)
		for k in 3:
			_box(node, Vector3(0.1, h, 0.2), Vector3(x, base + h * 0.5, -width * 0.5 + width * float(k) * 0.5), beam)


## Petite église de pierre : nef, chœur plus bas, clocher-mur à l'ouest.
func _church(node: Node3D, length: float, width: float, base: float, rng: RandomNumberGenerator) -> void:
	var h := rng.randf_range(6.0, 7.0)
	var nave := length * 0.7
	_box(node, Vector3(nave, h, width), Vector3(-length * 0.15, base + h * 0.5, 0), _mat("church"))
	var roof := "slate" if rng.randf() < 0.5 else "tiles"
	var sub := Node3D.new()
	sub.position = Vector3(-length * 0.15, 0, 0)
	node.add_child(sub)
	_roof(sub, nave, width, width * 0.75, base + h, _mat(roof))
	var choir := length * 0.3
	var ch := h * 0.8
	var cw := width * 0.75
	_box(node, Vector3(choir, ch, cw), Vector3(length * 0.5 - choir * 0.5, base + ch * 0.5, 0), _mat("church"))
	var sub2 := Node3D.new()
	sub2.position = Vector3(length * 0.5 - choir * 0.5, 0, 0)
	node.add_child(sub2)
	_roof(sub2, choir, cw, cw * 0.75, base + ch, _mat(roof))
	# Clocher-mur (pignon ouest surélevé, percé de deux baies).
	var gx := -length * 0.5 - 0.2
	var gh := h + width * 0.75 + 3.5
	_box(node, Vector3(1.2, gh, width * 0.7), Vector3(gx, base + gh * 0.5, 0), _mat("church"))
	for dz in [-0.9, 0.9]:
		_box(node, Vector3(1.3, 1.4, 0.8), Vector3(gx, base + gh - 1.6, dz), _mat("beam"))


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
