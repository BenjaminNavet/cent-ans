extends SceneTree

## Lot B4 : rythme de la bataille de démonstration (France 1337, armées principales) jouée par
## l'IA des deux camps, sans rendu. Imprime l'instant du premier contact et l'état des
## régiments toutes les 30 s (`--every=<s>`, jusqu'à `--until=<s>`) ; `--dump=<json>` écrit
## la mise en place de la bataille (fixture des tests Rust `sim-battle/tests/fixtures/`).
##   godot --headless --path game --script res://tests/b4_pacing.gd [-- --dump=<json>]

var _every := 30.0


func _init() -> void:
	var dump := ""
	var until := 600.0
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--every="):
			_every = float(arg.trim_prefix("--every="))
		elif arg.begins_with("--until="):
			until = float(arg.trim_prefix("--until="))
		elif arg.begins_with("--dump="):
			dump = arg.trim_prefix("--dump=")
	var sim: Object = ClassDB.instantiate("CampaignSim")
	var data_dir := ProjectSettings.globalize_path("res://").path_join("../data").simplify_path()
	if not sim.call("new_campaign", data_dir, "fac_france", 1337):
		push_error("b4_pacing: campaign failed")
		quit(1)
		return
	var armies := BattleScene.main_armies(sim, "fac_france", "fac_england")
	var index: int = sim.call("debug_stage_battle", armies[0], armies[1])
	var setup: Dictionary = sim.call("get_battle_setup", index)
	if dump != "":
		var file := FileAccess.open(dump, FileAccess.WRITE)
		file.store_string(JSON.stringify(setup, " "))
		file.close()
	var battle: Object = ClassDB.instantiate("BattleSim")
	battle.call("setup", setup, 1337)
	battle.call("set_ai", "attacker", true)
	var contact := -1.0
	var next_report := 0.0
	while float(battle.call("get_elapsed")) < until and not battle.call("is_finished"):
		battle.call("tick", 0.1)
		var elapsed := float(battle.call("get_elapsed"))
		var units: Array = battle.call("get_units")
		if contact < 0.0:
			for unit in units:
				if str(unit["state"]) == "melee":
					contact = elapsed
					print("b4_pacing: first contact at %.1f s" % contact)
					break
		if elapsed >= next_report:
			next_report += _every
			print("t=%.0f" % elapsed)
			for unit in units:
				if bool(unit["present"]):
					print("  %s %-28s (%4.0f, %4.0f) %-9s tgt %d" % [unit["side"], unit["name"], unit["x"], unit["z"], unit["state"], unit["target"]])
	print("b4_pacing: end at %.0f s, contact %.1f s" % [float(battle.call("get_elapsed")), contact])
	quit(0)
