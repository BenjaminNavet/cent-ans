class_name NavalSea
extends Node3D

## Mer des batailles navales (lot NV1) : maillage fin qui suit la caméra (houle déplacée,
## `naval_sea.gdshader`) posé sur un grand plan lointain sans déplacement jusqu'à l'horizon ;
## rivage d'estuaire en option (l'Écluse : terre basse, dunes, clochers au loin). Les navires lisent
## la houle par `waves` (`NavalWaves`). Rendu seulement.

const SEA_SHADER := preload("res://shaders/naval_sea.gdshader")
const NEAR_SIZE := 1800.0
const NEAR_CELLS := 300
const FAR_SIZE := 40000.0
const SNAP := 12.0
const MAX_SHIPS := 40

var waves := NavalWaves.new()
var time: float = 0.0
var shore: bool = false
var shore_x: float = 560.0
var material: ShaderMaterial
var _near: MeshInstance3D
var _far: MeshInstance3D
var _far_material: ShaderMaterial


func setup(wind_to: float, strength: float, sky: Color, p_shore: bool, p_shore_x: float) -> void:
	name = "Sea"
	shore = p_shore
	shore_x = p_shore_x
	waves.configure(wind_to, strength)
	material = ShaderMaterial.new()
	material.shader = SEA_SHADER
	var normal := NoiseTexture2D.new()
	normal.seamless = true
	normal.as_normal_map = true
	normal.bump_strength = 5.0
	normal.width = 512
	normal.height = 512
	var noise := FastNoiseLite.new()
	noise.frequency = 0.03
	noise.fractal_octaves = 3
	normal.noise = noise
	var macro := NoiseTexture2D.new()
	macro.seamless = true
	macro.width = 256
	macro.height = 256
	var macro_noise := FastNoiseLite.new()
	macro_noise.frequency = 0.02
	macro.noise = macro_noise
	material.set_shader_parameter("wave_normal", normal)
	material.set_shader_parameter("macro_noise", macro)
	material.set_shader_parameter("sky_color", sky)
	material.set_shader_parameter("shore_on", shore)
	material.set_shader_parameter("shore_x", shore_x)
	material.set_shader_parameter("choppiness", 0.6 + strength)
	waves.apply(material)
	var plane := PlaneMesh.new()
	plane.size = Vector2(NEAR_SIZE, NEAR_SIZE)
	plane.subdivide_width = NEAR_CELLS
	plane.subdivide_depth = NEAR_CELLS
	_near = MeshInstance3D.new()
	_near.name = "NearSea"
	_near.mesh = plane
	_near.material_override = material
	_near.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	# Boîte élargie : la houle soulève les sommets.
	_near.extra_cull_margin = 4.0
	add_child(_near)
	_far_material = material.duplicate() as ShaderMaterial
	_far_material.set_shader_parameter("displace", false)
	var far_plane := PlaneMesh.new()
	far_plane.size = Vector2(FAR_SIZE, FAR_SIZE)
	_far = MeshInstance3D.new()
	_far.name = "FarSea"
	_far.mesh = far_plane
	_far.material_override = _far_material
	_far.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	_far.position.y = -0.8  # sous le maillage fin (pas de scintillement au raccord)
	add_child(_far)
	if shore:
		_build_shore()


## Avance le temps de la mer (temps de bataille : la pause fige la houle) et recentre le
## maillage fin sous la caméra.
func update(p_time: float, focus: Vector3, ships: Array) -> void:
	time = p_time
	var snapped := Vector3(snappedf(focus.x, SNAP), 0.0, snappedf(focus.z, SNAP))
	_near.position = snapped
	_far.position.x = snapped.x
	_far.position.z = snapped.z
	for m in [material, _far_material]:
		m.set_shader_parameter("sea_time", time)
		m.set_shader_parameter("displace_center", Vector2(focus.x, focus.z))
	var data: Array[Vector4] = []
	var motion: Array[Vector2] = []
	for ship in ships:
		if data.size() >= MAX_SHIPS:
			break
		if str(ship.get("status", "")) in ["sunk", "escaped"]:
			continue
		data.append(Vector4(float(ship["x"]), float(ship["z"]), float(ship["heading"]), float(ship["length"])))
		motion.append(Vector2(float(ship.get("speed", 0.0)), float(ship["beam"])))
	var count := data.size()
	while data.size() < MAX_SHIPS:
		data.append(Vector4.ZERO)
		motion.append(Vector2.ZERO)
	material.set_shader_parameter("ships", data)
	material.set_shader_parameter("ship_motion", motion)
	material.set_shader_parameter("ship_count", count)


func height_at(x: float, z: float) -> float:
	return waves.height(x, z, time)


## Terre basse de l'estuaire : grève, dunes et prés, quelques clochers de la ville au loin.
func _build_shore() -> void:
	var land := MeshInstance3D.new()
	land.name = "Shore"
	var cells_x := 60
	var cells_z := 80
	var size_x := 2600.0
	var size_z := 5200.0
	var noise := FastNoiseLite.new()
	noise.frequency = 0.004
	noise.seed = 1340
	var st := SurfaceTool.new()
	st.begin(Mesh.PRIMITIVE_TRIANGLES)
	var sand := Color(0.62, 0.55, 0.4)
	var grass := Color(0.26, 0.33, 0.16)
	var mud := Color(0.28, 0.25, 0.2)
	var heights := []
	for j in cells_z + 1:
		var row := []
		for i in cells_x + 1:
			var x := shore_x - 60.0 + size_x * float(i) / cells_x
			var z := -size_z * 0.5 + size_z * float(j) / cells_z
			var inland := x - shore_x
			var h := -2.5 + clampf(inland / 70.0, 0.0, 1.0) * 4.0 + maxf(inland, 0.0) * 0.004
			h += noise.get_noise_2d(x, z) * clampf(inland / 200.0, 0.0, 1.0) * 6.0
			row.append(h)
		heights.append(row)
	for j in cells_z:
		for i in cells_x:
			for corner in [[0, 0], [1, 0], [1, 1], [0, 0], [1, 1], [0, 1]]:
				var ci: int = i + corner[0]
				var cj: int = j + corner[1]
				var x := shore_x - 60.0 + size_x * float(ci) / cells_x
				var z := -size_z * 0.5 + size_z * float(cj) / cells_z
				var h: float = heights[cj][ci]
				var inland := x - shore_x
				var color := mud.lerp(sand, clampf((h + 1.0) / 1.5, 0.0, 1.0)).lerp(grass, clampf((inland - 90.0) / 120.0, 0.0, 1.0))
				st.set_color(color)
				st.add_vertex(Vector3(x, h, z))
	st.generate_normals()
	land.mesh = st.commit()
	var mat := StandardMaterial3D.new()
	mat.vertex_color_use_as_albedo = true
	mat.roughness = 0.95
	land.material_override = mat
	add_child(land)
	# La ville et ses clochers, au fond de l'estuaire.
	var town := ModelLibrary.instantiate("town", 60.0)
	if town != null:
		town.position = Vector3(shore_x + 520.0, 1.0, 240.0)
		town.rotation.y = 0.6
		add_child(town)
	var church := ModelLibrary.instantiate("cathedral", 45.0)
	if church != null:
		church.position = Vector3(shore_x + 700.0, 1.5, -380.0)
		add_child(church)
