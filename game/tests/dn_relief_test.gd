extends SceneTree

## Test headless du lot DN-RELIEF (roches DN sur la carte de campagne, ADR 0218) :
##  1. catalogue : chaque modèle DN se charge en 3 niveaux, ramené à 1 m (plus grand côté
##     horizontal ≈ 1), pied à y = 0 ; les entrées `coast` portent une géologie valide ;
##  2. géologie de côte : craie sur la côte de Caux, granit en Bretagne, jamais de roche de craie
##     sur le granit ni de roche de côte dans les terres (Beauce) ;
##  3. relief : pierriers / falaises dans les Alpes, rien en mer ; coût imprimé (instances,
##     triangles, appels de dessin, durées de semis).
## Usage : godot --headless --path game --script res://tests/dn_relief_test.gd

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")
const MONT_BLANC := Vector2(2653.0, 3705.0)
const BEAUCE := Vector2(2140.0, 3290.0)

var _failures := 0


func _init() -> void:
	await process_frame
	_test_catalogue()
	await _test_map()
	if _failures > 0:
		push_error("dn_relief_test: %d failure(s)" % _failures)
		quit(1)
		return
	print("dn_relief_test: OK")
	quit(0)


func _check(condition: bool, label: String) -> bool:
	if not condition:
		_failures += 1
		push_error("FAIL: " + label)
	return condition


func _test_catalogue() -> void:
	var catalogue := RockOutcrops.load_catalogue(MAP_PATHS.default_data_dir().path_join(RockOutcrops.CATALOGUE_FILE))
	var dn_entries := 0
	for entry: Dictionary in catalogue.get("outcrops", []):
		if not entry.has("model"):
			continue
		dn_entries += 1
		var mesh := RockOutcrops._entry_mesh(entry, 0)
		if not _check(mesh != null, "%s lod0 loads" % entry["id"]):
			continue
		var box := mesh.get_aabb()
		var side := maxf(box.size.x, box.size.z)
		_check(absf(side - 1.0) < 0.12, "%s normalised to 1 m (%.3f)" % [entry["id"], side])
		_check(absf(box.position.y) < 0.02, "%s foot at y=0 (%.3f)" % [entry["id"], box.position.y])
		if entry.has("coast"):
			for rock: String in (entry["coast"] as Dictionary).get("rock", []):
				_check(rock in ["rock", "chalk", "granite"], "%s coast rock %s" % [entry["id"], rock])
	_check(dn_entries >= 20, "DN entries catalogued (%d)" % dn_entries)


func _settle(rig: CampaignCamera, at: Vector2, distance: float, data: MapData, outcrops: RockOutcrops) -> void:
	rig.look_at_point(Vector3(at.x, data.surface_world_at(at.x, at.y), at.y), distance)
	rig.snap()
	for i in 400:
		await process_frame
		if i > 5 and outcrops.pending_jobs() == 0:
			break
	outcrops.flush()
	for i in 3:
		await process_frame


func _ids_near(outcrops: RockOutcrops, at: Vector2, radius: float) -> Dictionary:
	var ids := {}
	for item: Dictionary in outcrops.instances_in(Rect2(at - Vector2(radius, radius), Vector2(radius, radius) * 2.0)):
		ids[item["id"]] = int(ids.get(item["id"], 0)) + 1
	return ids


## Premier pixel de terre côtier (rivage) de la région `id`, parcouru tous les 3 px.
func _shore_in(data: MapData, region_id: String, skip: int) -> Vector2:
	var seen := 0
	for region: Dictionary in CoastLook.data().get("regions", []):
		if region["id"] != region_id:
			continue
		var polygon := CoastLook.polygon_of(region)
		var box := Rect2(polygon[0], Vector2.ZERO)
		for point in polygon:
			box = box.expand(point)
		var y := box.position.y
		while y < box.end.y:
			var x := box.position.x
			while x < box.end.x:
				var at := Vector2(x, y)
				if Geometry2D.is_point_in_polygon(at, polygon) and data.is_land_px(int(x), int(y)):
					var d := (data.coast_dist_image.get_pixel(int(x), int(y)).r * 255.0 - 128.0) / 2.0
					if d > 0.0 and d <= 1.5 and data.height_m_at(x, y) > 5.0:
						seen += 1
						if seen > skip:
							return at
				x += 3.0
			y += 3.0
	return Vector2(-1, -1)


func _test_map() -> void:
	var map: Node3D = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	root.add_child(map)
	for i in 5:
		await process_frame
	if not _check(map.get("load_ok") == true, "campaign map loads"):
		map.queue_free()
		return
	var rig: CampaignCamera = map.camera_rig
	rig.edge_pan_enabled = false
	var data: MapData = map.map_data
	var outcrops: RockOutcrops = map.get_node_or_null("RockOutcrops")
	if not _check(outcrops != null, "RockOutcrops node"):
		map.queue_free()
		return
	_check(outcrops.models.size() >= 20, "models loaded (%d)" % outcrops.models.size())
	# Côtes : craie de Caux, granit de Bretagne.
	var chalk_total := 0
	var granite_total := 0
	var wrong := 0
	for region_id: String in ["chalk_caux", "granite_brittany", "granite_cornwall"]:
		for skip: int in [0, 15, 40, 80, 120, 200]:
			var shore := _shore_in(data, region_id, skip)
			if shore.x < 0.0:
				continue
			await _settle(rig, shore, 80.0, data, outcrops)
			var ids := _ids_near(outcrops, shore, 25.0)
			print("dn-relief: %s %s -> %s" % [region_id, shore, ids])
			if region_id == "chalk_caux":
				chalk_total += int(ids.get("coast_chalk", 0))
				wrong += int(ids.get("coast_granite", 0))
			else:
				granite_total += int(ids.get("coast_granite", 0))
				wrong += int(ids.get("coast_chalk", 0))
	_check(chalk_total > 0, "chalk cliffs on the Caux coast (%d)" % chalk_total)
	_check(granite_total > 0, "granite blocks on the Brittany/Cornwall coast (%d)" % granite_total)
	_check(wrong == 0, "no rock of the wrong geology (%d)" % wrong)
	await _settle(rig, BEAUCE, 150.0, data, outcrops)
	var inland := _ids_near(outcrops, BEAUCE, 40.0)
	_check(inland.is_empty(), "nothing in the Beauce %s" % inland)
	await _settle(rig, MONT_BLANC, 150.0, data, outcrops)
	var alps := _ids_near(outcrops, MONT_BLANC, 40.0)
	print("dn-relief: Mont-Blanc %s" % alps)
	_check(int(alps.get("scree", 0)) + int(alps.get("cliff_limestone", 0)) + int(alps.get("schist_ridge", 0)) > 20, "scree and cliffs in the Alps")
	for d: float in [60.0, 250.0, 400.0]:
		await _settle(rig, MONT_BLANC, d, data, outcrops)
		print("dn-relief: Alps d=%d stats %s" % [int(d), JSON.stringify(outcrops.stats)])
	map.queue_free()
	await process_frame
