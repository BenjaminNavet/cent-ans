extends SceneTree

## Lot FA2 : fumée d'un coup de bombarde, pour l'A/B des planches de fumée. Assaut de la Guyenne
## avec une bombarde ; la bataille avance jusqu'au premier tir, la simulation est mise en pause
## et la fumée (qui vit en temps réel) est prise à des âges fixes après le coup : toujours les
## mêmes instants d'une exécution à l'autre, graines des particules fixées, caméra fixe.
## Usage (avec affichage) :
##   godot --path game --resolution 1600x900 --script res://tests/fa2_bombard_shot.gd -- \
##     --out=<dossier> [--prefix=fa2] [--view=close|medium] [--ages=0.5,2.5,4.5]
##     [--flipbooks=<dossier>] [--hide=flash,burst,linger] [--trail] [--probe]
## `--hide` retire un des trois effets du coup (éclair, bouffée de `BattleEffects`, nuage qui
## s'attarde de `BattleSmoke`) pour savoir lequel dessine quoi ; `--probe` liste en texte ce qui
## est dessiné à moins de 8 m de la bouche (nœud, matériau, planche).
## La traînée du boulet (`SiegeAssaultFx`, disque flou sans rapport avec les planches) est cachée
## sauf `--trail` : simulation en pause, le boulet reste à la bouche et sa traînée s'y empile en
## un grand disque pâle qui n'existe pas en jeu.

const FLIPBOOKS := preload("res://tests/fx_flipbook_override.gd")

var _out := ""
var _prefix := "fa2"
var _view := "close"
var _ages := PackedFloat32Array([0.5, 2.5, 4.5])
var _hide := PackedStringArray()
var _textures := {}
var _scene: Node = null
var _camera: Camera3D = null
var _muzzle := Vector3.INF
var _probe := false
var _trail := false
## Âge de la fumée : somme des pas d'image depuis le coup (l'horloge des particules), et non
## l'heure murale, qu'une image longue (compilation de shader) décale d'une exécution à l'autre.
var _clock := 0.0


func _init() -> void:
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			_out = arg.trim_prefix("--out=")
		elif arg.begins_with("--prefix="):
			_prefix = arg.trim_prefix("--prefix=")
		elif arg.begins_with("--view="):
			_view = arg.trim_prefix("--view=")
		elif arg.begins_with("--ages="):
			_ages = PackedFloat32Array()
			for age in arg.trim_prefix("--ages=").split(",", false):
				_ages.append(float(age))
		elif arg.begins_with("--hide="):
			_hide = arg.trim_prefix("--hide=").split(",", false)
		elif arg == "--trail":
			_trail = true
		elif arg == "--probe":
			_probe = true
		elif arg.begins_with("--flipbooks="):
			_textures = FLIPBOOKS.load_textures(arg.trim_prefix("--flipbooks="))
	if _out == "":
		push_error("fa2_bombard_shot: --out=<dossier> required")
		quit(1)
		return
	DirAccess.make_dir_recursive_absolute(_out)
	await process_frame
	var data_dir := ProjectSettings.globalize_path("res://").path_join("../data").simplify_path()
	var sim: Object = ClassDB.instantiate("CampaignSim")
	if not sim.call("new_campaign", data_dir, "fac_france", 1337):
		quit(1)
		return
	var armies: Array = BattleScene.main_armies(sim, "fac_france", "fac_england")
	var index: int = sim.call("debug_stage_siege", armies[0], "prov_guyenne")
	if index < 0:
		push_error("fa2_bombard_shot: cannot stage the siege")
		quit(1)
		return
	BattleScene.demo_args = PackedStringArray(["--siege-engines=unit_bombard", "--no-speech"])
	_scene = (load("res://scenes/battle/battle.tscn") as PackedScene).instantiate()
	_scene.configure(sim, index, 7)
	root.add_child(_scene)
	for _i in 30:
		await process_frame
	var battle: Object = _scene.battle
	if _scene.deployment != null and _scene.deployment.active:
		_scene.deployment.finish()
	if battle.call("is_deploying"):
		battle.call("start_battle")
	battle.call("set_ai", "attacker", true)
	battle.call("set_ai", "defender", true)
	_scene.camera_rig.edge_pan_enabled = false
	_fix_seeds(_scene)
	_scene.effects.cannon_fired.connect(func(muzzle: Vector3) -> void:
		if _muzzle == Vector3.INF:
			_muzzle = muzzle)
	var limit := float(battle.call("get_elapsed")) + 300.0
	while _muzzle == Vector3.INF and float(battle.call("get_elapsed")) < limit and not battle.call("is_finished"):
		_scene._fast_forward(float(battle.call("get_elapsed")) + 0.1)
	if _muzzle == Vector3.INF:
		push_error("fa2_bombard_shot: no bombard shot caught")
		quit(1)
		return
	_scene.paused = true
	_clock = 0.0
	var facing := 0.0
	for unit in battle.call("get_units"):
		if str(unit["type"]) == "unit_bombard" and bool(unit["present"]):
			facing = float(unit["facing"])
	_frame(facing)
	_hide_effects()
	print("fa2_bombard_shot: shot at %.1f s, muzzle %s, view %s" % [float(battle.call("get_elapsed")), _muzzle, _view])
	for age in _ages:
		while _clock < age:
			FLIPBOOKS.apply(_scene, _textures)
			_hide_ui(_scene)
			await process_frame
		if _probe:
			print("fa2_bombard_shot: drawn within 8 m of the muzzle at %.1f s" % age)
			_probe_near(_scene)
		await _shot("%s-%s-%04dms.png" % [_prefix, _view, int(age * 1000.0)])
	quit(0)


func _process(delta: float) -> bool:
	_clock += delta
	return false


## Mêmes tirages d'une exécution à l'autre pour tous les émetteurs (bouffées, nuages, éclairs).
func _fix_seeds(node: Node) -> void:
	if node is GPUParticles3D:
		var particles := node as GPUParticles3D
		particles.use_fixed_seed = true
		particles.seed = 4242 + hash(str(particles.get_path())) % 1000
	for child in node.get_children():
		_fix_seeds(child)


func _probe_near(node: Node) -> void:
	if node is GeometryInstance3D and (node as Node3D).is_visible_in_tree() and (node as Node3D).global_position.distance_to(_muzzle) < 8.0:
		var material: Material = (node as GeometryInstance3D).material_override
		var extra := ""
		if node is GPUParticles3D:
			var particles := node as GPUParticles3D
			if particles.draw_pass_1 != null and material == null:
				material = particles.draw_pass_1.surface_get_material(0)
			extra = " emitting=%s one_shot=%s lifetime=%.2f amount=%d" % [particles.emitting, particles.one_shot, particles.lifetime, particles.amount]
		var shader := ""
		if material is ShaderMaterial and (material as ShaderMaterial).shader != null:
			shader = (material as ShaderMaterial).shader.resource_path
		elif material is BaseMaterial3D:
			shader = "StandardMaterial3D blend=%d albedo=%s" % [(material as BaseMaterial3D).blend_mode, (material as BaseMaterial3D).albedo_color]
		print("  %s [%s] %s%s" % [_scene.get_path_to(node), node.get_class(), shader, extra])
	for child in node.get_children():
		_probe_near(child)


func _hide_effects() -> void:
	var pools: Dictionary = _scene.effects._bursts
	if _hide.has("flash"):
		for particles in pools.get("flash", []):
			(particles as Node3D).visible = false
	if _hide.has("burst"):
		for particles in pools.get("smoke", []):
			(particles as Node3D).visible = false
	if not _trail and _scene.assault_fx != null:
		_hide_particles(_scene.assault_fx)
	if _hide.has("linger") and _scene.staging != null and _scene.staging.smoke != null:
		for particles in _scene.staging.smoke._linger:
			(particles as Node3D).visible = false


func _hide_particles(node: Node) -> void:
	if node is GPUParticles3D:
		(node as Node3D).visible = false
	for child in node.get_children():
		_hide_particles(child)


## Caméra propre au script : de trois quarts avant sur la bouche (gros plan) ou de côté à
## distance de bataille (vue moyenne).
func _frame(facing: float) -> void:
	var close := _view == "close"
	var distance := 9.0 if close else 30.0
	var pitch := 0.22 if close else 0.2
	var yaw := facing + PI * 0.5 + (0.9 if close else 0.3)
	var aim := _muzzle + Vector3(sin(facing), 0.0, cos(facing)) * (2.0 if close else 5.0) + Vector3.UP * (1.0 if close else 3.0)
	_scene.camera_rig.look_at_point(_muzzle, distance, yaw)
	_camera = Camera3D.new()
	_camera.fov = _scene.camera_rig.camera.fov
	_camera.near = _scene.camera_rig.camera.near
	_camera.far = _scene.camera_rig.camera.far
	_camera.attributes = _scene.camera_rig.camera.attributes
	_scene.add_child(_camera)
	_camera.global_position = aim + Vector3(sin(yaw) * cos(pitch), sin(pitch), cos(yaw) * cos(pitch)) * distance
	_camera.look_at(aim, Vector3.UP)
	_camera.make_current()


## Interface écartée de l'image (par décalage : plusieurs panneaux rétablissent `visible`).
func _hide_ui(node: Node) -> void:
	if node is CanvasLayer:
		(node as CanvasLayer).offset = Vector2(100000.0, 0.0)
	for child in node.get_children():
		_hide_ui(child)


func _shot(file_name: String) -> void:
	FLIPBOOKS.apply(_scene, _textures)
	_hide_ui(_scene)
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	var path := _out.path_join(file_name)
	print("fa2_bombard_shot: %s (%s)" % [path, error_string(image.save_png(path))])
