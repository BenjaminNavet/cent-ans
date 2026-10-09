extends SceneTree

## Lot AS1 : planches des bêtes animées (carte et camp), écrites en PNG pour un regard humain, et
## mesure numérique du mouvement (écart moyen entre images successives, imprimé).
## Usage (avec affichage) :
##   godot --path game --resolution 1280x720 --script res://tests/as1_shot.gd -- --out=<dossier>
## Sorties : <dossier>/idle_N.png, walk_N.png, cart_N.png, camp_N.png (N = 0..3).

const BEASTS := ["ox", "cow", "horse", "sheep"]
const CARTS := ["stone_cart", "merchant_cart", "dead_cart"]
const FRAMES := 4

var _out := ""


func _init() -> void:
	_out = CmdArgs.value("--out", _out)
	_run()


func _scene() -> Node3D:
	var world := Node3D.new()
	root.add_child(world)
	var sun := DirectionalLight3D.new()
	sun.rotation_degrees = Vector3(-50, 40, 0)
	world.add_child(sun)
	var env := WorldEnvironment.new()
	env.environment = Environment.new()
	env.environment.background_mode = Environment.BG_COLOR
	env.environment.background_color = Color(0.55, 0.65, 0.78)
	env.environment.ambient_light_source = Environment.AMBIENT_SOURCE_COLOR
	env.environment.ambient_light_color = Color(0.7, 0.7, 0.7)
	world.add_child(env)
	var ground := MeshInstance3D.new()
	var plane := PlaneMesh.new()
	plane.size = Vector2(80, 600)
	ground.position = Vector3(0, 0, 100)
	ground.mesh = plane
	var mat := StandardMaterial3D.new()
	mat.albedo_color = Color(0.32, 0.42, 0.2)
	ground.material_override = mat
	world.add_child(ground)
	return world


func _props(world: Node3D, roles: Array, custom: Color, spacing: float) -> Array:
	var materials := []
	for k in roles.size():
		var role: String = roles[k]
		var mm := MultiMesh.new()
		mm.transform_format = MultiMesh.TRANSFORM_3D
		mm.use_custom_data = true
		mm.mesh = FolkModels.prop_mesh(role)
		mm.instance_count = 1
		mm.set_instance_transform(0, Transform3D(Basis(), Vector3(0, 0, (k - (roles.size() - 1) * 0.5) * spacing)))
		mm.set_instance_custom_data(0, custom)
		var mmi := MultiMeshInstance3D.new()
		mmi.multimesh = mm
		# Le trajet est fait dans le shader : boîte englobante élargie pour ne pas être culled.
		mmi.custom_aabb = AABB(Vector3(-8, -2, -8), Vector3(16, 12, 520))
		world.add_child(mmi)
		for s in mm.mesh.get_surface_count():
			materials.append(mm.mesh.surface_get_material(s))
	return materials


func _camera(world: Node3D, from: Vector3, to: Vector3) -> Camera3D:
	var camera := Camera3D.new()
	camera.fov = 45.0
	world.add_child(camera)
	camera.look_at_from_position(from, to)
	camera.current = true
	return camera


func _grab(name: String, frames: Array) -> void:
	await RenderingServer.frame_post_draw
	var image := root.get_viewport().get_texture().get_image()
	frames.append(image)
	if _out != "":
		DirAccess.make_dir_recursive_absolute(_out)
		image.save_png(_out.path_join("%s_%d.png" % [name, frames.size() - 1]))


func _diff(a: Image, b: Image) -> float:
	var total := 0.0
	var step := 2
	var n := 0
	for y in range(0, a.get_height(), step):
		for x in range(0, a.get_width(), step):
			var ca := a.get_pixel(x, y)
			var cb := b.get_pixel(x, y)
			total += absf(ca.r - cb.r) + absf(ca.g - cb.g) + absf(ca.b - cb.b)
			n += 1
	return total / float(n)


func _report(name: String, frames: Array) -> void:
	var parts := PackedStringArray()
	for i in range(1, frames.size()):
		parts.append("%.5f" % _diff(frames[0], frames[i]))
	print("AS1 SHOT %s diff vs frame 0: %s" % [name, " ".join(parts)])


func _run() -> void:
	await process_frame
	print("AS1 SHOT as1=%s" % AnimalMotion.enabled())
	# Bêtes à l'arrêt (broutage, queue, souffle), vues de côté, temps de campagne simulé.
	var world := _scene()
	var materials := _props(world, BEASTS, Color(0, 0, 0, 0), 4.0)
	var _c := _camera(world, Vector3(11, 1.6, 0), Vector3(0, 0.8, 0))
	var frames := []
	for i in FRAMES:
		for m in materials:
			(m as ShaderMaterial).set_shader_parameter("anim_time", 3.1 * i)
		await process_frame
		await _grab("idle", frames)
	_report("idle", frames)
	world.queue_free()
	await process_frame
	# Au pas : trajet de 400 m à 1,2 m/s, images à 0,3 s d'écart (un pas = ~1,3 s).
	world = _scene()
	materials = _props(world, BEASTS, Color(400, 1.2, 100, 0), 4.0)
	var cam := _camera(world, Vector3(11, 1.6, 0), Vector3(0, 0.8, 0))
	frames = []
	for i in FRAMES:
		cam.position.z = 100.0 + 1.2 * 0.35 * i
		for m in materials:
			(m as ShaderMaterial).set_shader_parameter("anim_time", 0.35 * i)
		await process_frame
		await _grab("walk", frames)
	_report("walk", frames)
	world.queue_free()
	await process_frame
	# Charrettes attelées au pas.
	world = _scene()
	materials = _props(world, CARTS, Color(400, 1.2, 100, 0), 9.0)
	cam = _camera(world, Vector3(16, 3.0, 0), Vector3(0, 1.0, 0))
	frames = []
	for i in FRAMES:
		cam.position.z = 100.0 + 1.2 * 0.35 * i
		for m in materials:
			(m as ShaderMaterial).set_shader_parameter("anim_time", 0.35 * i)
		await process_frame
		await _grab("cart", frames)
	_report("cart", frames)
	world.queue_free()
	await process_frame
	# Chevaux du camp (heure réelle).
	world = _scene()
	var transforms := {"horse_0": [], "horse_1": [], "horse_2": []}
	var k := 0
	for name in transforms:
		for j in 2:
			(transforms[name] as Array).append(Transform3D(Basis(), Vector3((k - 2.5) * 3.2, 0, 0)))
			k += 1
	AnimalMotion.build_camp_horses(world, transforms, 0.0)
	_camera(world, Vector3(0, 1.3, 10), Vector3(0, 0.8, 0))
	frames = []
	for i in FRAMES:
		await create_timer(2.5).timeout
		await _grab("camp", frames)
	_report("camp", frames)
	quit()
