extends SceneTree

## Lot AS5 : planche de contrôle pour la session principale (écrit user://as5_shot.png, non
## commitée) et compilation réelle des trois shaders (fenêtre requise : sans `--headless`).
## Foyers de carte (flammes + fumées), bannière de maquette ondulante, imposteur d'arbre.
## Usage : godot --path game --script res://tests/as5_shot.gd [-- --no-as5]

func _init() -> void:
	var env := WorldEnvironment.new()
	env.environment = Environment.new()
	env.environment.background_mode = Environment.BG_COLOR
	env.environment.background_color = Color(0.35, 0.42, 0.3)
	root.add_child(env)
	var sun := DirectionalLight3D.new()
	sun.rotation_degrees = Vector3(-50, 30, 0)
	root.add_child(sun)
	var effects := LifeEffects.new()
	root.add_child(effects)
	effects.setup(null, null)
	var points: Array = effects.get("_fire_points")
	for k in 5:
		points.append(effects.call("_fixed_point", Vector2(k * 5.0, 0.0), 0.1, float(k) / 5.0))
	effects.call("_fill", effects.get("_fires"), points, LifeEffects.FIRE_SIZE, 1.0)
	effects.call("_fill_flames")
	var banner_quad := MeshInstance3D.new()
	var plane := PlaneMesh.new()
	plane.size = Vector2(3.0, 2.0)
	plane.orientation = PlaneMesh.FACE_Z
	banner_quad.mesh = plane
	var banner := ShaderMaterial.new()
	banner.shader = load("res://shaders/maquette_banner.gdshader")
	var wind := MapFireWind.section("maquette_banner")
	banner.set_shader_parameter("amplitude", float(wind.get("amplitude", 0.0)))
	banner.set_shader_parameter("banner_size", 3.0)
	banner_quad.material_override = banner
	banner_quad.position = Vector3(-8.0, 3.0, 0.0)
	root.add_child(banner_quad)
	var trees := BattleTrees.new()
	root.add_child(trees)
	trees.add_impostor_tile("as5", [Transform3D(Basis.IDENTITY, Vector3(8.0, 0.0, 0.0))], [Color.WHITE], [0])
	var camera := Camera3D.new()
	root.add_child(camera)
	camera.look_at_from_position(Vector3(5.0, 6.0, 30.0), Vector3(5.0, 2.0, 0.0))
	camera.make_current()
	effects.update_view(10.0, null)
	for i in 90:
		await process_frame
	var image := root.get_texture().get_image()
	image.save_png("user://as5_shot.png")
	print("as5_shot: ", ProjectSettings.globalize_path("user://as5_shot.png"))
	quit(0)
