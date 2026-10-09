extends TestCase

## PB3e (ADR 0090) : le pas de bataille calculé sur un fil (`set_step_thread(true)`) donne la même
## bataille que le mode synchrone, image par image, ordres compris (mêmes `get_units`, mêmes
## tampons de figurines, même journal) ; les versions des tampons ne changent qu'avec un pas.
## Usage : godot --headless --path game --script res://tests/pb3e_step_thread_test.gd
## Code de sortie 0 si tout passe, 1 sinon.


func _init() -> void:
	await process_frame
	if not ClassDB.class_exists("BattleSim"):
		print("pb3e: extension absente, test ignoré")
		quit(0)
		return
	_test_step_thread()
	finish()


func _new_battle(sim: Object, setup: Dictionary) -> Object:
	var battle: Object = ClassDB.instantiate("BattleSim")
	battle.call("setup", setup, 1337)
	battle.call("set_ai", "attacker", true)
	battle.call("set_ai", "defender", true)
	return battle


func _test_step_thread() -> void:
	var sim: Object = ClassDB.instantiate("CampaignSim")
	if not check(sim.call("new_campaign", HistoricalBattlesMenu.data_dir(), "fac_france", 1337), "new_campaign"):
		return
	var armies: Array = BattleScene.main_armies(sim, "fac_france", "fac_england")
	var index: int = sim.call("debug_stage_battle", armies[0], armies[1])
	var setup: Dictionary = sim.call("get_battle_setup", index)
	var sync_battle := _new_battle(sim, setup)
	var threaded := _new_battle(sim, setup)
	threaded.call("set_step_thread", true)
	check(bool(threaded.call("get_step_thread")), "step thread on")
	var frame_times := [1.0 / 60.0, 1.0 / 30.0, 0.013, 0.25, 1.0 / 144.0]
	var last_versions := PackedInt64Array()
	var last_ticks := -1
	for frame in 900:
		if frame == 40 or frame == 300:
			for battle in [sync_battle, threaded]:
				var ours: Array = []
				for unit in battle.call("get_units"):
					if str(unit["side"]) == "attacker" and bool(unit["present"]):
						ours.append(int(unit["id"]))
				battle.call("issue_command", {"type": "move", "units": ours.slice(0, 3), "x": 500.0, "z": 300.0})
		var dt: float = frame_times[frame % frame_times.size()]
		sync_battle.call("tick", dt)
		threaded.call("tick", dt)
		if frame % 7 != 0:
			continue
		var a: Array = sync_battle.call("get_units")
		var b: Array = threaded.call("get_units")
		if not check(str(a) == str(b), "frame %d: same units" % frame):
			return
		if not check(str(sync_battle.call("get_events")) == str(threaded.call("get_events")), "frame %d: same journal" % frame):
			return
		var capacities := PackedInt32Array()
		for unit in a:
			capacities.append(int(unit.get("figures", 0)))
		var ra: Array = sync_battle.call("get_soldier_buffers", capacities)
		var rb: Array = threaded.call("get_soldier_buffers", capacities)
		if not check(ra[1] == rb[1], "frame %d: same figure buffers" % frame):
			return
		var ticks := int(threaded.call("get_ticks"))
		var versions: PackedInt64Array = rb[2]
		if ticks == last_ticks:
			check(versions == last_versions, "frame %d: versions kept between steps" % frame)
		last_ticks = ticks
		last_versions = versions
	var stats: Dictionary = threaded.call("get_step_stats")
	check(int(stats.get("adopted", 0)) > 50, "steps adopted from the worker: %s" % str(stats))
