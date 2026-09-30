extends SceneTree

## GA3-L5 : capture de contrôle des engins générés animés. Devant : trébuchet GA3 en plein
## basculement (verge, contrepoids et fronde posés par `SiegeEnginesFx._pose_trebuchet`) et
## bélier GA3 poutre ramenée ; derrière, à droite : les modèles procéduraux d'origine dans la
## même pose (comparaison). Écrit l'image sans la lire.
## Usage (avec affichage) :
##   godot --path game --resolution 960x540 --script res://tests/ga3_l5_siege_shot.gd -- \
##     [--out=<fichier.png>] [--phase=0.45]

var _out := ""
var _phase := 0.45


func _init() -> void:
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			_out = arg.trim_prefix("--out=")
		elif arg.begins_with("--phase="):
			_phase = float(arg.trim_prefix("--phase="))
	if _out == "":
		_out = ProjectSettings.globalize_path("res://").path_join("../docs/audit/captures/ga3/ga3_l5_siege.png").simplify_path()
	DirAccess.make_dir_recursive_absolute(_out.get_base_dir())
	await process_frame
	var stage := Node3D.new()
	root.add_child(stage)
	_setup_world(stage)
	var fx := SiegeEnginesFx.new()
	stage.add_child(fx)
	var c: Dictionary = SiegeEnginesFx.settings()["trebuchet"]
	var unit := {"reload": 0.0, "reload_period": 12.0}
	var spots := [
		[SiegeEnginesFx.instantiate("trebuchet"), Vector3(-7, 0, 0)],
		[SiegeEnginesFx.instantiate("ram"), Vector3(7, 0, 4)],
		[_procedural("trebuchet"), Vector3(-2, 0, -22)],
		[_procedural("ram"), Vector3(14, 0, -16)],
	]
	for spot in spots:
		var node: Node3D = spot[0]
		stage.add_child(node)
		node.position = spot[1]
		node.rotation.y = deg_to_rad(-90.0)  # tir vers -x : profil face à la caméra
		if node.find_child("Arm", true, false) != null:
			fx._pose_trebuchet(node, c, unit, float(c["swing_s"]) * _phase, true)
		var pivot := node.find_child("BeamPivot", true, false) as Node3D
		if pivot != null:
			pivot.rotation.x = 0.35
	print("ga3_l5_siege_shot: GA3 variants %s / %s" % [(spots[0][0] as Node).get_meta("ga3", "-"), (spots[1][0] as Node).get_meta("ga3", "-")])
	for _i in 8:
		await process_frame
	if DisplayServer.get_name() == "headless":
		print("ga3_l5_siege_shot: headless, no capture")
	else:
		await RenderingServer.frame_post_draw
		var image := root.get_texture().get_image()
		print("ga3_l5_siege_shot: %s (%s)" % [_out, error_string(image.save_png(_out))])
	quit(0)


## Modèle procédural d'origine (sans variante GA3), habillé comme en bataille.
func _procedural(model: String) -> Node3D:
	var node := (load("res://assets/models/siege/%s.glb" % model) as PackedScene).instantiate() as Node3D
	SiegeEnginesFx._dress(node)
	return node


func _setup_world(stage: Node3D) -> void:
	var env := Environment.new()
	env.background_mode = Environment.BG_COLOR
	env.background_color = Color(0.62, 0.7, 0.78)
	env.ambient_light_source = Environment.AMBIENT_SOURCE_COLOR
	env.ambient_light_color = Color(0.55, 0.58, 0.62)
	env.ambient_light_energy = 0.7
	env.tonemap_mode = Environment.TONE_MAPPER_FILMIC
	var world := WorldEnvironment.new()
	world.environment = env
	stage.add_child(world)
	var sun := DirectionalLight3D.new()
	sun.rotation_degrees = Vector3(-42, -35, 0)
	sun.light_energy = 1.3
	sun.shadow_enabled = true
	stage.add_child(sun)
	var ground := MeshInstance3D.new()
	var plane := PlaneMesh.new()
	plane.size = Vector2(120, 120)
	ground.mesh = plane
	var mat := StandardMaterial3D.new()
	mat.albedo_color = Color(0.36, 0.4, 0.26)
	mat.roughness = 1.0
	ground.material_override = mat
	stage.add_child(ground)
	var camera := Camera3D.new()
	stage.add_child(camera)
	camera.fov = 50.0
	camera.position = Vector3(2, 9, 26)
	camera.look_at(Vector3(1, 4, -4))
	camera.current = true
