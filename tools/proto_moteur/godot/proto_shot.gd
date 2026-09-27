extends SceneTree
## Builds the shared prototype scene from scene.json, lets GI and TAA settle, saves a screenshot.
## Run: godot --path tools/proto_moteur/godot --script res://proto_shot.gd -- <out.png>

const SETTLE_FRAMES := 300

var _frames := 0
var _out_path := "user://godot.png"


func _initialize() -> void:
	var args := OS.get_cmdline_user_args()
	if not args.is_empty():
		_out_path = args[0]
	var scene: Dictionary = JSON.parse_string(FileAccess.get_file_as_string("res://assets/scene.json"))
	var world := Node3D.new()
	root.add_child(world)
	var cache := {}
	for placement in scene["placements"]:
		world.add_child(_instance(placement, cache))
	_add_camera(world, scene["camera"])
	_add_sun_and_sky(world, scene["sun"], scene["fog"])


func _instance(placement: Dictionary, cache: Dictionary) -> Node3D:
	var asset: String = placement["asset"]
	if not cache.has(asset):
		cache[asset] = load("res://assets/%s.glb" % asset)
	var node: Node3D = (cache[asset] as PackedScene).instantiate()
	var part: String = placement["node"]
	if part != "":
		var child := node.find_child(part, true, false) as Node3D
		assert(child != null, "node %s missing in %s" % [part, asset])
		child.get_parent().remove_child(child)
		node.free()
		node = child
		node.transform = Transform3D.IDENTITY
	var p: Array = placement["pos"]
	node.position = Vector3(p[0], p[1], p[2])
	node.rotation_degrees.y = placement["yaw"]
	node.scale *= float(placement["scale"])
	return node


func _add_camera(world: Node3D, spec: Dictionary) -> void:
	var camera := Camera3D.new()
	world.add_child(camera)
	var p: Array = spec["pos"]
	var t: Array = spec["look_at"]
	camera.look_at_from_position(Vector3(p[0], p[1], p[2]), Vector3(t[0], t[1], t[2]))
	camera.fov = spec["fov_y"]
	camera.far = 2000.0
	camera.make_current()


func _add_sun_and_sky(world: Node3D, sun_spec: Dictionary, fog_spec: Dictionary) -> void:
	var sun := DirectionalLight3D.new()
	world.add_child(sun)
	# Azimuth measured from -Z (camera forward) turning toward +X, elevation above the horizon.
	sun.rotation_degrees = Vector3(-float(sun_spec["elevation"]), 180.0 - float(sun_spec["azimuth"]), 0)
	var c: Array = sun_spec["color"]
	sun.light_color = Color(c[0], c[1], c[2])
	sun.light_energy = 2.4
	sun.light_angular_distance = 0.6
	sun.shadow_enabled = true
	sun.directional_shadow_mode = DirectionalLight3D.SHADOW_PARALLEL_4_SPLITS
	sun.directional_shadow_max_distance = 150.0
	sun.directional_shadow_blend_splits = true
	sun.light_volumetric_fog_energy = 1.5

	var sky_material := PhysicalSkyMaterial.new()
	sky_material.turbidity = 6.0
	sky_material.energy_multiplier = 1.0
	var sky := Sky.new()
	sky.sky_material = sky_material
	var env := Environment.new()
	env.background_mode = Environment.BG_SKY
	env.sky = sky
	env.ambient_light_source = Environment.AMBIENT_SOURCE_SKY
	env.reflected_light_source = Environment.REFLECTION_SOURCE_SKY
	env.tonemap_mode = Environment.TONE_MAPPER_AGX
	env.tonemap_exposure = 1.0
	env.sdfgi_enabled = true
	env.sdfgi_use_occlusion = true
	env.sdfgi_cascades = 6
	env.sdfgi_min_cell_size = 0.1
	env.ssil_enabled = true
	env.ssao_enabled = true
	env.glow_enabled = true
	env.glow_intensity = 0.4
	env.volumetric_fog_enabled = true
	env.volumetric_fog_density = float(fog_spec["density"])
	var f: Array = fog_spec["color"]
	env.volumetric_fog_albedo = Color(f[0], f[1], f[2])
	env.volumetric_fog_anisotropy = 0.6
	env.adjustment_enabled = true
	env.adjustment_saturation = 1.05
	var world_env := WorldEnvironment.new()
	world_env.environment = env
	world.add_child(world_env)


func _process(_delta: float) -> bool:
	_frames += 1
	if _frames < SETTLE_FRAMES:
		return false
	var image := root.get_viewport().get_texture().get_image()
	image.save_png(_out_path)
	print("PROTO godot shot saved ", _out_path)
	return true
