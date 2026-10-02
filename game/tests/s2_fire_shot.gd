extends SceneTree

## Lot S2 : captures d'un siège en feu sur l'assaut de la Guyenne (`debug_stage_siege`). Quatre
## maisons voisines sont allumées (`BattleSim.debug_ignite`), la simulation avance pour laisser le
## feu se propager, puis trois vues sans interface : gros plan sur le foyer, vue large avec la
## muraille, ruines plus tard.
## Lot FA2 : `--flipbooks=<dossier>` remplace les planches de flammes et de fumée par celles du
## dossier (A/B ancien / nouveau dans la même scène, voir `fx_flipbook_override.gd`).
## Usage (avec affichage, pas en headless) :
##   godot --path game --resolution 1600x900 --script res://tests/s2_fire_shot.gd -- --out=<dossier>
##     [--flipbooks=<dossier>] [--prefix=s2] [--only=proche,large,ruines] [--close=45] [--wide=170]

const FLIPBOOKS := preload("res://tests/fx_flipbook_override.gd")

var _out := ""
var _prefix := "s2"
var _only := PackedStringArray()
var _close := 45.0
var _wide := 170.0
var _textures := {}
var _scene: Node = null


func _init() -> void:
	var flipbooks := ""
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			_out = arg.trim_prefix("--out=")
		elif arg.begins_with("--flipbooks="):
			flipbooks = arg.trim_prefix("--flipbooks=")
		elif arg.begins_with("--prefix="):
			_prefix = arg.trim_prefix("--prefix=")
		elif arg.begins_with("--only="):
			_only = arg.trim_prefix("--only=").split(",", false)
		elif arg.begins_with("--close="):
			_close = float(arg.trim_prefix("--close="))
		elif arg.begins_with("--wide="):
			_wide = float(arg.trim_prefix("--wide="))
	if _out == "":
		push_error("s2_fire_shot: --out=<dossier> required")
		quit(1)
		return
	_textures = FLIPBOOKS.load_textures(flipbooks)
	if flipbooks != "" and _textures.size() < 2:
		quit(1)
		return
	await process_frame
	var data_dir := ProjectSettings.globalize_path("res://").path_join("../data").simplify_path()
	var sim: Object = ClassDB.instantiate("CampaignSim")
	if not sim.call("new_campaign", data_dir, "fac_france", 1337):
		quit(1)
		return
	var armies: Array = BattleScene.main_armies(sim, "fac_france", "fac_england")
	var index: int = sim.call("debug_stage_siege", armies[0], "prov_guyenne")
	if index < 0:
		push_error("s2_fire_shot: cannot stage the siege")
		quit(1)
		return
	BattleScene.demo_args = PackedStringArray(["--no-speech"])
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
	battle.call("set_ai", "attacker", false)
	battle.call("set_ai", "defender", false)
	_scene.camera_rig.edge_pan_enabled = false
	var siege: Dictionary = battle.call("get_siege")
	var houses: Array = siege["houses"]
	var center: Vector2 = siege.get("center", Vector2(600, 560))
	# Le foyer : la maison intra-muros la plus proche du centre et ses trois voisines.
	var order := range(houses.size())
	order = order.filter(func(i: int) -> bool: return not bool(houses[i]["suburb"]))
	order.sort_custom(func(i: int, j: int) -> bool: return _pos(houses[i]).distance_to(center) < _pos(houses[j]).distance_to(center))
	var seed_pos := _pos(houses[order[0]])
	order.sort_custom(func(i: int, j: int) -> bool: return _pos(houses[i]).distance_to(seed_pos) < _pos(houses[j]).distance_to(seed_pos))
	var hearth := Vector2.ZERO
	for k in 4:
		battle.call("debug_ignite", order[k])
		hearth += _pos(houses[order[k]]) * 0.25
	# La scène tourne (non figée) : le rendu du feu suit l'état du cœur image après image.
	await _advance(40.0)
	var ground: float = _scene.terrain.height_at(hearth.x, hearth.y)
	# Visée à hauteur des toits : les flammes au milieu de l'image, le panache au-dessus.
	var focus := Vector3(hearth.x, ground + 8.0, hearth.y)
	siege = battle.call("get_siege")
	var wind: Vector2 = siege.get("wind", Vector2.ZERO)
	# Vent de travers : le panache part sur le côté de l'image, pas vers la caméra.
	var yaw := atan2(wind.x, wind.y) + PI * 0.5 if wind.length() > 0.0001 else 0.6 + PI
	print("s2_fire_shot: %d burning (%d drawn), wind %s, status « %s »" % [int(siege.get("houses_burning", 0)), _drawn_fires(), wind, BattleScene.siege_status(siege)])
	if _wants("proche"):
		_scene.camera_rig.look_at_point(focus, _close, yaw)
		await _wait(3.0)
		await _shot("feu-proche")
	if _wants("large"):
		_scene.camera_rig.look_at_point(Vector3(center.x, ground, center.y), _wide, yaw)
		await _wait(2.0)
		await _shot("feu-large")
	if _wants("ruines"):
		await _advance(150.0)
		_scene.camera_rig.look_at_point(focus, _close * 1.5, yaw)
		await _wait(3.0)
		siege = battle.call("get_siege")
		print("s2_fire_shot: later %d burning, %d burnt" % [int(siege.get("houses_burning", 0)), int(siege.get("houses_burnt", 0))])
		await _shot("ruines")
	quit(0)


func _wants(key: String) -> bool:
	return _only.is_empty() or _only.has(key)


func _pos(house: Dictionary) -> Vector2:
	return Vector2(float(house["x"]), float(house["z"]))


func _drawn_fires() -> int:
	var siege_view: Node = _scene.get("siege_view")
	if siege_view == null or siege_view.get("fire_fx") == null:
		return -1
	return int(siege_view.fire_fx.burning_count)


## Avance la simulation de `seconds` puis laisse quelques images au rendu pour suivre.
func _advance(seconds: float) -> void:
	_scene.paused = false
	_scene._fast_forward(float(_scene.battle.call("get_elapsed")) + seconds)
	for _i in 4:
		await process_frame
	_scene.paused = true


func _wait(seconds: float) -> void:
	var end := Time.get_ticks_msec() + int(seconds * 1000.0)
	while Time.get_ticks_msec() < end:
		FLIPBOOKS.apply(_scene, _textures)
		await process_frame


## Interface cachée : toutes les couches 2D de la bataille (HUD, ordres du chef, alertes).
func _hide_ui(node: Node) -> void:
	if node is CanvasLayer:
		(node as CanvasLayer).visible = false
	for child in node.get_children():
		_hide_ui(child)


func _shot(view: String) -> void:
	FLIPBOOKS.apply(_scene, _textures)
	_hide_ui(_scene)
	await process_frame
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	var path := _out.path_join("%s-%s.png" % [_prefix, view])
	DirAccess.make_dir_recursive_absolute(_out)
	print("s2_fire_shot: %s (%s)" % [path, error_string(image.save_png(path))])
