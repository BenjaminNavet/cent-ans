extends SceneTree

## RJ-b : figurines et régiments interpolés entre deux pas de simulation (0,1 s).
## Bataille de démonstration (France 1337, armées principales), tous les régiments en marche
## vers le camp adverse, sans rendu, 60 images par seconde simulée :
## 1. avec `set_pose_lerp(true)`, la première figurine d'un régiment en marche bouge à presque
##    chaque image, par petits pas réguliers (sans interpolation : immobile 5 images, puis saut) ;
##    le centre du régiment (`get_units` x/z) aussi ;
## 2. un régiment immobile garde la même version de tampon entre deux pas (pas de renvoi) ;
## 3. `ground_speed` (vitesse du dernier pas) est fourni ;
## 4. coût de `get_soldier_buffers` par image, interpolation coupée puis active (µs, imprimé).
## Usage : godot --headless --path game --script res://tests/rj_b_pose_lerp_test.gd

const FRAME := 1.0 / 60.0

var _failures := 0


func _init() -> void:
	var sim: Object = ClassDB.instantiate("CampaignSim")
	var data_dir := ProjectSettings.globalize_path("res://").path_join("../data").simplify_path()
	if not sim.call("new_campaign", data_dir, "fac_france", 1337):
		push_error("rj_b_pose_lerp_test: campaign failed")
		quit(1)
		return
	var armies := BattleScene.main_armies(sim, "fac_france", "fac_england")
	var index: int = sim.call("debug_stage_battle", armies[0], armies[1])
	var setup: Dictionary = sim.call("get_battle_setup", index)
	var plain := _battle(setup, false)
	var blended := _battle(setup, true)
	_check(blended.has_method("set_pose_lerp") and bool(blended.call("get_pose_lerp")), "set_pose_lerp missing")
	# Tous les régiments marchent vers le centre du camp adverse ; 3 s simulées au pas fixe.
	for battle in [plain, blended]:
		_march(battle)
		for i in 30:
			battle.call("tick", 0.1)
			battle.call("get_units")  # positions de deux pas consécutifs (vitesse du dernier pas)
	print("rj_b_pose_lerp_test: ticks %d" % int(blended.call("get_ticks")))
	var capacities := _capacities(blended.call("get_units"))
	var mover := _moving_unit(blended, capacities)
	_check(mover >= 0, "no marching regiment after 3 s")
	if mover >= 0:
		var smooth := _track(blended, capacities, mover, 120)
		var stepped := _track(plain, capacities, mover, 120)
		print("rj_b_pose_lerp_test: regiment %d, figure moved on %d/%d frames (%d without lerp), max/mean step %.2f (%.2f without)" % [mover, smooth["moved"], smooth["frames"], stepped["moved"], smooth["ratio"], stepped["ratio"]])
		_check(int(smooth["moved"]) >= int(smooth["frames"]) * 0.8, "blended figure still frozen between steps: %s" % [smooth])
		_check(int(stepped["moved"]) <= int(stepped["frames"]) * 0.3, "plain figure should move only at steps: %s" % [stepped])
		_check(float(smooth["ratio"]) < 2.5, "blended steps uneven: %s" % [smooth])
		_check(int(smooth["centre_moved"]) >= int(smooth["frames"]) * 0.8, "regiment centre still stepped: %s" % [smooth])
		_check(float(smooth["ground_speed"]) > 0.2, "ground_speed missing or zero: %s" % [smooth])
	var idle := _battle(setup, true)
	idle.call("set_ai", "attacker", false)
	idle.call("set_ai", "defender", false)
	for i in 3:
		idle.call("tick", 0.1)
		idle.call("get_units")
	_check_idle_versions(idle, capacities)
	_bench(setup)
	print("rj_b_pose_lerp_test: %s" % ("OK" if _failures == 0 else "%d failure(s)" % _failures))
	quit(1 if _failures > 0 else 0)


func _check(condition: bool, message: String) -> bool:
	if not condition:
		_failures += 1
		push_error("rj_b_pose_lerp_test: " + message)
	return condition


func _battle(setup: Dictionary, lerp: bool) -> Object:
	var battle: Object = ClassDB.instantiate("BattleSim")
	battle.call("setup", setup, 1337)
	battle.call("set_ai", "attacker", true)
	battle.call("set_ai", "defender", true)
	battle.call("set_pose_lerp", lerp)
	return battle


func _march(battle: Object) -> void:
	var units: Array = battle.call("get_units")
	var centres := {"attacker": Vector2.ZERO, "defender": Vector2.ZERO}
	var counts := {"attacker": 0, "defender": 0}
	for unit in units:
		var side := str(unit["side"])
		centres[side] += Vector2(float(unit["x"]), float(unit["z"]))
		counts[side] += 1
	for unit in units:
		var other := "defender" if str(unit["side"]) == "attacker" else "attacker"
		var target: Vector2 = centres[other] / maxf(float(counts[other]), 1.0)
		battle.call("issue_command", {"type": "move", "units": [int(unit["id"])], "x": target.x, "z": target.y})


func _capacities(units: Array) -> PackedInt32Array:
	var out := PackedInt32Array()
	out.resize(units.size())
	for i in units.size():
		out[i] = int(units[i].get("figures", units[i]["soldiers"])) if bool(units[i]["present"]) else -1
	return out


## Premier régiment dont le centre a bougé sur le dernier pas.
func _moving_unit(battle: Object, capacities: PackedInt32Array) -> int:
	for unit in battle.call("get_units"):
		if bool(unit["present"]) and float(unit.get("ground_speed", 0.0)) > 0.5 and capacities[int(unit["id"])] > 0:
			return int(unit["id"])
	return -1


## Avance `frames` images de 1/60 s ; pas de la première figurine et du centre de `id`.
func _track(battle: Object, capacities: PackedInt32Array, id: int, frames: int) -> Dictionary:
	var last := Vector2.INF
	var last_centre := Vector2.INF
	var moved := 0
	var centre_moved := 0
	var total := 0.0
	var largest := 0.0
	var speed := 0.0
	for f in frames:
		battle.call("tick", FRAME)
		var unit: Dictionary = battle.call("get_units")[id]
		speed = maxf(speed, float(unit.get("ground_speed", 0.0)))
		var buffers: Array = battle.call("get_soldier_buffers", capacities)
		var buffer: PackedFloat32Array = buffers[1][id]
		if buffer.size() < 12:
			continue
		var at := Vector2(buffer[3], buffer[11])
		var centre := Vector2(float(unit["x"]), float(unit["z"]))
		if last != Vector2.INF:
			var d := at.distance_to(last)
			if d > 1e-4:
				moved += 1
				total += d
				largest = maxf(largest, d)
			if centre.distance_to(last_centre) > 1e-4:
				centre_moved += 1
		last = at
		last_centre = centre
	var mean := total / maxf(float(moved), 1.0)
	return {"frames": frames - 1, "moved": moved, "centre_moved": centre_moved, "ratio": largest / maxf(mean, 1e-6), "ground_speed": speed}


## Entre deux pas, un régiment immobile rend la même version de tampon.
func _check_idle_versions(battle: Object, capacities: PackedInt32Array) -> void:
	var first: Array = battle.call("get_soldier_buffers", capacities)
	var ticks := int(battle.call("get_ticks"))
	var frac := float(battle.call("get_step_fraction"))
	_check(frac >= 0.0 and frac <= 1.0, "step fraction out of range: %f" % frac)
	if frac + FRAME / 0.1 >= 1.0:
		battle.call("tick", FRAME)
		first = battle.call("get_soldier_buffers", capacities)
		ticks = int(battle.call("get_ticks"))
	battle.call("tick", FRAME)
	var second: Array = battle.call("get_soldier_buffers", capacities)
	if int(battle.call("get_ticks")) != ticks:
		return
	var units: Array = battle.call("get_units")
	var idle := 0
	for unit in units:
		var id := int(unit["id"])
		if capacities[id] <= 0 or float(unit.get("ground_speed", 1.0)) > 0.0:
			continue
		var a: PackedFloat32Array = first[1][id]
		var b: PackedFloat32Array = second[1][id]
		if a != b:
			continue  # figures qui bougent dans un régiment arrêté (mêlée, poussée)
		idle += 1
		_check((first[2] as PackedInt64Array)[id] == (second[2] as PackedInt64Array)[id], "idle regiment %d got a new buffer version" % id)
	print("rj_b_pose_lerp_test: %d idle regiments kept their buffer version" % idle)


## Coût par image de `get_soldier_buffers` (toutes les figurines), sans puis avec interpolation.
func _bench(setup: Dictionary) -> void:
	for lerp in [false, true, false, true]:
		var battle := _battle(setup, lerp)
		_march(battle)
		for i in 30:
			battle.call("tick", 0.1)
		var capacities := _capacities(battle.call("get_units"))
		var figures := 0
		for c in capacities:
			figures += maxi(c, 0)
		var spent := 0
		var frames := 360
		for f in frames:
			battle.call("tick", FRAME)
			var start := Time.get_ticks_usec()
			battle.call("get_units")
			battle.call("get_soldier_buffers", capacities)
			spent += Time.get_ticks_usec() - start
		print("rj_b_pose_lerp_test: bench lerp=%s, %d figures, get_units+get_soldier_buffers %.0f µs/frame" % [lerp, figures, float(spent) / frames])
