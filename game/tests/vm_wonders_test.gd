extends TestCase

## Test headless VM (ADR 0356) : sites isolés Gizeh et Pétra planifiés à leur place, sur la terre,
## affichés de près avec leur glb Meshy quand le paquet de modèles est installé.
## Usage : godot --headless --path game --script res://tests/vm_wonders_test.gd

const SITES := {"giza_pyramids": "pyramids_giza", "petra": "petra_ruins"}


## Brouillard factice : les provinces des deux sites sont cachées au joueur.
class FogStub:
	extends RefCounted
	var hidden_provinces := {}


func _init() -> void:
	await process_frame
	await _run()
	finish()


func _run() -> void:
	var data_dir := OutbuildingLayer.data_dir()
	var config := DecorPlanner.load_config(data_dir)
	var map_data := MapData.load_from_dir(data_dir.path_join("map"))
	var data := SettlementData.load_from(data_dir, data_dir.path_join("map"))
	var planner := DecorPlanner.new()
	planner.setup(config, map_data, data, DecorPlanner.load_province_info(data_dir), DecorPlanner.load_crossings(data_dir))
	var plan := planner.plan()
	var by_site := {}
	for instance: Dictionary in plan:
		if SITES.has(instance.get("site", "")):
			by_site[instance["site"]] = instance
	for site: String in SITES:
		check(by_site.has(site), "site %s not planned" % site)
	print("vm_wonders_test: planned %s" % str(by_site.keys()))
	var world := Node3D.new()
	root.add_child(world)
	var terrain := TerrainBuilder.new()
	world.add_child(terrain)
	terrain.build(map_data)
	var camera := Camera3D.new()
	world.add_child(camera)
	camera.current = true
	var layer := SettlementLayer.new()
	world.add_child(layer)
	layer.setup(map_data, terrain, data, ZoomTiers.load_default())
	var decor := layer.decor
	if not check(decor != null and decor.enabled, "no decor layer"):
		return
	decor.force_active = true
	decor.set_date(1337, "summer")
	var fog := FogStub.new()
	for site: String in by_site:
		fog.hidden_provinces[str(by_site[site]["province"])] = true
	decor.fog_source = fog
	for site: String in by_site:
		var focus: Vector2 = by_site[site]["px"]
		camera.look_at_from_position(Vector3(focus.x, 40.0, focus.y + 40.0), Vector3(focus.x, 0.0, focus.y), Vector3.UP)
		decor.update_view(60.0)
		decor.flush(focus)
		var shown := decor.shown_instances().filter(func(i: Dictionary) -> bool: return i.get("site", "") == site)
		check(shown.size() == 1, "site %s not shown at rig 60 under fog" % site)
		# Largeur monde attendue (le renderer factice ne garde pas les transformations d'instances).
		var spec: Dictionary = decor.config["types"][SITES[site]]
		var width_units := float((spec["size_m"] as Array)[0]) * decor.factor_of(spec, decor._size_distance()) / decor._mpu
		check(width_units > 6.0, "site %s too small on screen (%.1f units)" % [site, width_units])
		print("vm_wonders_test: %s source=%s width=%.1f units" % [site, decor.model_source(SITES[site]), width_units])
