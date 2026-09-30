extends SceneTree

## Lot TF : gros plan d'une cité de campagne avec son château Kenney (`SettlementGrowth`,
## maquette de niveau 3 + château), posée seule sur un sol neutre sous un soleil fixe (cadrage
## reproductible pour les captures avant/après ; la carte entière n'est pas chargée). Fenêtre réelle.
## Usage : godot --path game --resolution 1280x720 --script res://tests/tf_castle_shot.gd -- \
##     --out=<png absolu> [--yaw=35] [--dist=1.0] [--seed=0]


func _init() -> void:
	var out := ""
	var yaw := 35.0
	var dist := 1.0
	var variant_seed := 0
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			out = arg.substr(6)
		elif arg.begins_with("--yaw="):
			yaw = float(arg.substr(6))
		elif arg.begins_with("--dist="):
			dist = float(arg.substr(7))
		elif arg.begins_with("--seed="):
			variant_seed = int(arg.substr(7))
	if out == "":
		push_error("tf_castle_shot: --out=<png> required")
		quit(1)
		return
	await process_frame
	var stage := Node3D.new()
	root.add_child(stage)
	var env := Environment.new()
	env.background_mode = Environment.BG_SKY
	env.sky = Sky.new()
	env.sky.sky_material = ProceduralSkyMaterial.new()
	env.ambient_light_source = Environment.AMBIENT_SOURCE_SKY
	env.tonemap_mode = Environment.TONE_MAPPER_FILMIC
	var world_env := WorldEnvironment.new()
	world_env.environment = env
	stage.add_child(world_env)
	var sun := DirectionalLight3D.new()
	sun.rotation_degrees = Vector3(-42, 35, 0)
	sun.shadow_enabled = true
	stage.add_child(sun)
	var ground := MeshInstance3D.new()
	var plane := PlaneMesh.new()
	plane.size = Vector2(200, 200)
	ground.mesh = plane
	var ground_mat := StandardMaterial3D.new()
	ground_mat.albedo_color = Color(0.32, 0.36, 0.2)
	ground_mat.roughness = 1.0
	ground.material_override = ground_mat
	stage.add_child(ground)
	var model := SettlementGrowth.build_model(3, variant_seed, true)
	if model == null:
		push_error("tf_castle_shot: settlement models missing")
		quit(1)
		return
	stage.add_child(model)
	var castle := model.find_child("KenneyCastle", true, false) as MeshInstance3D
	var box := castle.get_aabb()
	var centre := castle.global_transform * box.get_center()
	var size := (castle.global_transform.basis * box.size).abs().length()
	var camera := Camera3D.new()
	camera.fov = 40.0
	camera.near = 0.01
	stage.add_child(camera)
	var yaw_rad := deg_to_rad(yaw)
	camera.global_position = centre + Vector3(sin(yaw_rad), 0.6, cos(yaw_rad)).normalized() * size * 2.2 * dist
	camera.look_at(centre, Vector3.UP)
	camera.current = true
	for i in 20:
		await process_frame
	await RenderingServer.frame_post_draw
	var image := root.get_viewport().get_texture().get_image()
	print("tf_castle_shot: %s %s" % [out, error_string(image.save_png(out))])
	quit(0)
