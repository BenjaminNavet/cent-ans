extends SceneTree

## TX T4 : une même maison du kit dans six régions (matières régionales de l'atlas `Building`), côte à
## côte sous un soleil. Usage : tools/godot_bg.sh --path game --resolution 1280x720 \
##   --script res://tests/tx_building_regions_shot.gd -- --out=<fichier.png> [--legacy-textures]

const REGIONS := ["france_nord", "angleterre_nord", "italie_centre", "maghreb", "scandinavie", "grece"]


func _init() -> void:
	var out := CmdArgs.value("--out", "user://tx_building_regions.png")
	await process_frame
	var env := Environment.new()
	env.background_mode = Environment.BG_COLOR
	env.background_color = Color(0.55, 0.65, 0.78)
	env.ambient_light_source = Environment.AMBIENT_SOURCE_COLOR
	env.ambient_light_color = Color(0.6, 0.62, 0.66)
	var we := WorldEnvironment.new()
	we.environment = env
	root.add_child(we)
	var sun := DirectionalLight3D.new()
	sun.rotation_degrees = Vector3(-45, 35, 0)
	root.add_child(sun)
	var ground := MeshInstance3D.new()
	var plane := PlaneMesh.new()
	plane.size = Vector2(120, 60)
	ground.mesh = plane
	var gm := StandardMaterial3D.new()
	gm.albedo_color = Color(0.32, 0.36, 0.24)
	ground.material_override = gm
	root.add_child(ground)
	var models := BuildingKit.models_of("townhouse")
	if models.is_empty():
		models = BuildingKit.models_of("cottage")
	var model_name := str(models[0])
	print("tx_building_regions_shot: model=", model_name, " regional=", BuildingMaterials.regional_ready())
	for i in REGIONS.size():
		BuildingMaterials.set_region(REGIONS[i])
		var mesh_instance := MeshInstance3D.new()
		mesh_instance.mesh = BuildingKit.mesh(model_name)
		mesh_instance.position = Vector3((i - 2.5) * 11.0, 0.0, 0.0)
		mesh_instance.rotation_degrees.y = -25.0
		root.add_child(mesh_instance)
	var camera := Camera3D.new()
	root.add_child(camera)
	camera.position = Vector3(0, 9.0, 34.0)
	camera.look_at(Vector3(0, 3.5, 0))
	for _i in 40:
		await process_frame
	await RenderingServer.frame_post_draw
	root.get_texture().get_image().save_png(out)
	print("tx_building_regions_shot: ", out)
	quit(0)
