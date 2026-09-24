extends SceneTree

## Lot B7 : repère les passages de gué de la bataille de démonstration (France 1337) jouée par
## l'IA des deux camps, sans rendu, pour caler une capture des éclaboussures : imprime, toutes
## les `--every=<s>` secondes, les régiments en mouvement dont l'emprise touche la rivière.
##   godot --headless --path game --script res://tests/b7_ford_probe.gd [-- --units=<n> --until=<s>]


func _init() -> void:
	var every := 5.0
	var until := 400.0
	var pad := 0
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--every="):
			every = float(arg.trim_prefix("--every="))
		elif arg.begins_with("--until="):
			until = float(arg.trim_prefix("--until="))
		elif arg.begins_with("--units="):
			pad = int(arg.trim_prefix("--units="))
	var sim: Object = ClassDB.instantiate("CampaignSim")
	var data_dir := ProjectSettings.globalize_path("res://").path_join("../data").simplify_path()
	if not sim.call("new_campaign", data_dir, "fac_france", 1337):
		push_error("b7_ford_probe: campaign failed")
		quit(1)
		return
	var armies := BattleScene.main_armies(sim, "fac_france", "fac_england")
	var index: int = sim.call("debug_stage_battle", armies[0], armies[1])
	var setup: Dictionary = sim.call("get_battle_setup", index)
	if pad > 0:
		# Comme `--units=` de la scène : chaque camp complété à `pad` régiments de 120.
		for side in ["attacker", "defender"]:
			var list: Array = setup[side]["units"]
			var base := list.duplicate(true)
			var i := 0
			while list.size() < pad and not base.is_empty():
				list.append(base[i % base.size()].duplicate(true))
				i += 1
			for unit in list:
				unit["soldiers"] = 120
				unit["max_soldiers"] = 120
	# Même graine et même IA que la scène autonome (`battle_seed` 1, IA du joueur activée).
	var battle: Object = ClassDB.instantiate("BattleSim")
	battle.call("setup", setup, 1)
	var player_side := str(setup.get("player_side", "attacker"))
	battle.call("set_ai", player_side if player_side != "" else "attacker", true)
	var terrain: Dictionary = battle.call("get_terrain")
	var river: Dictionary = terrain.get("river", {})
	if river.is_empty():
		print("b7_ford_probe: no river")
		quit(0)
		return
	print("b7_ford_probe: river keys %s, width %.0f, fords %s" % [river.keys(), float(river.get("width", 0.0)), river.get("fords", [])])
	var next := 0.0
	while float(battle.call("get_elapsed")) < until and not battle.call("is_finished"):
		battle.call("tick", 0.1)
		var now := float(battle.call("get_elapsed"))
		if now < next:
			continue
		next = now + every
		for unit in battle.call("get_units"):
			if not bool(unit["present"]):
				continue
			var state := str(unit["state"])
			if state != "marching" and state != "charging" and state != "routing":
				continue
			var x := float(unit["x"])
			var z := float(unit["z"])
			var dz := z - _river_z(river, x)
			if absf(dz) < float(river.get("width", 0.0)) * 0.5 + float(unit["depth"]) * 0.5 + 4.0:
				print("%5.0f s  %-28s %-9s (%.0f, %.0f) dz %.0f" % [now, str(unit["name"]), state, x, z, dz])
	quit(0)


## Ordonnée du lit (z) à l'abscisse `x` (polyligne `points` du dictionnaire de la rivière).
func _river_z(river: Dictionary, x: float) -> float:
	var points: Array = river.get("points", [])
	for i in points.size() - 1:
		var a: Vector2 = points[i]
		var b: Vector2 = points[i + 1]
		if x >= a.x and x <= b.x:
			return lerpf(a.y, b.y, (x - a.x) / maxf(b.x - a.x, 0.001))
	return float(river.get("z", 0.0))
