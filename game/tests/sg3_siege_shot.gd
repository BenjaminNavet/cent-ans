extends SceneTree

## SG3 : captures des servants d'engins (treuil et chargement du trébuchet, écouvillon et charge de
## la bombarde, poussée du bélier et du beffroi), de la bombarde au tir (éclair, fumée, recul) et
## des engins au loin (maillage simplifié). La bataille avance vite jusqu'au moment voulu, puis la
## simulation est mise en pause pour la capture (les effets continuent en temps réel).
## Usage (avec affichage) :
##   godot --path game --resolution 1600x900 --script res://tests/sg3_siege_shot.gd -- \
##     --out=<dossier> [--landmark=avignon --attacker=fac_england] [--prefix=sg3] \
##     [--only=bombard,treb,push,lod] [--flipbooks=<dossier>]
## Lot FA2 : `--flipbooks` remplace les planches de feu et de fumée (A/B, `fx_flipbook_override.gd`).

const FLIPBOOKS := preload("res://tests/fx_flipbook_override.gd")

var _out := ""
var _prefix := "sg3"
var _landmark := ""
var _attacker := "fac_france"
var _engines := "unit_trebuchet,unit_bombard,unit_mangonel,unit_siege_tower"
var _only: PackedStringArray = []
var _textures := {}
var scene: Node
var battle: Object
var engines: SiegeEnginesFx


func _init() -> void:
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			_out = arg.trim_prefix("--out=")
		elif arg.begins_with("--prefix="):
			_prefix = arg.trim_prefix("--prefix=")
		elif arg.begins_with("--landmark="):
			_landmark = arg.trim_prefix("--landmark=")
		elif arg.begins_with("--attacker="):
			_attacker = arg.trim_prefix("--attacker=")
		elif arg.begins_with("--engines="):
			_engines = arg.trim_prefix("--engines=")
		elif arg.begins_with("--only="):
			_only = arg.trim_prefix("--only=").split(",", false)
		elif arg.begins_with("--flipbooks="):
			_textures = FLIPBOOKS.load_textures(arg.trim_prefix("--flipbooks="))
	if _out == "":
		push_error("sg3_siege_shot: --out=<dossier> required")
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
	var index := -1
	if _landmark != "":
		index = sim.call("debug_stage_landmark_siege", armies[1] if _attacker == "fac_england" else armies[0], _landmark)
	else:
		index = sim.call("debug_stage_siege", armies[0], "prov_guyenne")
	if index < 0:
		push_error("sg3_siege_shot: cannot stage the siege")
		quit(1)
		return
	BattleScene.demo_args = PackedStringArray(["--siege-engines=" + _engines, "--no-speech"])
	scene = (load("res://scenes/battle/battle.tscn") as PackedScene).instantiate()
	scene.configure(sim, index, 7)
	root.add_child(scene)
	for _i in 30:
		await process_frame
	battle = scene.battle
	if scene.deployment != null and scene.deployment.active:
		scene.deployment.finish()
	if battle.call("is_deploying"):
		battle.call("start_battle")
	battle.call("set_ai", "attacker", true)
	battle.call("set_ai", "defender", true)
	scene.camera_rig.edge_pan_enabled = false
	engines = scene.engines_fx
	if engines == null:
		push_error("sg3_siege_shot: no engine fx")
		quit(1)
		return
	print("sg3_siege_shot: crew %s" % ("on" if engines.crew != null else "off"))
	if _wants("push"):
		await _pushers("tower", "beffroi_pousse")
		await _pushers("ram", "belier_pousse")
	if _wants("lod"):
		await _lod_view()
	if _wants("bombard"):
		await _bombard_fire()
	if _wants("treb"):
		await _trebuchet_crew()
	print("sg3_siege_shot: done at %.0f s, %d servants drawn" % [float(battle.call("get_elapsed")), engines.crew.shown if engines.crew != null else 0])
	quit(0)


func _wants(key: String) -> bool:
	return _only.is_empty() or _only.has(key)


func _unit_of(type: String) -> Dictionary:
	for unit in battle.call("get_units"):
		if str(unit["type"]) == type and bool(unit["present"]):
			return unit
	return {}


func _unit_by_render(render: String) -> Dictionary:
	for unit in battle.call("get_units"):
		if str(unit.get("render", "")) == render and bool(unit["present"]):
			return unit
	return {}


func _step(seconds: float) -> void:
	scene._fast_forward(float(battle.call("get_elapsed")) + seconds)
	# Une image réelle : les nœuds animés (engins, servants) suivent la simulation.
	await process_frame


## Bombarde : attend un tir du cœur, puis trois images (éclair, fumée et recul, fumée qui monte),
## ensuite l'écouvillon et la charge pendant le rechargement.
func _bombard_fire() -> void:
	var limit := float(battle.call("get_elapsed")) + 300.0
	var last_swing := -1.0
	while float(battle.call("get_elapsed")) < limit and not battle.call("is_finished"):
		await _step(0.1)
		var unit := _unit_of("unit_bombard")
		if unit.is_empty():
			continue
		var entry: Dictionary = engines._engines.get(int(unit["id"]), {})
		if entry.is_empty() or int(entry.get("shown", 0)) == 0:
			continue
		if last_swing < 0.0:
			last_swing = float(entry["swing"])
			continue
		if float(entry["swing"]) == last_swing:
			continue
		# Tir : caméra de trois quarts avant sur la bouche.
		var node: Node3D = entry["figures"][0]
		var facing := float(unit["facing"])
		var focus := node.global_position + Vector3(sin(facing), 0.0, cos(facing)) * 3.0
		focus.y = 0.0
		scene.paused = true
		scene.camera_rig.look_at_point(focus, 14.0, facing + PI * 0.5 + 0.9)
		await _frames(1)
		await _shot("bombarde_tir_eclair.png")
		await _wait(0.12)
		await _shot("bombarde_tir_recul.png")
		await _wait(0.9)
		await _shot("bombarde_tir_fumee.png")
		scene.paused = false
		print("sg3_siege_shot: bombard shot at %.1f s, reload %.1f / %.1f" % [float(battle.call("get_elapsed")), float(unit.get("reload", 0.0)), float(unit.get("reload_period", 0.0))])
		# Écouvillon (première moitié du rechargement), puis charge.
		await _step(2.5)
		scene.paused = true
		scene.camera_rig.look_at_point(focus, 11.0, facing + PI * 0.5 + 0.6)
		await _frames(4)
		await _shot("bombarde_ecouvillon.png")
		scene.paused = false
		await _step(4.0)
		scene.paused = true
		await _frames(4)
		await _shot("bombarde_charge.png")
		scene.paused = false
		return
	print("sg3_siege_shot: no bombard shot caught")


## Trébuchet : servants au treuil (verge qui remonte), puis chargeur (pierre dans la fronde).
func _trebuchet_crew() -> void:
	var limit := float(battle.call("get_elapsed")) + 240.0
	var wanted := ["treuil", "chargement"]
	while not wanted.is_empty() and float(battle.call("get_elapsed")) < limit and not battle.call("is_finished"):
		await _step(0.2)
		var unit := _unit_of("unit_trebuchet")
		if unit.is_empty():
			continue
		var entry: Dictionary = engines._engines.get(int(unit["id"]), {})
		if entry.is_empty() or int(entry.get("shown", 0)) == 0 or not bool(entry["fired"]):
			continue
		var c := engines._kind_cfg("trebuchet")
		var wound := SiegeEnginesFx._wound(c, unit, true)
		var tau := engines.time_now - float(entry["swing"])
		if tau < float(c["swing_s"]) + float(c["settle_s"]) + 0.3:
			continue
		var node: Node3D = entry["figures"][0]
		var facing := float(unit["facing"])
		var back := node.global_position - Vector3(sin(facing), 0.0, cos(facing)) * 4.5
		back.y = 0.0
		var name := ""
		if wanted.has("treuil") and wound > 0.25 and wound < 0.5:
			name = "treuil"
		elif wanted.has("chargement") and wound > 0.7 and wound < 0.9:
			name = "chargement"
		if name == "":
			continue
		wanted.erase(name)
		scene.paused = true
		scene.camera_rig.look_at_point(back, 16.0, facing + PI * 0.5 + 0.5)
		await _frames(4)
		await _shot("trebuchet_%s.png" % name)
		scene.paused = false
	if not wanted.is_empty():
		print("sg3_siege_shot: missing trebuchet ", wanted)


## Bélier ou beffroi en marche : servants qui poussent, vus de trois quarts arrière.
func _pushers(render: String, name: String) -> void:
	var limit := float(battle.call("get_elapsed")) + 200.0
	while float(battle.call("get_elapsed")) < limit and not battle.call("is_finished"):
		await _step(0.2)
		var unit := _unit_by_render(render)
		if unit.is_empty():
			continue
		var mover: Dictionary = engines._movers.get(int(unit["id"]), {})
		if mover.is_empty() or engines.time_now - float(mover.get("moved_at", -1000.0)) > 0.3:
			continue
		var facing := float(unit["facing"])
		var focus := Vector3(float(unit["x"]), 0.0, float(unit["z"])) - Vector3(sin(facing), 0.0, cos(facing)) * 4.0
		scene.paused = true
		scene.camera_rig.look_at_point(focus, 16.0 if render == "ram" else 24.0, facing + PI + 0.7)
		await _frames(4)
		await _shot(name + ".png")
		scene.paused = false
		return
	print("sg3_siege_shot: no moving %s caught" % render)


## Engins au loin : caméra à ~220 m, maillages simplifiés.
func _lod_view() -> void:
	var unit := _unit_of("unit_trebuchet")
	if unit.is_empty():
		print("sg3_siege_shot: no trebuchet for the LOD view")
		return
	var facing := float(unit["facing"])
	var focus := Vector3(float(unit["x"]), 0.0, float(unit["z"]))
	scene.paused = true
	scene.camera_rig.look_at_point(focus, 220.0, facing + PI * 0.5 + 0.5)
	await _frames(6)
	var entry: Dictionary = engines._engines.get(int(unit["id"]), {})
	var lods := 0
	for lod in entry.get("lods", []):
		if lod != null and (lod as Node3D).visible:
			lods += 1
	print("sg3_siege_shot: %d simplified trebuchet figures shown at 220 m" % lods)
	await _shot("engins_lointains.png")
	scene.paused = false


func _frames(n: int) -> void:
	for _i in n:
		await process_frame


func _wait(seconds: float) -> void:
	var end := Time.get_ticks_msec() + int(seconds * 1000.0)
	while Time.get_ticks_msec() < end:
		FLIPBOOKS.apply(scene, _textures)
		await process_frame


func _shot(file: String) -> void:
	FLIPBOOKS.apply(scene, _textures)
	await RenderingServer.frame_post_draw
	var img := root.get_viewport().get_texture().get_image()
	var path := _out.path_join("%s_%s" % [_prefix, file])
	img.save_png(path)
	print("sg3_siege_shot: ", path)
