class_name BattleTerrainMesh
extends RefCounted

## Maillages du terrain de bataille (SC BT7, découpé de `battle_terrain.gd`) : sol, anneaux (en Rust),
## rivière, ruisseaux et pierres des gués. L'état reste sur `BattleTerrain` (`host`).

var host: BattleTerrain


func _init(p_terrain: BattleTerrain) -> void:
	host = p_terrain


func _add_mesh(node_name: String, mesh: ArrayMesh, shadows: bool) -> MeshInstance3D:
	var instance := MeshInstance3D.new()
	instance.name = node_name
	instance.mesh = mesh
	instance.material_override = host.ground_material
	instance.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_ON if shadows else GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	instance.layers |= host.DECAL_LAYER
	host.add_child(instance)
	return instance


## Grille du champ (10 m, hauteurs de la simulation) avec une jupe verticale sur le pourtour.
func _field_mesh() -> ArrayMesh:
	return host._kernel.field_mesh()


## Anneau de terrain (grille `step`) autour de `hole` (quads entièrement dedans omis).
func _ring_mesh(rect: Rect2, step: float, hole: Rect2, sink: float) -> ArrayMesh:
	return host._kernel.ring_mesh(rect, step, hole, sink)


## Ruban d'eau le long du lit prolongé ; niveau pris sur les berges (non creusées) de part et
## d'autre, lissé, pour que l'eau affleure les rives et reste peu profonde sur les gués.
func _build_river(river: Dictionary) -> void:
	var points := host._river_points
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
		var inside := p.x >= 0.0 and p.x <= host.FIELD_W
		levels[i] = host.world_height(p.x, p.y) + (0.2 if inside and host._in_ford(p.x) else 0.5)
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
		var reach := (host._river_widths[i] if i < host._river_widths.size() else 18.0) * 0.5 + 8.0
		for s in 2:
			var sign := 1.0 if s == 0 else -1.0
			var d := 1.0
			while d < reach:
				var q := p + n * sign * d
				if host.world_height(q.x, q.y) > levels[i] + 0.08:
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
	host._river_levels = levels
	var arrays := []
	arrays.resize(Mesh.ARRAY_MAX)
	arrays[Mesh.ARRAY_VERTEX] = vertices
	arrays[Mesh.ARRAY_NORMAL] = normals
	arrays[Mesh.ARRAY_TEX_UV] = uvs
	arrays[Mesh.ARRAY_INDEX] = indices
	var mesh := ArrayMesh.new()
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	var mat := ShaderMaterial.new()
	mat.shader = host.WATER_SHADER
	mat.set_shader_parameter("river_width", half * 2.0)
	# L'eau coule vers le bas du champ (UV.y croît avec x).
	mat.set_shader_parameter("flow_speed", 0.55 * host.river_flow)
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
	mat.set_shader_parameter("macro_noise", host.macro_noise)
	# Reflet : le ciel entre horizon et zénith du préréglage météo.
	var preset: Dictionary = BattleAtmosphere.PRESETS.get(host.weather_key, BattleAtmosphere.PRESETS["clear"])
	mat.set_shader_parameter("sky_color", (preset["horizon"] as Color).lerp(preset["zenith"], 0.3))
	if host.weather_key == "rain":
		mat.set_shader_parameter("turbidity", 0.75)
		mat.set_shader_parameter("ripple", 1.0)
	elif host.weather_key == "snow":
		mat.set_shader_parameter("deep_color", Color(0.05, 0.09, 0.11))
	var instance := MeshInstance3D.new()
	instance.name = "River"
	instance.mesh = mesh
	instance.material_override = mat
	instance.layers |= host.DECAL_LAYER
	instance.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	host.add_child(instance)


## Affluent et ruisseaux, rubans d'eau étroits (même shader que la rivière) ; le niveau suit
## le lit creusé par la simulation, lissé, sous les berges.
func _build_streams() -> void:
	for stream in host.terrain.get("streams", []):
		var raw: PackedVector2Array = stream["points"]
		if raw.size() < 2:
			continue
		var width := float(stream["width"])
		# Rééchantillonnage tous les 4 m.
		var pts := PackedVector2Array([raw[0]])
		for i in range(raw.size() - 1):
			var steps := maxi(int(raw[i].distance_to(raw[i + 1]) / 4.0), 1)
			for s in range(1, steps + 1):
				pts.append(raw[i].lerp(raw[i + 1], float(s) / float(steps)))
		host._streams.append({"points": pts, "width": width, "kind": str(stream["kind"])})
		host._add_stream_chunks(pts, width * 0.5)
		var levels := PackedFloat32Array()
		levels.resize(pts.size())
		for i in pts.size():
			levels[i] = host.height_at(pts[i].x, pts[i].y) + (0.22 if str(stream["kind"]) == "tributary" else 0.14)
		for _pass in 6:
			var smoothed := levels.duplicate()
			for i in range(1, pts.size() - 1):
				smoothed[i] = minf(levels[i], (levels[i - 1] + levels[i] * 2.0 + levels[i + 1]) * 0.25)
			levels = smoothed
		var vertices := PackedVector3Array()
		var uvs := PackedVector2Array()
		var normals := PackedVector3Array()
		var indices := PackedInt32Array()
		var along := 0.0
		for i in pts.size():
			var a := pts[maxi(i - 1, 0)]
			var b := pts[mini(i + 1, pts.size() - 1)]
			var dir := (b - a).normalized()
			var n := Vector2(-dir.y, dir.x)
			var half := width * 0.5 + 0.8
			if i > 0:
				along += pts[i].distance_to(pts[i - 1])
			var left := pts[i] + n * half
			var right := pts[i] - n * half
			vertices.append(Vector3(left.x, levels[i], left.y))
			vertices.append(Vector3(right.x, levels[i], right.y))
			uvs.append(Vector2(0.0, along))
			uvs.append(Vector2(1.0, along))
			normals.append(Vector3.UP)
			normals.append(Vector3.UP)
			if i > 0:
				var k := i * 2
				indices.append_array([k - 2, k - 1, k, k - 1, k + 1, k])
		var arrays := []
		arrays.resize(Mesh.ARRAY_MAX)
		arrays[Mesh.ARRAY_VERTEX] = vertices
		arrays[Mesh.ARRAY_NORMAL] = normals
		arrays[Mesh.ARRAY_TEX_UV] = uvs
		arrays[Mesh.ARRAY_INDEX] = indices
		var mesh := ArrayMesh.new()
		mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
		var mat := ShaderMaterial.new()
		mat.shader = host.WATER_SHADER
		mat.set_shader_parameter("river_width", width + 1.6)
		mat.set_shader_parameter("flow_speed", 0.8)
		mat.set_shader_parameter("clarity", 1.6)
		mat.set_shader_parameter("turbidity", 0.15)
		mat.set_shader_parameter("macro_noise", host.macro_noise)
		mat.set_shader_parameter("wave_normal", host.water_waves())
		mat.set_shader_parameter("sky_color", host.sky_reflection())
		var instance := MeshInstance3D.new()
		instance.name = "Stream"
		instance.mesh = mesh
		instance.material_override = mat
		instance.layers |= host.DECAL_LAYER
		instance.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		instance.visibility_range_end = 1400.0
		host.add_child(instance)


## Gués visibles : pierres et galets qui affleurent en travers du courant.
func _build_ford_stones(river: Dictionary) -> void:
	var fords: Array = river.get("fords", [])
	if fords.is_empty():
		return
	var rng := RandomNumberGenerator.new()
	rng.seed = 4711
	var transforms: Array[Transform3D] = []
	for ford in fords:
		var fx := float(ford["x"])
		var half := float(ford["half_width"])
		for _i in int(half * 5.0):
			var x := fx + rng.randf_range(-half, half)
			var w := host.river_width_at(x)
			var z := host.river_center_z(x) + rng.randf_range(-0.55, 0.55) * w
			var s := rng.randf_range(0.18, 0.55)
			var basis := Basis(Vector3.UP, rng.randf() * TAU).scaled(Vector3(s * rng.randf_range(1.0, 1.6), s * rng.randf_range(0.4, 0.7), s))
			var level := host.water_level_at(x)
			var y := host.height_at(x, z) + s * 0.1
			if level != -INF:
				y = minf(y, level + s * 0.15)
			transforms.append(Transform3D(basis, Vector3(x, y, z)))
	MultiMeshKit.make(BattleMeshes.rock(), transforms, {"name": "FordStones", "range_end": 700.0, "parent": host})
