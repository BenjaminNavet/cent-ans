extends TestCase

## Test headless du lot S2 (incendies de siège) sur les vraies données `data/` :
##  1. assaut de la Guyenne (`debug_stage_siege`) → `battle.tscn`, les maisons portent leur feu
##     (`get_siege().houses[i].fire`) et le vent ;
##  2. trois maisons allumées (`BattleSim.debug_ignite`) → flammes, fumée, lumières (≤ `max_lights`)
##     dans `SiegeFireFx`, ligne « maison(s) en feu » du HUD ;
##  3. 300 s de simulation → au moins une ruine (maison effondrée, tas noirci), sans erreur.
## Usage : godot --headless --path game --script res://tests/s2_fire_fx_test.gd


func _init() -> void:
	await process_frame
	await _run()
	finish()


func _run() -> void:
	if not check(ClassDB.class_exists("CampaignSim"), "GDExtension missing (run core/build.sh)"):
		return
	var data_dir: String = preload("res://scripts/map/map_paths.gd").default_data_dir()
	var sim: Object = ClassDB.instantiate("CampaignSim")
	if not check(sim.call("new_campaign", data_dir, "fac_france", 1337), "new_campaign failed"):
		return
	var armies: Array = BattleScene.main_armies(sim, "fac_france", "fac_england")
	var index: int = sim.call("debug_stage_siege", armies[0], "prov_guyenne")
	var scene: Node = (load("res://scenes/battle/battle.tscn") as PackedScene).instantiate()
	scene.configure(sim, index, 7)
	root.add_child(scene)
	for _i in 20:
		await process_frame
	if not check(scene.siege_view != null and scene.siege_view.fire_fx != null, "siege scene without fire effects"):
		return
	var fx: Node = scene.siege_view.fire_fx
	var battle: Object = scene.battle
	if battle.call("is_deploying"):
		battle.call("start_battle")
	battle.call("set_ai", "attacker", false)
	battle.call("set_ai", "defender", false)
	var siege: Dictionary = battle.call("get_siege")
	var houses: Array = siege.get("houses", [])
	check(houses.size() > 10, "houses missing")
	check((houses[0] as Dictionary).has("fire"), "houses[i].fire missing")
	check(siege.has("wind") and siege.has("gate_fire"), "wind or gate_fire missing")
	var lit := 0
	for i in [0, 3, 6]:
		if battle.call("debug_ignite", i):
			lit += 1
	check(lit == 3, "debug_ignite should light 3 houses (lit %d)" % lit)
	for _i in 200:
		battle.call("tick", 0.1)
	await create_timer(0.3).timeout
	await process_frame
	siege = battle.call("get_siege")
	check(int(siege.get("houses_burning", 0)) >= 3, "3 houses should burn (got %d)" % int(siege.get("houses_burning", 0)))
	check(int(fx.get("burning_count")) >= 3, "fire effects for the burning houses (got %d)" % int(fx.get("burning_count")))
	var lights := 0
	for child in fx.get_children():
		if child is OmniLight3D and (child as OmniLight3D).visible:
			lights += 1
	check(lights >= 3 and lights <= int(fx.get("params")["max_lights"]), "lights: %d" % lights)
	check(BattleScene.siege_status(siege).contains("maison(s) en feu"), "HUD line: %s" % BattleScene.siege_status(siege))
	for _i in 3000:
		battle.call("tick", 0.1)
	await create_timer(0.3).timeout
	await process_frame
	siege = battle.call("get_siege")
	check(int(siege.get("houses_burnt", 0)) >= 3, "the 3 houses should be ruins (got %d)" % int(siege.get("houses_burnt", 0)))
	check(int(fx.get("ruin_count")) >= 3, "ruins drawn (got %d)" % int(fx.get("ruin_count")))
	print("s2_fire_fx_test: %d burning, %d ruins, wind %s, status « %s »" % [int(siege.get("houses_burning", 0)), int(siege.get("houses_burnt", 0)), str(siege.get("wind")), BattleScene.siege_status(siege)])
	scene.queue_free()
	await process_frame
