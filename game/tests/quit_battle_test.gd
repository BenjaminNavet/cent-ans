extends TestCase

## « Quitter la bataille » : une bataille de campagne (France contre Angleterre, 1337) ; Échap
## sans sélection ouvre la confirmation (bataille en pause), « Quitter la bataille » fait quitter
## le champ à l'armée (règle `concede` du cœur, déploiement compris), l'écran de fin s'ouvre et
## son bouton rend la main à la carte avec un résultat appliqué.
## Usage : godot --headless --path game --script res://tests/quit_battle_test.gd

const BATTLE_SCENE := "res://scenes/battle/battle.tscn"


func _init() -> void:
	await process_frame
	if not ClassDB.class_exists("BattleSim"):
		print("quit_battle_test: extension absente, test ignoré")
		quit(0)
		return
	await _run()
	finish()


func _run() -> void:
	var data_dir := ProjectSettings.globalize_path("res://").path_join("../data").simplify_path()
	var camp: Object = ClassDB.instantiate("CampaignSim")
	check(bool(camp.call("new_campaign", data_dir, "fac_france", 1337)), "campaign setup")
	var armies: Array = BattleScene.main_armies(camp, "fac_france", "fac_england")
	if not check(armies.size() == 2, "two main armies"):
		return
	var index: int = camp.call("debug_stage_battle", armies[0], armies[1])
	var pending: Array = camp.call("get_pending_battles")
	var scene: BattleScene = (load(BATTLE_SCENE) as PackedScene).instantiate()
	scene.configure(camp, index, int(pending[0]["seed"]))
	var returned: Array = []
	scene.returned.connect(func(result: Dictionary) -> void: returned.append(result))
	get_root().add_child(scene)
	for _i in 3:
		await process_frame
	if not check(scene.battle != null, "campaign battle staged"):
		scene.queue_free()
		return
	scene.selected.clear()
	var escape := InputEventKey.new()
	escape.keycode = KEY_ESCAPE
	escape.pressed = true
	scene.input._unhandled_input(escape)
	check(scene.hud.quit_panel.visible and scene.paused, "Escape with nothing selected should ask to quit, battle paused")
	scene.input._unhandled_input(escape)
	check(not scene.hud.quit_panel.visible and not scene.paused, "Escape again should close it and resume")
	scene.toggle_quit_menu()
	(scene.hud.quit_panel.find_child("Confirm", true, false) as Button).emit_signal("pressed")
	check(bool(scene.battle.call("is_finished")), "quitting should end the battle")
	check(str((scene.battle.call("get_outcome") as Dictionary).get("winner", "")) != scene.player_side, "quitting loses the battle")
	for _i in 5:
		await process_frame
	if not check(scene.result_screen != null, "result screen shown"):
		scene.queue_free()
		return
	scene.result_screen.return_pressed.emit()
	for _i in 60:
		if not returned.is_empty():
			break
		await process_frame
	check(returned.size() == 1, "the scene should hand back to the campaign")
	if not returned.is_empty():
		check(bool(returned[0].get("ok", false)), "result applied: %s" % returned[0])
	scene.queue_free()
	await process_frame
