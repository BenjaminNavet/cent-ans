extends SceneTree

## EP13 : rejeu d'après bataille — pont GDExtension (enregistrer, lister, relire, sauter, refus des
## ordres, divergence, autre format, N derniers gardés), menu « Rejeux », scène de bataille en
## rejeu (barre, vitesses, sauts) et « Revoir la bataille » depuis l'écran de fin.
## Usage : godot --headless --path game --script res://tests/ep13_replay_test.gd
## Code de sortie 0 si tout passe, 1 sinon.

const BATTLE_SCENE := "res://scenes/battle/battle.tscn"

var _failures := 0
var _dir := ""


func _init() -> void:
	await process_frame
	if not ClassDB.class_exists("BattleSim") or not ClassDB.instantiate("BattleSim").has_method("load_replay"):
		print("ep13: extension absente, test ignoré")
		quit(0)
		return
	_dir = ProjectSettings.globalize_path("user://ep13_test_replays")
	_clear_dir()
	ReplaysMenu.dir_override = _dir
	var path := _record_battle()
	if path != "":
		_check_playback(path)
		_check_divergence_and_format(path)
		await _check_menu()
		await _check_scene(path)
		_check_keep_count(path)
	await _check_result_screen_replay()
	_clear_dir()
	ReplaysMenu.dir_override = ""
	if _failures == 0:
		print("ep13 replay OK")
	quit(1 if _failures > 0 else 0)


## Bataille France–Angleterre de la campagne 1337, IA des deux camps, jusqu'à la fin ; enregistrée.
var _outcome: Dictionary = {}
var _setup: Dictionary = {}


func _record_battle() -> String:
	var sim: Object = ClassDB.instantiate("CampaignSim")
	var data_dir := HistoricalBattlesMenu.data_dir()
	if not _check(sim.call("new_campaign", data_dir, "fac_france", 1337), "new_campaign"):
		return ""
	var armies: Array = BattleScene.main_armies(sim, "fac_france", "fac_england")
	var index: int = sim.call("debug_stage_battle", armies[0], armies[1])
	var setup: Dictionary = sim.call("get_battle_setup", index)
	_setup = setup
	var battle: Object = ClassDB.instantiate("BattleSim")
	if not _check(battle.call("setup", setup, 1337), "setup"):
		return ""
	battle.call("set_ai", "attacker", true)
	# Un ordre du joueur (enregistré) avant de laisser l'IA mener.
	var ours: Array = []
	for unit in battle.call("get_units"):
		if str(unit["side"]) == "attacker":
			ours.append(int(unit["id"]))
	battle.call("issue_command", {"type": "halt", "units": [ours[0]]})
	_check(not battle.call("is_replay"), "a live battle is not a replay")
	var frames := 0
	while not battle.call("is_finished") and frames < 20000:
		battle.call("tick", 0.4)
		frames += 1
	if not _check(battle.call("is_finished"), "the recorded battle ends"):
		return ""
	_outcome = battle.call("get_outcome")
	var path := str(battle.call("save_replay", _dir, "Bataille d'essai"))
	_check(path != "" and FileAccess.file_exists(path), "save_replay writes a file: %s" % path)
	var listed: Array = battle.call("list_replays", _dir)
	_check(listed.size() == 1, "one replay listed, got %d" % listed.size())
	if listed.size() == 1:
		var entry: Dictionary = listed[0]
		_check(str(entry["title"]) == "Bataille d'essai" and bool(entry["readable"]), "title and readable: %s" % str(entry))
		_check(str(entry["winner"]) == str(_outcome["winner"]), "listed winner")
		_check(absf(float(entry["duration"]) - float(_outcome["duration"])) < 1.0, "listed duration")
		_check(ReplaysMenu.summary(entry).contains("contre"), "summary line: %s" % ReplaysMenu.summary(entry))
	return path


## Relecture : même issue, ordres refusés, sauts avant et arrière.
func _check_playback(path: String) -> void:
	var replay: Object = ClassDB.instantiate("BattleSim")
	var loaded: Dictionary = replay.call("load_replay", path)
	if not _check(bool(loaded.get("ok", false)), "load_replay: %s" % loaded.get("error", "")):
		return
	_check(replay.call("is_replay"), "is_replay")
	var refused: Dictionary = replay.call("issue_command", {"type": "halt", "units": [0]})
	_check(not bool(refused.get("ok", true)) and str(refused.get("error", "")).contains("rejeu"), "orders refused during a replay")
	_check(not bool((replay.call("deploy_unit", 0, 100.0, 100.0, NAN) as Dictionary).get("ok", true)), "deployment refused")
	var info: Dictionary = replay.call("get_replay")
	_check(absf(float(info["duration"]) - float(_outcome["duration"])) < 1.0, "replay duration")
	_check((info["divergence"] as Dictionary).is_empty(), "no divergence at the start")
	# Saut en avant, puis en arrière, puis lecture jusqu'au bout à ×8.
	replay.call("replay_seek", 30.0)
	_check(absf(float(replay.call("get_elapsed")) - 30.0) < 0.2, "seek forward to 30 s")
	var at_30: Array = replay.call("get_units")
	replay.call("replay_seek", float(info["duration"]) * 0.8)
	replay.call("replay_seek", 30.0)
	_check(absf(float(replay.call("get_elapsed")) - 30.0) < 0.2, "seek back to 30 s")
	var again: Array = replay.call("get_units")
	_check(str(at_30[0]["x"]) == str(again[0]["x"]) and int(at_30[0]["soldiers"]) == int(again[0]["soldiers"]), "same state after a jump back")
	var guard := 0
	while not bool((replay.call("get_replay") as Dictionary)["at_end"]) and guard < 20000:
		replay.call("tick", 0.1 * 8.0)
		guard += 1
	var outcome: Dictionary = replay.call("get_outcome")
	_check(str(outcome.get("winner", "")) == str(_outcome["winner"]), "same winner on replay")
	_check(absf(float(outcome.get("duration", 0.0)) - float(_outcome["duration"])) < 0.01, "same duration on replay")
	_check(int((outcome["attacker"] as Dictionary)["total_losses"]) == int((_outcome["attacker"] as Dictionary)["total_losses"]), "same attacker losses")
	_check((replay.call("get_replay")["divergence"] as Dictionary).is_empty(), "no divergence to the end")


## Règles changées (empreinte faussée) : signalé ; autre format : refusé avec un message.
func _check_divergence_and_format(path: String) -> void:
	# Le texte est modifié tel quel : le JSON de Godot arrondirait la graine (entier 64 bits).
	var text := FileAccess.get_file_as_string(path)
	var json: Dictionary = JSON.parse_string(text)
	var checkpoints: Array = json["checkpoints"]
	if checkpoints.size() > 2:
		var digest := str(checkpoints[2]["digest"])
		var tampered := _dir.path_join("rejeu-0000000001.json")
		_write(tampered, text.replace('"digest":"%s"' % digest, '"digest":"0000000000000000"'))
		var replay: Object = ClassDB.instantiate("BattleSim")
		var loaded: Dictionary = replay.call("load_replay", tampered)
		if _check(bool(loaded.get("ok", false)), "tampered replay loads: %s" % loaded.get("error", "")):
			replay.call("replay_seek", 100000.0)
			var divergence: Dictionary = (replay.call("get_replay") as Dictionary)["divergence"]
			_check(str(divergence.get("message", "")).contains("règles"), "divergence reported: %s" % str(divergence))
		DirAccess.remove_absolute(tampered)
	var future := _dir.path_join("rejeu-0000000002.json")
	_write(future, text.replace('"format":1,', '"format":99,'))
	var reader: Object = ClassDB.instantiate("BattleSim")
	var result: Dictionary = reader.call("load_replay", future)
	_check(not bool(result.get("ok", true)) and str(result.get("error", "")).contains("plus récente"), "another format refused: %s" % str(result))
	var listed: Array = reader.call("list_replays", _dir)
	var unreadable := listed.filter(func(e: Dictionary) -> bool: return not bool(e["readable"]))
	_check(unreadable.size() == 1, "the other format is listed as unreadable")


## Les N derniers seulement (`data/rules/battle_replay.json`).
func _check_keep_count(_path: String) -> void:
	var rules: Dictionary = JSON.parse_string(FileAccess.get_file_as_string(HistoricalBattlesMenu.data_dir().path_join("rules/battle_replay.json")))
	var keep := int(rules["keep_count"])
	var battle: Object = ClassDB.instantiate("BattleSim")
	battle.call("setup", _setup, 3)
	for i in keep + 3:
		battle.call("save_replay", _dir, "Copie %d" % i)
	var listed: Array = battle.call("list_replays", _dir)
	_check(listed.size() == keep, "%d replays kept, got %d" % [keep, listed.size()])
	_check(str(listed[0]["title"]) == "Copie %d" % (keep + 2), "newest first: %s" % str(listed[0]["title"]))


## Menu « Rejeux » : une ligne et un bouton « Revoir » par rejeu.
func _check_menu() -> void:
	var menu := ReplaysMenu.new()
	get_root().add_child(menu)
	await process_frame
	_check(menu.buttons.size() == menu.replays.size() and menu.replays.size() > 0, "menu lists the replays (%d)" % menu.buttons.size())
	_check(menu.empty_label == null, "no empty notice")
	menu.queue_free()
	await process_frame


## Scène de bataille en rejeu (`--replay=`) : barre, pas d'ordre, ×8, sauts.
func _check_scene(path: String) -> void:
	BattleScene.demo_args = ReplaysMenu.args_for(path)
	var scene: BattleScene = (load(BATTLE_SCENE) as PackedScene).instantiate()
	get_root().add_child(scene)
	for _i in 5:
		await process_frame
	if not _check(scene.replay_mode and scene.replay_bar != null, "scene in replay mode: %s" % scene.replay_error):
		scene.queue_free()
		await process_frame
		return
	_check(scene.deployment == null, "no deployment during a replay")
	_check(not bool(scene.issue({"type": "halt", "units": [0]}).get("ok", true)), "scene refuses orders")
	_check(scene._leader_bar == null or not scene._leader_bar.visible, "leader orders hidden")
	scene.replay_bar.speed_chosen.emit(8.0)
	_check(is_equal_approx(scene.speed, 8.0) and not scene.paused, "×8")
	for _i in 10:
		await process_frame
	_check(float(scene.battle.call("get_elapsed")) > 0.0, "the replay plays")
	scene.replay_bar.seek_requested.emit(40.0)
	_check(absf(float(scene.battle.call("get_elapsed")) - 40.0) < 0.2, "timeline jump to 40 s")
	scene.replay_seek(10.0)
	_check(absf(float(scene.battle.call("get_elapsed")) - 10.0) < 0.2, "jump back to 10 s")
	_check(scene.soldiers != null and is_instance_valid(scene.soldiers), "figures rebuilt after a jump back")
	await process_frame
	scene.replay_bar.play_toggled.emit()
	_check(scene.paused, "pause")
	_check(scene.replay_bar.play_button.text == "Lecture", "play button label")
	scene.queue_free()
	await process_frame


## Bataille jouée jusqu'à l'écran de fin : enregistrée, « Revoir la bataille » la rejoue sur place.
func _check_result_screen_replay() -> void:
	BattleScene.demo_args = PackedStringArray(["--autoplay"])
	var scene: BattleScene = (load(BATTLE_SCENE) as PackedScene).instantiate()
	get_root().add_child(scene)
	await process_frame
	if not _check(scene.battle != null, "demo battle staged"):
		scene.queue_free()
		return
	scene._fast_forward(3600.0)
	for _i in 3:
		await process_frame
	if not _check(scene.result_screen != null, "result screen shown"):
		scene.queue_free()
		return
	_check(scene.replay_saved_path != "", "battle saved at its end")
	_check(scene.result_screen.replay_button.visible, "« Revoir la bataille » shown")
	scene.result_screen.replay_pressed.emit()
	await process_frame
	_check(scene.replay_mode and bool(scene.battle.call("is_replay")), "replay started in place")
	_check(float(scene.battle.call("get_elapsed")) < 1.0, "from the start")
	_check(scene.result_screen == null, "result screen closed")
	scene.queue_free()
	await process_frame


func _write(path: String, text: String) -> void:
	var file := FileAccess.open(path, FileAccess.WRITE)
	file.store_string(text)
	file.close()


func _clear_dir() -> void:
	if not DirAccess.dir_exists_absolute(_dir):
		return
	for name in DirAccess.get_files_at(_dir):
		DirAccess.remove_absolute(_dir.path_join(name))


func _check(cond: bool, msg: String) -> bool:
	if not cond:
		_failures += 1
		push_error("ep13: " + msg)
	return cond
