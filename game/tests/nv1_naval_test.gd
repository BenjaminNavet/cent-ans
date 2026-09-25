extends SceneTree

## Lot NV1 : batailles navales. Vérifie (sans fenêtre) :
## 1. la scène autonome `--naval-scenario=sluys` se construit (26 navires, vues, équipages),
##    joue jusqu'au bout avec l'IA des deux camps et affiche l'écran de fin (victoire anglaise,
##    navires français pris) ;
## 2. depuis la campagne : une interception mise en scène (`debug_stage_naval`) ouvre l'écran
##    d'avant-bataille navale, la résolution automatique rend des événements et vide l'attente.
## Avec une fenêtre et `-- --shot=<png>` : capture de l'écran d'avant-bataille navale.
##   godot --headless --path game --script res://tests/nv1_naval_test.gd

var _failures: int = 0


func _init() -> void:
	await process_frame
	await _run_scene()
	await _run_campaign()
	print("NV1 naval test: %s" % ("OK" if _failures == 0 else "%d failure(s)" % _failures))
	quit(1 if _failures > 0 else 0)


func _check(condition: bool, message: String) -> void:
	if not condition:
		_failures += 1
		push_error("NV1: " + message)
	else:
		print("  ok  " + message)


func _data_dir() -> String:
	return ProjectSettings.globalize_path("res://").path_join("../data").simplify_path()


func _run_scene() -> void:
	var scene: NavalScene = load("res://scenes/naval/naval_battle.tscn").instantiate()
	scene.scenario_id = "sluys"
	scene.autoplay = true
	root.add_child(scene)
	await process_frame
	_check(scene.battle != null, "NavalBattleSim created")
	_check(scene.ships.size() == 26, "Sluys: 26 ships (%d)" % scene.ships.size())
	_check(scene.views.size() == scene.ships.size(), "one view per ship")
	var figures := 0
	for view in scene.views.values():
		for node in (view as Node).find_children("Crew_*", "MultiMeshInstance3D", true, false):
			figures += (node as MultiMeshInstance3D).multimesh.visible_instance_count
	_check(figures > 1500, "crews drawn on the decks (%d figures)" % figures)
	_check(scene.hud.get_node("Root/TopBanner") != null, "naval HUD banner")
	scene._fast_forward(900.0)
	await process_frame
	_check(bool(scene.battle.call("is_finished")), "battle finished within 15 min")
	_check(scene.hud.result_shown(), "result screen shown")
	var outcome: Dictionary = scene.battle.call("get_outcome")
	_check(str(outcome.get("winner", "")) == "attacker", "English victory at Sluys")
	var prizes: Array = (outcome.get("attacker", {}) as Dictionary).get("prizes", [])
	_check(prizes.size() >= 8, "French ships taken (%d prizes)" % prizes.size())
	_check(scene.volleys.launched > 1000, "volleys drawn (%d arrows)" % scene.volleys.launched)
	scene.queue_free()
	await process_frame


func _run_campaign() -> void:
	if not ClassDB.class_exists("CampaignSim"):
		_check(false, "CampaignSim class")
		return
	var sim: Object = ClassDB.instantiate("CampaignSim")
	_check(bool(sim.call("new_campaign", _data_dir(), "fac_england", 1337)), "campaign as England")
	var armies := BattleScene.main_armies(sim, "fac_england", "fac_france")
	_check(not armies.is_empty(), "main English army")
	if armies.is_empty():
		return
	var index: int = sim.call("debug_stage_naval", armies[0], "set_calais", "fac_france")
	_check(index >= 0, "French squadron intercepts the crossing to Calais")
	if index < 0:
		return
	var pending: Array = sim.call("get_pending_naval_battles")
	_check(pending.size() == 1, "one pending naval battle")
	var setup: Dictionary = sim.call("get_naval_battle_setup", index)
	_check(not setup.is_empty() and not (setup["attacker"]["ships"] as Array).is_empty(), "naval battle setup")
	var dialog := NavalPreBattleDialog.new()
	var layer := CanvasLayer.new()
	root.add_child(layer)
	layer.add_child(dialog)
	await process_frame
	dialog.show_naval(sim, pending[0])
	_check(dialog.visible and dialog.title_label.text.begins_with("Flotte ennemie en vue"), "naval pre-battle screen: %s" % dialog.title_label.text)
	_check(dialog.body_label.text.contains("Vent"), "forecast wind: %s" % dialog.body_label.text)
	var shot := ""
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--shot="):
			shot = arg.trim_prefix("--shot=")
	if shot != "" and DisplayServer.get_name() != "headless":
		for _i in 30:
			await process_frame
		var image := root.get_texture().get_image()
		image.save_png(shot)
		print("NV1: pre-battle screenshot %s" % shot)
	var result: Dictionary = sim.call("auto_resolve_naval_battle", index)
	_check(bool(result.get("ok", false)), "auto-resolve accepted")
	_check(not (result.get("events", []) as Array).is_empty(), "auto-resolve reports the battle")
	_check((sim.call("get_pending_naval_battles") as Array).is_empty(), "nothing left pending")
	var naval: Dictionary = sim.call("get_naval_state")
	_check(naval.has("control"), "sea control exposed")
	layer.queue_free()
	await process_frame
