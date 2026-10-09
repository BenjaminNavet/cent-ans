extends SceneTree

## TX T3 : les cartes de plantes de `GroundCards` rendues par `ground_clutter.gdshader`, cinq par biome
## (désert, steppe, maquis égéen), en rangées sur une prairie. Usage :
##   tools/godot_bg.sh --path game --resolution 1280x720 \
##     --script res://tests/tx_ground_cards_shot.gd -- --out=<fichier.png>

const BIOMES := [8, 4, 14]


func _init() -> void:
	var out := CmdArgs.value("--out", "user://tx_ground_cards.png")
	await process_frame
	var env := Environment.new()
	env.background_mode = Environment.BG_COLOR
	env.background_color = Color(0.6, 0.7, 0.82)
	env.ambient_light_source = Environment.AMBIENT_SOURCE_COLOR
	env.ambient_light_color = Color(0.7, 0.72, 0.75)
	var we := WorldEnvironment.new()
	we.environment = env
	root.add_child(we)
	var sun := DirectionalLight3D.new()
	sun.rotation_degrees = Vector3(-50, 30, 0)
	root.add_child(sun)
	var ground := MeshInstance3D.new()
	var plane := PlaneMesh.new()
	plane.size = Vector2(40, 20)
	ground.mesh = plane
	var gm := StandardMaterial3D.new()
	gm.albedo_color = Color(0.36, 0.34, 0.22)
	ground.material_override = gm
	root.add_child(ground)
	var material := ShaderMaterial.new()
	material.shader = preload("res://shaders/ground_clutter.gdshader")
	print("tx_ground_cards_shot: ready=", GroundCards.ready())
	material.set_shader_parameter("cards", GroundCards.albedo())
	material.set_shader_parameter("has_cards", true)
	material.set_shader_parameter("radius", 1000.0)
	material.set_shader_parameter("fade", 1.0)
	material.set_shader_parameter("wind_strength", 0.0)
	var multimesh := MultiMesh.new()
	multimesh.transform_format = MultiMesh.TRANSFORM_3D
	multimesh.use_custom_data = true
	multimesh.mesh = GroundClutter.crossed_cards_mesh()
	multimesh.instance_count = BIOMES.size() * 5
	var i := 0
	for row in BIOMES.size():
		var layers := GroundCards.layers_for_biome(BIOMES[row])
		print("tx_ground_cards_shot: biome ", BIOMES[row], " layers ", layers)
		for k in 5:
			var layer: int = layers[k % maxi(layers.size(), 1)]
			var size := GroundCards.size_m(layer)
			multimesh.set_instance_transform(i, Transform3D(Basis().scaled(Vector3.ONE * size), Vector3((k - 2) * 2.6, 0.0, (row - 1) * 3.4)))
			multimesh.set_instance_custom_data(i, Color(2.0 * float(layer + 1), 0.5, 0.3, 0.0))
			i += 1
	var instance := MultiMeshInstance3D.new()
	instance.multimesh = multimesh
	instance.material_override = material
	instance.custom_aabb = AABB(Vector3(-30, -5, -30), Vector3(60, 20, 60))
	root.add_child(instance)
	var camera := Camera3D.new()
	root.add_child(camera)
	camera.position = Vector3(0, 3.2, 9.0)
	camera.look_at(Vector3(0, 0.6, 0))
	for _i in 30:
		await process_frame
	await RenderingServer.frame_post_draw
	root.get_texture().get_image().save_png(out)
	print("tx_ground_cards_shot: ", out)
	quit(0)
