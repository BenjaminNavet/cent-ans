extends SceneTree

## EP8b : la relation « temps de bataille écoulé -> heure affichée » (ADR 0055, addendum EP8b).
## Une vraie bataille (France contre Angleterre, 1337) forcée à démarrer à 10 h 30 (comme
## Azincourt, ADR 0035) : après 170 s simulées (2 min 50 à vitesse x1, le cas signalé), l'heure
## du cœur doit être 11 h 04 (11,0667) en phase « midday », et le bandeau formaté
## (`BattleTimeOfDay.clock_label`) doit lire « Midi, 11 h 00 » : la compression documentée par
## l'ADR 0055 (0,2 min de jour par seconde simulée), pas un bug, mais lisible.
## Usage : godot --headless --path game --script res://tests/ep8b_clock_test.gd

var _failures := 0


func _init() -> void:
	await process_frame
	if not ClassDB.class_exists("BattleSim"):
		print("ep8b: extension absente, test ignoré")
		quit(0)
		return
	var data_dir := ProjectSettings.globalize_path("res://").path_join("../data").simplify_path()
	var camp: Object = ClassDB.instantiate("CampaignSim")
	_check(bool(camp.call("new_campaign", data_dir, "fac_france", 1337)), "campaign setup")
	var armies: Array = BattleScene.main_armies(camp, "fac_france", "fac_england")
	_check(armies.size() == 2, "two main armies")
	var index: int = camp.call("debug_stage_battle", armies[0], armies[1])
	var pending: Array = camp.call("get_pending_battles")
	var setup: Dictionary = camp.call("get_battle_setup", index)
	# Comme Azincourt (ADR 0035) : 10 h 30 forcée au lieu de l'heure tirée par la campagne.
	setup["hour"] = 10.5

	var battle: Object = ClassDB.instantiate("BattleSim")
	_check(bool(battle.call("setup", setup, int(pending[0]["seed"]))), "BattleSim.setup")

	var tod0: Dictionary = battle.call("get_time_of_day")
	_check(is_equal_approx(float(tod0.get("hour", -1.0)), 10.5), "start hour is 10.5, got %s" % tod0.get("hour"))

	# 170 s simulées à pas fixe de 0,1 s : le cas signalé (2 min 50 de bataille jouée à vitesse x1).
	for _i in 1700:
		battle.call("tick", 0.1)

	var elapsed := float(battle.call("get_elapsed"))
	_check(absf(elapsed - 170.0) < 1e-6, "elapsed should be 170 s, got %s" % elapsed)

	var tod: Dictionary = battle.call("get_time_of_day")
	var hour := float(tod.get("hour", -1.0))
	_check(absf(hour - 11.066666666666666) < 1e-6, "hour should be ~11.0667 (11h04), got %s" % hour)
	_check(str(tod.get("key", "")) == "midday", "phase should be midday, got %s" % tod.get("key"))

	var label := BattleTimeOfDay.clock_label(tod)
	_check(label == "Midi, 11 h 00", "clock label should read 'Midi, 11 h 00', got '%s'" % label)

	if _failures == 0:
		print("ep8b clock OK")
	quit(1 if _failures > 0 else 0)


func _check(cond: bool, msg: String) -> bool:
	if not cond:
		_failures += 1
		push_error("ep8b: " + msg)
	return cond
