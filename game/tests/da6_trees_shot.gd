extends SceneTree

## Lot DA6 : planche des essences d'arbres de bataille (maillage complet, allégé, imposteur côte à
## côte) et atlas des imposteurs. Usage (avec affichage) :
##   godot --path game --resolution 1600x900 --script res://tests/da6_trees_shot.gd -- --out=<png> [--winter] [--cam=x,y,z,tx,ty,tz]
## Écrit aussi `<png sans extension>_atlas.png`.

const SPECIES := ["oak", "beech", "ash", "poplar", "willow", "fruit", "bush"]


func _init() -> void:
	var out := "user://da6_trees.png"
	var winter := false
	var cam_from := Vector3(0, 9, 62)
	var cam_to := Vector3(0, 8, -20)
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			out = arg.trim_prefix("--out=")
		elif arg == "--winter":
			winter = true
		elif arg.begins_with("--cam="):
			var c := arg.trim_prefix("--cam=").split_floats(",")
			if c.size() == 6:
				cam_from = Vector3(c[0], c[1], c[2])
				cam_to = Vector3(c[3], c[4], c[5])
	var world := Node3D.new()
	root.add_child(world)
	var sun := DirectionalLight3D.new()
	sun.rotation_degrees = Vector3(-38, 35, 0)
	sun.shadow_enabled = true
	sun.light_energy = 1.1
	world.add_child(sun)
	var env := WorldEnvironment.new()
	env.environment = Environment.new()
	env.environment.background_mode = Environment.BG_COLOR
	env.environment.background_color = Color(0.62, 0.72, 0.84)
	env.environment.ambient_light_source = Environment.AMBIENT_SOURCE_COLOR
	env.environment.ambient_light_color = Color(0.62, 0.68, 0.75)
	env.environment.ambient_light_energy = 0.8
	env.environment.tonemap_mode = Environment.TONE_MAPPER_FILMIC
	world.add_child(env)
	var ground := MeshInstance3D.new()
	var plane := PlaneMesh.new()
	plane.size = Vector2(400, 400)
	ground.mesh = plane
	var gmat := StandardMaterial3D.new()
	gmat.albedo_color = Color(0.36, 0.38, 0.26)
	ground.material_override = gmat
	world.add_child(ground)
	var trees := BattleTrees.new()
	world.add_child(trees)
	# Rangée 1 (devant) : maillages complets ; rangée 2 : allégés ; rangée 3 : imposteurs. Les
	# plages de distance sont ouvertes (la caméra est proche) pour tout voir.
	var imp_t: Array = []
	var imp_c: Array = []
	var imp_r: Array = []
	for i in SPECIES.size():
		var sp: String = SPECIES[i]
		var x := (float(i) - 3.0) * 17.0
		for lod in 2:
			var inst := MeshInstance3D.new()
			inst.mesh = BattleTrees.mesh(sp, lod, winter, 0.0, 100000.0)
			inst.position = Vector3(x + (8.0 if lod == 1 else 0.0), 0, -float(lod) * 34.0)
			world.add_child(inst)
		if BattleTrees.impostor_row(sp) >= 0:
			imp_t.append(Transform3D(Basis.IDENTITY, Vector3(x + 8.0, 0, 16)))
			imp_c.append(Color(1, 1, 1))
			imp_r.append(BattleTrees.impostor_row(sp))
	trees.add_impostor_tile("Imp", imp_t, imp_c, imp_r)
	trees.impostor_material.set_shader_parameter("lod_near", 0.0)
	var camera := Camera3D.new()
	world.add_child(camera)
	camera.look_at_from_position(cam_from, cam_to)
	camera.fov = 62
	await process_frame
	await trees.bake_impostors(winter)
	for _i in 8:
		await process_frame
	await RenderingServer.frame_post_draw
	root.get_viewport().get_texture().get_image().save_png(out)
	if trees.atlas_image != null:
		var atlas := trees.atlas_image.duplicate() as Image
		atlas.clear_mipmaps()
		atlas.save_png(out.get_basename() + "_atlas.png")
	quit(0)
