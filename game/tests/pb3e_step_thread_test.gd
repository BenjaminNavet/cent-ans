extends SceneTree

## PB3e (ADR 0090) : le pas de bataille calculé sur un fil (`set_step_thread(true)`) donne la même
## bataille que le mode synchrone, image par image, ordres compris (mêmes `get_units`, mêmes
## tampons de figurines, même journal) ; les versions des tampons ne changent qu'avec un pas ;
## `StampMap` (piétinement, herbe couchée) remplit les mêmes octets que les boucles GDScript.
## Usage : godot --headless --path game --script res://tests/pb3e_step_thread_test.gd
## Code de sortie 0 si tout passe, 1 sinon.

var _failures := 0


func _init() -> void:
	await process_frame
	if not ClassDB.class_exists("StampMap") or not ClassDB.instantiate("BattleSim").has_method("set_step_thread"):
		print("pb3e: extension absente, test ignoré")
		quit(0)
		return
	_test_step_thread()
	_test_stamp_map()
	if _failures == 0:
		print("pb3e step thread OK")
	quit(1 if _failures > 0 else 0)


func _new_battle(sim: Object, setup: Dictionary) -> Object:
	var battle: Object = ClassDB.instantiate("BattleSim")
	battle.call("setup", setup, 1337)
	battle.call("set_ai", "attacker", true)
	battle.call("set_ai", "defender", true)
	return battle


func _test_step_thread() -> void:
	var sim: Object = ClassDB.instantiate("CampaignSim")
	if not _check(sim.call("new_campaign", HistoricalBattlesMenu.data_dir(), "fac_france", 1337), "new_campaign"):
		return
	var armies: Array = BattleScene.main_armies(sim, "fac_france", "fac_england")
	var index: int = sim.call("debug_stage_battle", armies[0], armies[1])
	var setup: Dictionary = sim.call("get_battle_setup", index)
	var sync_battle := _new_battle(sim, setup)
	var threaded := _new_battle(sim, setup)
	threaded.call("set_step_thread", true)
	_check(bool(threaded.call("get_step_thread")), "step thread on")
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
		if not _check(str(a) == str(b), "frame %d: same units" % frame):
			return
		if not _check(str(sync_battle.call("get_events")) == str(threaded.call("get_events")), "frame %d: same journal" % frame):
			return
		var capacities := PackedInt32Array()
		for unit in a:
			capacities.append(int(unit.get("figures", 0)))
		var ra: Array = sync_battle.call("get_soldier_buffers", capacities)
		var rb: Array = threaded.call("get_soldier_buffers", capacities)
		if not _check(ra[1] == rb[1], "frame %d: same figure buffers" % frame):
			return
		var ticks := int(threaded.call("get_ticks"))
		var versions: PackedInt64Array = rb[2]
		if ticks == last_ticks:
			_check(versions == last_versions, "frame %d: versions kept between steps" % frame)
		last_ticks = ticks
		last_versions = versions
	var stats: Dictionary = threaded.call("get_step_stats")
	_check(int(stats.get("adopted", 0)) > 50, "steps adopted from the worker: %s" % str(stats))


## Les deux chemins de `BattleGrassFlatten` : boucles GDScript et `StampMap`.
func _test_stamp_map() -> void:
	var gd := BattleGrassFlatten.new()
	gd.setup(Vector2(300.0, 200.0))
	var rs := BattleGrassFlatten.new()
	rs.setup(Vector2(300.0, 200.0))
	gd._map = null
	if not _check(rs._map != null, "StampMap used"):
		return
	var rng := RandomNumberGenerator.new()
	rng.seed = 7
	for k in 300:
		var c := Vector2(rng.randf_range(-20.0, 320.0), rng.randf_range(-120.0, 320.0))
		var facing := rng.randf_range(-PI, PI)
		var half := Vector2(rng.randf_range(1.0, 30.0), rng.randf_range(0.5, 8.0))
		var add_r := rng.randi_range(1, 40)
		var add_g := rng.randi_range(0, 20) if k % 3 == 0 else 0
		var cap: int = [70, 150, 255][k % 3]
		gd._stamp_box(c, facing, half, add_r, add_g, cap)
		rs._stamp_box(c, facing, half, add_r, add_g, cap)
		if k % 5 == 0:
			var p := Vector3(c.x, 0.0, c.y)
			gd.on_corpse(p, "cavalry" if k % 2 == 0 else "infantry", 0.7)
			rs.on_corpse(p, "cavalry" if k % 2 == 0 else "infantry", 0.7)
	var bytes: PackedByteArray = rs._map.call("get_bytes")
	var diff := 0
	for i in bytes.size():
		if bytes[i] != gd._bytes[i]:
			diff += 1
	_check(diff == 0, "StampMap bytes equal GDScript bytes (%d differ of %d)" % [diff, bytes.size()])
	rs.flush()
	_check(not bool(rs._map.call("is_dirty")), "uploaded")


func _check(ok: bool, label: String) -> bool:
	if not ok:
		_failures += 1
		push_error("pb3e: " + label)
		print("FAIL pb3e: " + label)
	return ok
