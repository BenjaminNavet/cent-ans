extends SceneTree

## PB3c : `BattleSim.get_soldier_buffers` (tampons groupés, cache des poses entre deux pas) rend
## exactement les transformées de `get_soldier_buffer` (un appel par camp et famille), complétées
## par des zéros à la capacité demandée — au déploiement, après un ordre, entre deux pas (même
## état, tampons réutilisés) et après plusieurs pas de simulation.
## Usage : godot --headless --path game --script res://tests/pb3c_buffers_test.gd
## Code de sortie 0 si tout passe, 1 sinon.

const KINDS := ["infantry", "archer", "cavalry", "siege"]

var _failures := 0


func _init() -> void:
	await process_frame
	if not ClassDB.class_exists("BattleSim") or not ClassDB.instantiate("BattleSim").has_method("get_soldier_buffers"):
		print("pb3c: extension absente, test ignoré")
		quit(0)
		return
	var sim: Object = ClassDB.instantiate("CampaignSim")
	if not _check(sim.call("new_campaign", HistoricalBattlesMenu.data_dir(), "fac_france", 1337), "new_campaign"):
		quit(1)
		return
	var armies: Array = BattleScene.main_armies(sim, "fac_france", "fac_england")
	var index: int = sim.call("debug_stage_battle", armies[0], armies[1])
	var battle: Object = ClassDB.instantiate("BattleSim")
	if not _check(battle.call("setup", sim.call("get_battle_setup", index), 1337), "setup"):
		quit(1)
		return
	battle.call("begin_deployment")
	_compare(battle, 0, "deployment")
	# Un régiment déplacé pendant le déploiement (aucun pas de simulation) : cache invalidé.
	var units: Array = battle.call("get_units")
	var moved := -1
	for unit in units:
		if str(unit["side"]) == "attacker" and bool(unit["present"]):
			moved = int(unit["id"])
			battle.call("deploy_unit", moved, float(unit["x"]) + 12.0, float(unit["z"]), NAN)
			break
	_compare(battle, 3, "after deploy_unit")
	battle.call("start_battle")
	battle.call("set_ai", "attacker", true)
	battle.call("set_ai", "defender", true)
	# Formation changée sans pas de simulation.
	battle.call("issue_command", {"type": "formation", "units": [moved], "kind": "column"})
	_compare(battle, 0, "after issue_command")
	# Moins d'un pas : même état, mêmes tampons.
	battle.call("tick", 0.03)
	_compare(battle, 5, "between steps")
	for i in 60:
		battle.call("tick", 0.1)
		if i % 15 == 0:
			_compare(battle, i % 4, "step %d" % i)
	battle.call("set_figure_scale", 1.5)
	_compare(battle, 0, "figure scale")
	if _failures == 0:
		print("pb3c buffers OK")
	quit(1 if _failures > 0 else 0)


## Compare les tampons groupés (capacité = effectif + `extra`) aux tampons par camp et famille.
func _compare(battle: Object, extra: int, label: String) -> void:
	var units: Array = battle.call("get_units")
	var capacities := PackedInt32Array()
	for unit in units:
		capacities.append(int(unit.get("figures", 0)) + extra if KINDS.has(str(unit["render"])) else -1)
	var result: Array = battle.call("get_soldier_buffers", capacities)
	var counts: PackedInt32Array = result[0]
	var buffers: Array = result[1]
	if not _check(buffers.size() == units.size() and counts.size() == units.size(), "%s: one buffer per regiment" % label):
		return
	for side in ["attacker", "defender"]:
		for kind in KINDS:
			var joined := PackedFloat32Array()
			for i in units.size():
				var unit: Dictionary = units[i]
				if str(unit["side"]) != side or str(unit["render"]) != kind:
					continue
				var buffer: PackedFloat32Array = buffers[i]
				var n := counts[i]
				if not _check(buffer.size() == maxi(n, capacities[i]) * 12, "%s: unit %d padded to its capacity" % [label, i]):
					return
				for k in range(n * 12, buffer.size()):
					if buffer[k] != 0.0:
						_check(false, "%s: unit %d padding is zero" % [label, i])
						return
				joined.append_array(buffer.slice(0, n * 12))
			var expected: PackedFloat32Array = battle.call("get_soldier_buffer", side, kind)
			_check(joined == expected, "%s: %s/%s buffers match get_soldier_buffer" % [label, side, kind])


func _check(ok: bool, message: String) -> bool:
	if not ok:
		_failures += 1
		push_error("pb3c: " + message)
		print("FAIL pb3c: " + message)
	return ok
