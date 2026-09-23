class_name BattleTerrain
extends Node3D

## Champ de bataille maillé depuis `BattleSim.get_terrain()` (lot V4, rendu semi-réaliste) :
## - sol texturé (shader `battle_ground`, 9 couches PBR Poly Haven) piloté par une splatmap cuite
##   ici (sous-bois, chemins, boue, galets du lit et des gués, berges humides) ;
## - anneaux de terrain autour du champ (proche à 20 m, lointain à 200 m) : collines qui montent
##   vers l'horizon, vallée de la rivière prolongée, pour que l'horizon ne soit jamais vide ;
## - rivière (shader `battle_water` : profondeur, réfraction, reflets, écoulement) ;
## - arbres (tronc texturé + houppier en cartes alpha), buissons, rochers, bois lointains ;
## - herbe animée autour du regard (`BattleVegetation`).
## Rendu seulement : relief, zones, rivière et gués viennent de la simulation ; chemins, parcelles
## et bois hors du champ sont décoratifs.

const FIELD_W := 1200.0
const FIELD_D := 800.0
## Domaine de la splatmap et de la carte de hauteurs de l'herbe (x0, z0, largeur, profondeur).
const SPLAT_RECT := Rect2(-400, -400, 2000, 1600)
const SPLAT_TEXEL := 4.0
const HEIGHT_TEXEL := 10.0
const NEAR_RECT := Rect2(-900, -900, 3000, 2600)
const NEAR_STEP := 20.0
const FAR_RECT := Rect2(-7000, -7000, 15200, 14800)
const FAR_STEP := 200.0
const RIVER_CARVE := 1.6
const RIVER_SPAN := 27.0  # demi-largeur creusée (width * 1.5), comme la simulation

const GROUND_SHADER := preload("res://shaders/battle_ground.gdshader")
const WATER_SHADER := preload("res://shaders/battle_water.gdshader")
const ALBEDO_ARRAY := preload("res://assets/textures/battle/ground_albedo_array.jpg")
const NORMAL_ARRAY := preload("res://assets/textures/battle/ground_normal_array.jpg")

var terrain: Dictionary = {}
var weather_key: String = "clear"
var tree_count: int = 0
var ground_material: ShaderMaterial
var macro_noise: Texture2D
var splat_a: ImageTexture
var splat_b: ImageTexture
var height_texture: ImageTexture
var vegetation: BattleVegetation
var roads: Array[PackedVector2Array] = []
var _heights: PackedFloat32Array
var _nx: int = 0
var _nz: int = 0
var _resolution: float = 10.0
var _mean_height: float = 0.0
var _river_points: PackedVector2Array = PackedVector2Array()  # prolongée hors du champ
var _hills := FastNoiseLite.new()
var _woods := FastNoiseLite.new()


func build(p_terrain: Dictionary, weather: String) -> void:
	terrain = p_terrain
	weather_key = weather
	_heights = terrain.get("heights", PackedFloat32Array())
	_nx = int(terrain.get("nx", 0))
	_nz = int(terrain.get("nz", 0))
	_resolution = float(terrain.get("resolution", 10.0))
	for child in get_children():
		child.queue_free()
	roads.clear()
	if _nx < 2 or _nz < 2:
		return
	_mean_height = 0.0
	for h in _heights:
		_mean_height += h
	_mean_height /= float(_heights.size())
	_hills.seed = 1337
	_hills.noise_type = FastNoiseLite.TYPE_SIMPLEX_SMOOTH
	_hills.frequency = 1.0 / 1400.0
	_hills.fractal_octaves = 4
	_woods.seed = 4242
	_woods.noise_type = FastNoiseLite.TYPE_SIMPLEX_SMOOTH
	_woods.frequency = 1.0 / 420.0
	_woods.fractal_octaves = 3
	_extend_river()
	_plan_roads()
	_build_textures()
	_build_material(weather)
	_add_mesh("Ground", _field_mesh(), true)
	_add_mesh("NearRing", _ring_mesh(NEAR_RECT, NEAR_STEP, Rect2(0, 0, FIELD_W, FIELD_D), 0.0), true)
	_add_mesh("FarRing", _ring_mesh(FAR_RECT, FAR_STEP, NEAR_RECT.grow(-2.0 * FAR_STEP), 1.5), false)
	if terrain.has("river"):
		_build_river(terrain["river"])
	_build_trees()
	_build_rocks()
	vegetation = BattleVegetation.new()
	vegetation.name = "Vegetation"
	add_child(vegetation)
	vegetation.build(self, weather)


## Hauteur bilinéaire du champ (même formule que la simulation) ; bornée au champ.
func height_at(x: float, z: float) -> float:
	if _nx < 2:
		return 0.0
	var fx := clampf(x / _resolution, 0.0, float(_nx - 1))
	var fz := clampf(z / _resolution, 0.0, float(_nz - 1))
	var ix := int(floor(fx))
	var iz := int(floor(fz))
	var ix1 := mini(ix + 1, _nx - 1)
	var iz1 := mini(iz + 1, _nz - 1)
	var tx := fx - ix
	var tz := fz - iz
	var top := _heights[iz * _nx + ix] * (1.0 - tx) + _heights[iz * _nx + ix1] * tx
	var bottom := _heights[iz1 * _nx + ix] * (1.0 - tx) + _heights[iz1 * _nx + ix1] * tx
	return top * (1.0 - tz) + bottom * tz


## Hauteur décorative partout : le champ dedans, collines et vallée de la rivière dehors.
func world_height(x: float, z: float) -> float:
	var cx := clampf(x, 0.0, FIELD_W)
	var cz := clampf(z, 0.0, FIELD_D)
	var base := height_at(cx, cz)
	var d := Vector2(x - cx, z - cz).length()
	if d <= 0.0:
		return base
	var t := smoothstep(0.0, 450.0, d)
	var n := _hills.get_noise_2d(x, z) * 0.5 + 0.5
	var hills := pow(n, 1.6) * lerpf(22.0, 160.0, smoothstep(300.0, 4500.0, d))
	var rd := river_distance(x, z)
	if rd < INF:
		hills *= smoothstep(25.0, 320.0, rd)
	var h := lerpf(base, _mean_height + hills, t)
	if rd < RIVER_SPAN:
		# Le lit : même profil que la simulation, raccordé au bord du champ.
		h -= RIVER_CARVE * (1.0 - rd / RIVER_SPAN) * t
	return h


func _in_zones(zones: Array, x: float, z: float, margin: float = 0.0) -> bool:
	for zone in zones:
		var dx: float = x - float(zone["x"])
		var dz: float = z - float(zone["z"])
		var r: float = float(zone["radius"]) + margin
		if dx * dx + dz * dz <= r * r:
			return true
	return false


## Distance au lit de la rivière (prolongée au-delà du champ), INF sans rivière.
func river_distance(x: float, z: float) -> float:
	if _river_points.size() < 2:
		return INF
	var best := INF
	var first_x := _river_points[0].x
	var center := int((x - first_x) / 10.0)
	for i in range(maxi(center - 4, 0), mini(center + 4, _river_points.size() - 1)):
		var p := Geometry2D.get_closest_point_to_segment(Vector2(x, z), _river_points[i], _river_points[i + 1])
		best = minf(best, p.distance_to(Vector2(x, z)))
	return best


func _in_ford(x: float) -> bool:
	if not terrain.has("river"):
		return false
	for ford in terrain["river"]["fords"]:
		if absf(x - float(ford["x"])) <= float(ford["half_width"]):
			return true
	return false


## La rivière de la simulation (x de 0 à 1200, un point tous les 10 m) prolongée par symétries
## successives (onde triangulaire en x) jusqu'aux anneaux lointains.
func _extend_river() -> void:
	_river_points = PackedVector2Array()
	if not terrain.has("river"):
		return
	var points: PackedVector2Array = terrain["river"]["points"]
	if points.size() < 2:
		return
	var span := points[points.size() - 1].x - points[0].x
	var reach := 2600.0
	var x := points[0].x - reach
	while x <= points[points.size() - 1].x + reach:
		var m := fposmod(x - points[0].x, 2.0 * span)
		if m > span:
			m = 2.0 * span - m
		var f := m / span * float(points.size() - 1)
		var i := mini(int(f), points.size() - 2)
		_river_points.append(Vector2(x, lerpf(points[i].y, points[i + 1].y, f - i)))
		x += 10.0


## Chemins décoratifs : un par gué (du sud au nord à travers le gué), sinon une route qui
## traverse le champ, plus un chemin de traverse à l'arrière ; en siège, la route de la porte.
func _plan_roads() -> void:
	var rng := RandomNumberGenerator.new()
	rng.seed = 90210 + _nx * 7 + int(_mean_height * 10.0)
	var crossings: Array[float] = []
	if terrain.has("river"):
		for ford in terrain["river"]["fords"]:
			crossings.append(float(ford["x"]))
	elif not terrain.has("siege"):
		crossings.append(rng.randf_range(350.0, 850.0))
	for x0 in crossings:
		var pts := PackedVector2Array()
		var x := x0 + rng.randf_range(-120.0, 120.0)
		for z in range(-1400, 2201, 100):
			var target := x0 if absf(float(z) - 400.0) < 250.0 else x
			x = lerpf(x, target, 0.5) + rng.randf_range(-35.0, 35.0)
			pts.append(Vector2(x, float(z)))
		roads.append(_smooth(pts))
	if terrain.has("siege"):
		var siege: Dictionary = terrain["siege"]
		var center: Vector2 = siege.get("center", Vector2(600, 560))
		var pieces: Array = siege.get("pieces", [])
		var gate_index := int(siege.get("gate", -1))
		if gate_index >= 0 and gate_index < pieces.size():
			var gate: Dictionary = pieces[gate_index]
			var mid: Vector2 = ((gate["a"] as Vector2) + (gate["b"] as Vector2)) * 0.5
			var out := (mid - center).normalized()
			var pts := PackedVector2Array([center, mid])
			var p := mid
			for i in 22:
				out = out.rotated(rng.randf_range(-0.12, 0.12))
				p += out * 90.0
				pts.append(p)
			roads.append(_smooth(pts))
	# Chemin de traverse (est-ouest) derrière l'une des lignes.
	var zr := -170.0 if rng.randf() < 0.5 else 980.0
	var side := PackedVector2Array()
	for x in range(-1600, 2801, 120):
		side.append(Vector2(float(x), zr + rng.randf_range(-30.0, 30.0) + sin(float(x) * 0.004) * 60.0))
	roads.append(_smooth(side))


## Lissage de Chaikin (deux passes).
static func _smooth(points: PackedVector2Array) -> PackedVector2Array:
	var result := points
	for _pass in 2:
		var next := PackedVector2Array([result[0]])
		for i in range(result.size() - 1):
			next.append(result[i].lerp(result[i + 1], 0.25))
			next.append(result[i].lerp(result[i + 1], 0.75))
		next.append(result[result.size() - 1])
		result = next
	return result


# --- Textures cuites ----------------------------------------------------------------------


func _build_textures() -> void:
	var sw := int(SPLAT_RECT.size.x / SPLAT_TEXEL)
	var sh := int(SPLAT_RECT.size.y / SPLAT_TEXEL)
	var a := Image.create_empty(sw, sh, false, Image.FORMAT_RGBA8)
	var b := Image.create_empty(sw, sh, false, Image.FORMAT_RGBA8)
	a.fill(Color(0, 0, 0, 0))
	b.fill(Color(0, 0, 0, 0))
	# Sous-bois et boue : disques des zones de la simulation (bord adouci).
	for zone in terrain.get("forests", []):
		_stamp_disc(a, Vector2(float(zone["x"]), float(zone["z"])), float(zone["radius"]) + 5.0, 0, 12.0)
	for zone in terrain.get("mud", []):
		_stamp_disc(a, Vector2(float(zone["x"]), float(zone["z"])), float(zone["radius"]), 2, 14.0)
	# Rivière : galets dans le lit et sur les gués, berges humides.
	if _river_points.size() >= 2:
		var width := float(terrain["river"]["width"])
		for i in range(_river_points.size() - 1):
			var p := _river_points[i]
			if not SPLAT_RECT.grow(40.0).has_point(p):
				continue
			var ford := _in_ford(p.x)
			_stamp_disc(a, p, width * (0.95 if ford else 0.7), 3, 4.0)
			_stamp_disc(b, p, width * 1.25, 0, 8.0)
	for road in roads:
		for i in range(road.size() - 1):
			var p0 := road[i]
			var p1 := road[i + 1]
			var steps := maxi(int(p0.distance_to(p1) / 2.0), 1)
			for s in steps:
				var p := p0.lerp(p1, float(s) / float(steps))
				if SPLAT_RECT.grow(10.0).has_point(p):
					_stamp_disc(a, p, 3.2, 1, 2.0)
					_stamp_disc(b, p, 9.0, 1, 6.0)
	if terrain.has("siege"):
		var siege: Dictionary = terrain["siege"]
		var center: Vector2 = siege.get("center", Vector2(600, 560))
		# Dans les murs : terre battue ; autour : pas de parcelles.
		_stamp_disc(a, center, 150.0, 1, 30.0, 0.55)
		_stamp_disc(b, center, 260.0, 1, 60.0)
	splat_a = ImageTexture.create_from_image(a)
	splat_b = ImageTexture.create_from_image(b)
	# Carte de hauteurs (herbe) : le champ à 10 m, prolongé par `world_height`.
	var hw := int(SPLAT_RECT.size.x / HEIGHT_TEXEL) + 1
	var hh := int(SPLAT_RECT.size.y / HEIGHT_TEXEL) + 1
	var hdata := PackedFloat32Array()
	hdata.resize(hw * hh)
	for iz in hh:
		var z := SPLAT_RECT.position.y + iz * HEIGHT_TEXEL
		for ix in hw:
			var x := SPLAT_RECT.position.x + ix * HEIGHT_TEXEL
			if x >= 0.0 and x <= FIELD_W and z >= 0.0 and z <= FIELD_D:
				hdata[iz * hw + ix] = height_at(x, z)
			else:
				hdata[iz * hw + ix] = world_height(x, z)
	var himage := Image.create_from_data(hw, hh, false, Image.FORMAT_RF, hdata.to_byte_array())
	height_texture = ImageTexture.create_from_image(himage)
	var noise := FastNoiseLite.new()
	noise.seed = 7
	noise.noise_type = FastNoiseLite.TYPE_SIMPLEX_SMOOTH
	noise.frequency = 1.0 / 64.0
	noise.fractal_octaves = 4
	var noise_image := noise.get_seamless_image(512, 512)
	noise_image.generate_mipmaps()
	macro_noise = ImageTexture.create_from_image(noise_image)


## Disque adouci dans le canal `channel` de `image` (coordonnées monde) ; garde le maximum.
func _stamp_disc(image: Image, center: Vector2, radius: float, channel: int, feather: float, strength: float = 1.0) -> void:
	var cx := (center.x - SPLAT_RECT.position.x) / SPLAT_TEXEL
	var cz := (center.y - SPLAT_RECT.position.y) / SPLAT_TEXEL
	var r := (radius + feather) / SPLAT_TEXEL
	var x0 := maxi(int(cx - r), 0)
	var x1 := mini(int(cx + r) + 1, image.get_width() - 1)
	var z0 := maxi(int(cz - r), 0)
	var z1 := mini(int(cz + r) + 1, image.get_height() - 1)
	for iz in range(z0, z1 + 1):
		for ix in range(x0, x1 + 1):
			var d := Vector2(ix - cx, iz - cz).length() * SPLAT_TEXEL
			if d > radius + feather:
				continue
			var v := (1.0 - smoothstep(radius - feather * 0.5, radius + feather, d)) * strength
			var c := image.get_pixel(ix, iz)
			if v > c[channel]:
				c[channel] = v
				image.set_pixel(ix, iz, c)


func _build_material(weather: String) -> void:
	ground_material = ShaderMaterial.new()
	ground_material.shader = GROUND_SHADER
	ground_material.set_shader_parameter("albedo_array", ALBEDO_ARRAY)
	ground_material.set_shader_parameter("normal_array", NORMAL_ARRAY)
	ground_material.set_shader_parameter("macro_noise", macro_noise)
	ground_material.set_shader_parameter("splat_a", splat_a)
	ground_material.set_shader_parameter("splat_b", splat_b)
	ground_material.set_shader_parameter("splat_rect", Vector4(SPLAT_RECT.position.x, SPLAT_RECT.position.y, SPLAT_RECT.size.x, SPLAT_RECT.size.y))
	var calm := Vector4(150.0, 60.0, 1050.0, 740.0)
	ground_material.set_shader_parameter("calm_rect", calm)
	match weather:
		"rain":
			ground_material.set_shader_parameter("wetness", 0.75)
			ground_material.set_shader_parameter("grass_tint", Color(0.9, 0.95, 0.85))
		"snow":
			ground_material.set_shader_parameter("snow", 0.85)
			ground_material.set_shader_parameter("grass_tint", Color(0.85, 0.85, 0.78))
		"fog":
			ground_material.set_shader_parameter("wetness", 0.25)
		_:
			ground_material.set_shader_parameter("grass_tint", Color(0.9, 1.0, 0.8))


# --- Maillages du sol ---------------------------------------------------------------------


func _add_mesh(node_name: String, mesh: ArrayMesh, shadows: bool) -> MeshInstance3D:
	var instance := MeshInstance3D.new()
	instance.name = node_name
	instance.mesh = mesh
	instance.material_override = ground_material
	instance.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_ON if shadows else GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	add_child(instance)
	return instance


## Grille du champ (10 m, hauteurs de la simulation) avec une jupe verticale sur le pourtour.
func _field_mesh() -> ArrayMesh:
	var vertices := PackedVector3Array()
	var normals := PackedVector3Array()
	vertices.resize(_nx * _nz)
	normals.resize(_nx * _nz)
	for iz in _nz:
		for ix in _nx:
			var i := iz * _nx + ix
			vertices[i] = Vector3(ix * _resolution, _heights[i], iz * _resolution)
			var hl := _heights[iz * _nx + maxi(ix - 1, 0)]
			var hr := _heights[iz * _nx + mini(ix + 1, _nx - 1)]
			var hd := _heights[maxi(iz - 1, 0) * _nx + ix]
			var hu := _heights[mini(iz + 1, _nz - 1) * _nx + ix]
			normals[i] = Vector3(hl - hr, 2.0 * _resolution, hd - hu).normalized()
	var indices := PackedInt32Array()
	for iz in range(_nz - 1):
		for ix in range(_nx - 1):
			var a := iz * _nx + ix
			var b := a + 1
			var c := a + _nx
			var d := c + 1
			indices.append_array([a, b, c, b, d, c])
	# Jupe : chaque sommet du bord dupliqué 3 m plus bas.
	var border: Array[int] = []
	for ix in _nx:
		border.append(ix)
	for iz in range(1, _nz):
		border.append(iz * _nx + _nx - 1)
	for ix in range(_nx - 2, -1, -1):
		border.append((_nz - 1) * _nx + ix)
	for iz in range(_nz - 2, -1, -1):
		border.append(iz * _nx)
	var base := vertices.size()
	for k in border.size():
		vertices.append(vertices[border[k]] - Vector3(0, 3.0, 0))
		normals.append(normals[border[k]])
	for k in border.size():
		var k1 := (k + 1) % border.size()
		var top0 := border[k]
		var top1 := border[k1]
		indices.append_array([top0, base + k, top1, top1, base + k, base + k1])
	return _commit(vertices, normals, indices)


## Anneau de terrain (grille `step`) autour de `hole` (quads entièrement dedans omis).
func _ring_mesh(rect: Rect2, step: float, hole: Rect2, sink: float) -> ArrayMesh:
	var nx := int(rect.size.x / step) + 1
	var nz := int(rect.size.y / step) + 1
	var vertices := PackedVector3Array()
	var normals := PackedVector3Array()
	vertices.resize(nx * nz)
	normals.resize(nx * nz)
	var heights := PackedFloat32Array()
	heights.resize(nx * nz)
	for iz in nz:
		for ix in nx:
			var x := rect.position.x + ix * step
			var z := rect.position.y + iz * step
			var h := world_height(x, z)
			if hole.has_point(Vector2(x, z)):
				h -= sink
			heights[iz * nx + ix] = h
			vertices[iz * nx + ix] = Vector3(x, h, z)
	for iz in nz:
		for ix in nx:
			var hl := heights[iz * nx + maxi(ix - 1, 0)]
			var hr := heights[iz * nx + mini(ix + 1, nx - 1)]
			var hd := heights[maxi(iz - 1, 0) * nx + ix]
			var hu := heights[mini(iz + 1, nz - 1) * nx + ix]
			normals[iz * nx + ix] = Vector3(hl - hr, 2.0 * step, hd - hu).normalized()
	var indices := PackedInt32Array()
	for iz in range(nz - 1):
		for ix in range(nx - 1):
			var x0 := rect.position.x + ix * step
			var z0 := rect.position.y + iz * step
			if hole.encloses(Rect2(x0, z0, step, step)):
				continue
			var a := iz * nx + ix
			var b := a + 1
			var c := a + nx
			var d := c + 1
			indices.append_array([a, b, c, b, d, c])
	return _commit(vertices, normals, indices)


static func _commit(vertices: PackedVector3Array, normals: PackedVector3Array, indices: PackedInt32Array) -> ArrayMesh:
	var arrays := []
	arrays.resize(Mesh.ARRAY_MAX)
	arrays[Mesh.ARRAY_VERTEX] = vertices
	arrays[Mesh.ARRAY_NORMAL] = normals
	arrays[Mesh.ARRAY_INDEX] = indices
	var mesh := ArrayMesh.new()
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	return mesh


# --- Rivière ------------------------------------------------------------------------------


## Ruban d'eau le long du lit prolongé ; niveau pris sur les berges (non creusées) de part et
## d'autre, lissé, pour que l'eau affleure les rives et reste peu profonde sur les gués.
func _build_river(river: Dictionary) -> void:
	var points := _river_points
	if points.size() < 2:
		return
	# Niveau : au-dessus du fond du lit (0,5 m d'eau, 0,2 m sur les gués), lissé le long du cours.
	var levels := PackedFloat32Array()
	levels.resize(points.size())
	var dirs: Array[Vector2] = []
	for i in points.size():
		var a := points[maxi(i - 1, 0)]
		var b := points[mini(i + 1, points.size() - 1)]
		var dir := (b - a).normalized()
		dirs.append(Vector2(-dir.y, dir.x))
		var p := points[i]
		var inside := p.x >= 0.0 and p.x <= FIELD_W
		levels[i] = world_height(p.x, p.y) + (0.2 if inside and _in_ford(p.x) else 0.5)
	for _pass in 8:
		var smoothed := levels.duplicate()
		for i in range(1, points.size() - 1):
			smoothed[i] = (levels[i - 1] + levels[i] * 2.0 + levels[i + 1]) * 0.25
		levels = smoothed
	# Largeur de chaque rive : jusqu'où le sol reste sous l'eau, plus une marge sous la berge.
	var vertices := PackedVector3Array()
	var uvs := PackedVector2Array()
	var normals := PackedVector3Array()
	var indices := PackedInt32Array()
	var along := 0.0
	var width := 0.0
	for i in points.size():
		var p := points[i]
		var n := dirs[i]
		var extent := [0.0, 0.0]
		for s in 2:
			var sign := 1.0 if s == 0 else -1.0
			var d := 1.0
			while d < 16.0:
				var q := p + n * sign * d
				if world_height(q.x, q.y) > levels[i] + 0.08:
					break
				d += 1.0
			extent[s] = d + 2.5
		if i > 0:
			along += p.distance_to(points[i - 1])
		var left := p + n * float(extent[0])
		var right := p - n * float(extent[1])
		width = maxf(width, float(extent[0]) + float(extent[1]))
		vertices.append(Vector3(left.x, levels[i], left.y))
		vertices.append(Vector3(right.x, levels[i], right.y))
		uvs.append(Vector2(0.0, along))
		uvs.append(Vector2(1.0, along))
		normals.append(Vector3.UP)
		normals.append(Vector3.UP)
		if i > 0:
			var k := i * 2
			indices.append_array([k - 2, k - 1, k, k - 1, k + 1, k])
	var half := width * 0.5
	var arrays := []
	arrays.resize(Mesh.ARRAY_MAX)
	arrays[Mesh.ARRAY_VERTEX] = vertices
	arrays[Mesh.ARRAY_NORMAL] = normals
	arrays[Mesh.ARRAY_TEX_UV] = uvs
	arrays[Mesh.ARRAY_INDEX] = indices
	var mesh := ArrayMesh.new()
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	var mat := ShaderMaterial.new()
	mat.shader = WATER_SHADER
	mat.set_shader_parameter("river_width", half * 2.0)
	var waves := NoiseTexture2D.new()
	waves.seamless = true
	waves.as_normal_map = true
	waves.bump_strength = 6.0
	waves.width = 256
	waves.height = 256
	var wave_noise := FastNoiseLite.new()
	wave_noise.frequency = 0.035
	wave_noise.fractal_octaves = 3
	waves.noise = wave_noise
	mat.set_shader_parameter("wave_normal", waves)
	mat.set_shader_parameter("macro_noise", macro_noise)
	if weather_key == "rain":
		mat.set_shader_parameter("turbidity", 0.75)
		mat.set_shader_parameter("ripple", 1.0)
	elif weather_key == "snow":
		mat.set_shader_parameter("deep_color", Color(0.05, 0.09, 0.11))
	var instance := MeshInstance3D.new()
	instance.name = "River"
	instance.mesh = mesh
	instance.material_override = mat
	instance.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	add_child(instance)


# --- Arbres, buissons, rochers -----------------------------------------------------------


func _near_road(p: Vector2, margin: float) -> bool:
	for road in roads:
		for i in range(road.size() - 1):
			var a := road[i]
			var b := road[i + 1]
			if absf(a.y - p.y) > 200.0 and absf(b.y - p.y) > 200.0 and absf(a.x - p.x) > 200.0:
				continue
			if Geometry2D.get_closest_point_to_segment(p, a, b).distance_to(p) < margin:
				return true
	return false


## Place un arbre : transformée (échelle, lacet) + teinte par instance.
func _tree_transform(rng: RandomNumberGenerator, x: float, z: float, scale_min: float, scale_max: float) -> Transform3D:
	var s := rng.randf_range(scale_min, scale_max)
	var basis := Basis(Vector3.UP, rng.randf() * TAU).scaled(Vector3(s, s * rng.randf_range(0.9, 1.15), s))
	return Transform3D(basis, Vector3(x, world_height(x, z) - 0.25, z))


func _tree_tint(rng: RandomNumberGenerator) -> Color:
	var autumn := rng.randf() < 0.12
	if autumn:
		return Color(rng.randf_range(1.05, 1.25), rng.randf_range(0.9, 1.0), rng.randf_range(0.55, 0.7))
	var v := rng.randf_range(0.78, 1.12)
	return Color(v * rng.randf_range(0.9, 1.05), v, v * rng.randf_range(0.85, 1.05))


func _build_trees() -> void:
	var rng := RandomNumberGenerator.new()
	rng.seed = 1337
	var sets := {"oak": [], "poplar": [], "bush": [], "far": []}
	var tints := {"oak": [], "poplar": [], "bush": [], "far": []}
	var siege_center := Vector2(-1e6, -1e6)
	if terrain.has("siege"):
		siege_center = terrain["siege"].get("center", Vector2(600, 560))
	# Bois de la simulation : denses, lisière de buissons.
	for zone in terrain.get("forests", []):
		var r := float(zone["radius"])
		var count := clampi(int(PI * r * r / 55.0), 10, 700)
		for _i in count:
			var angle := rng.randf() * TAU
			var dist := sqrt(rng.randf()) * r
			var x := float(zone["x"]) + cos(angle) * dist
			var z := float(zone["z"]) + sin(angle) * dist
			var kind := "poplar" if rng.randf() < 0.15 else "oak"
			sets[kind].append(_tree_transform(rng, x, z, 0.75, 1.3))
			tints[kind].append(_tree_tint(rng))
		for _i in int(TAU * r / 6.0):
			var angle := rng.randf() * TAU
			var x := float(zone["x"]) + cos(angle) * (r + rng.randf_range(-2.0, 5.0))
			var z := float(zone["z"]) + sin(angle) * (r + rng.randf_range(-2.0, 5.0))
			sets["bush"].append(_tree_transform(rng, x, z, 0.6, 1.4))
			tints["bush"].append(_tree_tint(rng))
	var field := Rect2(0, 0, FIELD_W, FIELD_D)
	# Bois décoratifs de l'anneau proche, haies et arbres isolés.
	var step := 13.0
	var z := NEAR_RECT.position.y
	while z < NEAR_RECT.end.y:
		var x := NEAR_RECT.position.x
		while x < NEAR_RECT.end.x:
			var px := x + rng.randf_range(-5.0, 5.0)
			var pz := z + rng.randf_range(-5.0, 5.0)
			x += step
			var p := Vector2(px, pz)
			if field.grow(25.0).has_point(p) or p.distance_to(siege_center) < 280.0:
				continue
			var n := _woods.get_noise_2d(px, pz)
			var isolated := rng.randf() < 0.004
			if n < 0.28 and not isolated:
				continue
			if river_distance(px, pz) < RIVER_SPAN + 6.0 or _near_road(p, 9.0):
				continue
			if n >= 0.28 and n < 0.34 and rng.randf() < 0.6:
				sets["bush"].append(_tree_transform(rng, px, pz, 0.7, 1.5))
				tints["bush"].append(_tree_tint(rng))
				continue
			var kind := "poplar" if rng.randf() < (0.35 if isolated else 0.1) else "oak"
			sets[kind].append(_tree_transform(rng, px, pz, 0.8, 1.35))
			tints[kind].append(_tree_tint(rng))
		z += step
	# Quelques buissons épars dans le champ, hors du centre.
	for _i in 140:
		var p := Vector2(rng.randf_range(0.0, FIELD_W), rng.randf_range(0.0, FIELD_D))
		if absf(p.x - 600.0) < 420.0 and p.y > 120.0 and p.y < 680.0:
			continue
		if river_distance(p.x, p.y) < RIVER_SPAN or _near_road(p, 6.0) or p.distance_to(siege_center) < 200.0:
			continue
		sets["bush"].append(_tree_transform(rng, p.x, p.y, 0.6, 1.3))
		tints["bush"].append(_tree_tint(rng))
	# Bois lointains (anneau lointain) : arbres simplifiés, plus gros, sans ombre.
	step = 42.0
	z = FAR_RECT.position.y
	while z < FAR_RECT.end.y:
		var x := FAR_RECT.position.x
		while x < FAR_RECT.end.x:
			var px := x + rng.randf_range(-15.0, 15.0)
			var pz := z + rng.randf_range(-15.0, 15.0)
			x += step
			if NEAR_RECT.grow(-60.0).has_point(Vector2(px, pz)):
				continue
			if _woods.get_noise_2d(px * 0.6, pz * 0.6) < 0.22:
				continue
			sets["far"].append(_tree_transform(rng, px, pz, 1.3, 2.0))
			tints["far"].append(_tree_tint(rng))
		z += step
	tree_count = 0
	for kind in sets:
		tree_count += (sets[kind] as Array).size()
		_tree_layer(kind, sets[kind], tints[kind])


func _tree_layer(kind: String, transforms: Array, tints: Array) -> void:
	if transforms.is_empty():
		return
	var mm := MultiMesh.new()
	mm.transform_format = MultiMesh.TRANSFORM_3D
	mm.use_colors = true
	mm.mesh = BattleMeshes.tree(kind)
	mm.instance_count = transforms.size()
	for i in transforms.size():
		mm.set_instance_transform(i, transforms[i])
		mm.set_instance_color(i, tints[i])
	var instance := MultiMeshInstance3D.new()
	instance.name = "Trees_%s" % kind
	instance.multimesh = mm
	instance.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF if kind == "far" else GeometryInstance3D.SHADOW_CASTING_SETTING_ON
	add_child(instance)


## Rochers : sur les pentes raides du champ et dans les collines de l'anneau proche.
func _build_rocks() -> void:
	var rng := RandomNumberGenerator.new()
	rng.seed = 777
	var transforms: Array[Transform3D] = []
	for _i in 2600:
		var x := rng.randf_range(NEAR_RECT.position.x, NEAR_RECT.end.x)
		var z := rng.randf_range(NEAR_RECT.position.y, NEAR_RECT.end.y)
		var h := world_height(x, z)
		var slope := Vector2(world_height(x + 4.0, z) - world_height(x - 4.0, z), world_height(x, z + 4.0) - world_height(x, z - 4.0)).length() / 8.0
		var in_field := x >= 0.0 and x <= FIELD_W and z >= 0.0 and z <= FIELD_D
		var chance := smoothstep(0.12, 0.35, slope) + (0.0 if in_field else 0.04)
		if rng.randf() > chance or river_distance(x, z) < RIVER_SPAN:
			continue
		if in_field and absf(x - 600.0) < 380.0 and z > 150.0 and z < 650.0:
			continue
		var s := rng.randf_range(0.5, 2.6)
		var basis := Basis(Vector3.UP, rng.randf() * TAU).rotated(Vector3.RIGHT, rng.randf_range(-0.3, 0.3)).scaled(Vector3(s * rng.randf_range(0.8, 1.5), s * rng.randf_range(0.5, 0.9), s))
		transforms.append(Transform3D(basis, Vector3(x, h - s * 0.25, z)))
	if transforms.is_empty():
		return
	var mm := MultiMesh.new()
	mm.transform_format = MultiMesh.TRANSFORM_3D
	mm.mesh = BattleMeshes.rock()
	mm.instance_count = transforms.size()
	for i in transforms.size():
		mm.set_instance_transform(i, transforms[i])
	var instance := MultiMeshInstance3D.new()
	instance.name = "Rocks"
	instance.multimesh = mm
	add_child(instance)
