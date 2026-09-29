extends SceneTree

## Test headless du chantier FK, lot FK3 (carte vivante : réservoir, vie ordinaire, marchands) :
##  1. modèles : figurines de repli présentes, accessoires (FK2 ou maquettes) ;
##  2. réservoir plafonné (plafond, moitié en dépassement de budget, accessoires) ;
##  3. routine et charrettes instanciées au palier proche, vides au loin ;
##  4. aucune charrette sur une route commerciale coupée ; étapes maritimes écartées.
## FK4/FK5 (scènes, incidents) ajouteront leurs cas.
## Usage : godot --headless --path game --script res://tests/fk_folk_test.gd

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")

var _failures := 0


func _init() -> void:
	await process_frame
	await _run()
	FolkModels.clear_cache()
	print("fk_folk_test: %s" % ("OK" if _failures == 0 else "%d failure(s)" % _failures))
	quit(1 if _failures > 0 else 0)


func _check(condition: bool, message: String) -> bool:
	if not condition:
		_failures += 1
		push_error("fk_folk_test: " + message)
	return condition


## Arête de colonies avec tracé routier (terre) proche de `near`, hors de `avoid`.
func _edge_near(data: SettlementData, near: Vector2, avoid: Vector2) -> Array:
	var best: Array = []
	var best_d := INF
	for key in data.edge_paths:
		var ids := str(key).split("|")
		var points: PackedVector2Array = data.edge_paths[key]
		if ids.size() != 2 or points.size() < 2:
			continue
		var mid := points[points.size() / 2]
		var length := 0.0
		for i in points.size() - 1:
			length += points[i].distance_to(points[i + 1])
		if length < 15.0 or mid.distance_to(avoid) < 200.0:
			continue
		var d := mid.distance_to(near)
		if d < best_d:
			best_d = d
			best = [ids[0], ids[1], mid]
	return best


## Caméra posée sur `focus` : deux images (le placement attend une caméra immobile).
func _view(pool: FolkPool, focus: Vector2, distance: float, near_weight: float) -> void:
	pool.update_view(focus, distance, near_weight)
	pool.update_view(focus, distance, near_weight)


func _run() -> void:
	# 1. Modèles.
	_check(not FolkModels.figure_of("peasant").is_empty(), "peasant figure fallback")
	_check(not FolkModels.figure_of("guard").is_empty(), "guard figure")
	_check(not FolkModels.figure_of("rider").is_empty(), "rider figure")
	_check(FolkModels.prop_mesh("merchant_cart") != null, "merchant cart mesh")
	print("fk_folk_test: merchant_cart from %s, peasant %s" % [FolkModels.prop_source("merchant_cart"), FolkModels.figure_of("peasant")])
	var config := FolkModels.activity_config("crew", 0, "walk")
	_check((config["set"] as Array).size() > 0, "walk config")
	# Modèles FK2 : villageois par rôle, accessoires du manifeste, emplacements (+X devant).
	if BattleSkinned.has_figure("villager", 1):
		_check(FolkModels.figure_of("reaper") == ["villager", 1], "reapers are villager_1: %s" % [FolkModels.figure_of("reaper")])
		_check(FolkModels.figure_of("porter") == ["villager", 3], "porters are villager_3")
		_check(FolkModels.figure_of("peasant") == ["villager", 0], "peasants are villager_0")
		var scythe := FolkModels.activity_config("villager", 1, "scythe")
		_check((scythe["names"] as Array).has("scythe"), "reapers scythe: %s" % [scythe["names"]])
	if FolkModels.model_path("merchant_cart") != "" and ResourceLoader.exists(FolkModels.model_path("merchant_cart")):
		_check(FolkModels.prop_source("merchant_cart").ends_with("merchant_cart.glb"), "FK2 merchant cart: %s" % FolkModels.prop_source("merchant_cart"))
		_check(FolkModels.prop_source("peasant_cart").ends_with("stone_cart.glb"), "FK2 stone cart for peasants")
		var carter := FolkModels.slot("merchant_cart", "carter", Vector2(-9, -9))
		_check(carter.is_equal_approx(Vector2(0.7, 2.2)), "carter slot (lateral, ahead): %s" % carter)
		# Regard +X du glb tourné vers +Z (sens du déplacement) : le cheval est devant.
		var cart_aabb := FolkModels.prop_mesh("merchant_cart").get_aabb()
		_check(cart_aabb.end.z > 4.0 and cart_aabb.position.z > -1.5 and absf(cart_aabb.position.x) < 1.0, "cart faces +Z: %s" % cart_aabb)

	var data_dir := MAP_PATHS.default_data_dir()
	var map_dir := data_dir.path_join("map")
	var map_data := MapData.load_from_dir(map_dir)
	if not _check(map_data.load_error == "", "map load failed: %s" % map_data.load_error):
		return
	var data := SettlementData.load_from(data_dir, map_dir)
	var world := Node3D.new()
	root.add_child(world)
	var terrain := TerrainBuilder.new()
	world.add_child(terrain)
	terrain.build(map_data)
	var mask := TerroirMask.new()
	mask.build(data.settlements, data.hamlets, {}, VegetationFields.landuse(map_data), Vector2(map_data.size))

	var paris: Vector2 = data.get_settlement("set_paris")["px"]
	var active_edge := _edge_near(data, paris, Vector2(-1e6, -1e6))
	var cut_edge := _edge_near(data, paris + Vector2(0, 900), paris)
	if not _check(active_edge.size() == 3 and cut_edge.size() == 3, "trade edges found"):
		return
	var routes := [
		{"id": "trade_active", "path": PackedStringArray([active_edge[0], active_edge[1]]), "cut": false, "total_value": 90.0, "mode": "land"},
		{"id": "trade_cut", "path": PackedStringArray([cut_edge[0], cut_edge[1]]), "cut": true, "total_value": 90.0, "mode": "land"},
	]

	var pool := FolkPool.new()
	world.add_child(pool)
	pool.setup(map_data, terrain, 600)
	var caravans := FolkCaravans.new()
	caravans.setup(map_data, data)
	caravans.set_routes(routes)
	pool.register(caravans)
	var routine := FolkRoutine.new()
	routine.setup(map_data, data)
	routine.terroir = mask
	routine.season = "summer"
	routine.cities = PackedVector2Array([paris])
	pool.register(routine)
	pool.refresh(null)  # premier tour (sans simulation : routes posées par `set_routes`)
	print("fk_folk_test: routine %s, caravans %s" % [routine.stats, caravans.stats])

	# 4. Route coupée : aucun trajet ; route active : des trajets.
	var with_pieces := caravans.routes_with_pieces()
	_check(with_pieces.has("trade_active"), "active route has caravan pieces")
	_check(not with_pieces.has("trade_cut"), "cut route has no caravan piece")

	# 3. Palier proche sur la route active : charrettes, marchands et routine.
	var focus: Vector2 = active_edge[2]
	_view(pool, focus, 45.0, 1.0)
	print("fk_folk_test: near %s, caravans %s, routine %s" % [pool.stats, caravans.stats, routine.stats])
	_check(pool.figure_count() > 0 and pool.figure_count() <= 600, "figures near: %d" % pool.figure_count())
	_check(int(caravans.stats.get("carts", 0)) > 0, "merchant carts on the active route: %s" % caravans.stats)
	_check(int(routine.stats.get("roads", 0)) + int(routine.stats.get("fields", 0)) > 0, "routine folk near: %s" % routine.stats)
	_check(pool.prop_count() <= pool.effective_cap / 4, "props capped")
	_check(pool.visible, "pool visible near")
	var carts_mm := pool.get_node_or_null("Folk_merchant_cart") as MultiMeshInstance3D
	_check(carts_mm != null and carts_mm.multimesh.instance_count == int(caravans.stats["carts"]), "cart multimesh matches")
	# Déplacement en shader : trajet non nul dans les données d'instance.
	if carts_mm != null and carts_mm.multimesh.instance_count > 0:
		_check(pool.instance_custom("merchant_cart", 0).r > 0.0, "cart travels (custom data)")

	# Route coupée seule dans le rayon : aucune charrette.
	_view(pool, cut_edge[2], 45.0, 1.0)
	_check(int(caravans.stats.get("carts", -1)) == 0, "no cart on the cut route: %s" % caravans.stats)
	print("fk_folk_test: second placement %s" % pool.stats)

	# 2. Plafond : petit plafond atteint exactement, moitié en dépassement.
	pool.cap = 20
	pool.set_budget_exceeded(false)
	_view(pool, focus, 45.0, 1.0)
	_check(pool.figure_count() <= 20, "cap 20: %d" % pool.figure_count())
	pool.set_budget_exceeded(true)
	_view(pool, focus, 45.0, 1.0)
	_check(pool.effective_cap == 10 and pool.figure_count() <= 10, "halved cap: %d/%d" % [pool.figure_count(), pool.effective_cap])
	pool.cap = 600
	pool.set_budget_exceeded(false)

	# Au loin : réservoir vide.
	_view(pool, focus, 400.0, 0.0)
	_check(pool.figure_count() == 0 and not pool.visible, "empty far away")
	var total := 0
	for child in pool.get_children():
		total += (child as MultiMeshInstance3D).multimesh.instance_count
	_check(total == 0, "no instance far away: %d" % total)

	# Printemps : attelages de labour (charrue FK2 ou maquette) qui remontent leur sillon.
	routine.season = "spring"
	pool.invalidate()
	_view(pool, focus, 45.0, 1.0)
	var ploughs := pool.get_node_or_null("Folk_plough") as MultiMeshInstance3D
	_check(ploughs != null and ploughs.multimesh.instance_count > 0 and pool.instance_custom("plough", 0).r > 0.0, "plough teams in spring: %s" % routine.stats)

	# Saisons : hiver, bûcherons possibles, champs clairsemés ; pas d'erreur.
	routine.season = "winter"
	pool.invalidate()
	_view(pool, focus, 45.0, 1.0)
	print("fk_folk_test: winter %s" % routine.stats)
	_check(pool.figure_count() <= 600, "winter capped")

	world.queue_free()
	await process_frame
