extends SceneTree

## TX T3 : arbres procéduraux de bataille (chêne, hêtre, peuplier, saule) avec écorces et feuilles
## générées, vus de près. Usage : tools/godot_bg.sh --path game --resolution 1280x720 \
##   --script res://tests/tx_tree_bark_shot.gd -- --out=<fichier.png> [--legacy-textures]

const SPECIES := ["oak", "beech", "poplar", "willow"]


func _init() -> void:
	var out := CmdArgs.value("--out", "user://tx_tree_bark.png")
	await process_frame
	var env := Environment.new()
	env.background_mode = Environment.BG_COLOR
	env.background_color = Color(0.6, 0.7, 0.82)
	env.ambient_light_source = Environment.AMBIENT_SOURCE_COLOR
	env.ambient_light_color = Color(0.65, 0.67, 0.7)
	var we := WorldEnvironment.new()
	we.environment = env
	root.add_child(we)
	var sun := DirectionalLight3D.new()
	sun.rotation_degrees = Vector3(-40, 40, 0)
	root.add_child(sun)
	var ground := MeshInstance3D.new()
	var plane := PlaneMesh.new()
	plane.size = Vector2(120, 60)
	ground.mesh = plane
	var gm := StandardMaterial3D.new()
	gm.albedo_color = Color(0.3, 0.36, 0.2)
	ground.material_override = gm
	root.add_child(ground)
	for i in SPECIES.size():
		var mesh_instance := MeshInstance3D.new()
		mesh_instance.mesh = BattleTrees.mesh(SPECIES[i], 0, false, 0.0, 400.0)
		mesh_instance.position = Vector3((i - 1.5) * 9.0, 0.0, 0.0)
		root.add_child(mesh_instance)
	print("tx_tree_bark_shot: oak bark layer=", TreeTextures.bark_layer("oak"))
	var camera := Camera3D.new()
	root.add_child(camera)
	camera.position = Vector3(0, 4.0, 22.0)
	camera.look_at(Vector3(0, 6.0, 0))
	for _i in 40:
		await process_frame
	await RenderingServer.frame_post_draw
	root.get_texture().get_image().save_png(out)
	print("tx_tree_bark_shot: ", out)
	quit(0)
