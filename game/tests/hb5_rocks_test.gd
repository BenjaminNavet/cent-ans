extends SceneTree

## Test headless du lot HB5 (affleurements rocheux, ADR 0143) :
##  1. catalogue `data/art/rock_outcrops.yaml` lu, six modèles × 3 niveaux chargés (triangles
##     décroissants) ;
##  2. carte : instances posées dans les Alpes (Mont-Blanc, Écrins) et le Massif central (Sancy,
##     Cantal), aucune en Beauce ni en mer (golfe de Gascogne) ;
##  3. hauteur au sol cohérente (pied sous la surface affichée au centre, pas plus bas que
##     l'emprise ne le justifie) ; mesures (instances, triangles, appels) imprimées à d = 60, 250, 400.
## Usage : godot --headless --path game --script res://tests/hb5_rocks_test.gd

const MONT_BLANC := Vector2(2653.0, 3705.0)
const ECRINS := Vector2(2591.0, 3843.0)
const SANCY := Vector2(2212.0, 3721.0)
const CANTAL := Vector2(2200.0, 3785.0)
const BEAUCE := Vector2(2140.0, 3290.0)
const BISCAY := Vector2(1492.0, 3544.0)

var _failures := 0


func _init() -> void:
	await process_frame
	_test_catalogue()
	await _test_map()
	if _failures > 0:
		push_error("hb5_rocks_test: %d failure(s)" % _failures)
		quit(1)
		return
	print("hb5_rocks_test: OK")
	quit(0)


func _check(condition: bool, label: String) -> bool:
	if not condition:
		_failures += 1
		push_error("FAIL: " + label)
	return condition


func _test_catalogue() -> void:
	var catalogue := RockOutcrops.load_catalogue(MapPaths.default_data_dir().path_join(RockOutcrops.CATALOGUE_FILE))
	_check(not catalogue.is_empty(), "catalogue loads")
	var outcrops: Array = catalogue.get("outcrops", [])
	_check(outcrops.size() >= 6, "six outcrops catalogued (%d)" % outcrops.size())
	_check(catalogue.has("render"), "render settings")


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


func _count(outcrops: RockOutcrops, at: Vector2, radius: float) -> Array:
	return outcrops.instances_in(Rect2(at - Vector2(radius, radius), Vector2(radius, radius) * 2.0))


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
	_check(outcrops.models.size() == 6, "six models loaded (%d)" % outcrops.models.size())
	for model: Dictionary in outcrops.models:
		var tris: Array = model["triangles"]
		_check(int(tris[0]) > int(tris[1]) and int(tris[1]) > int(tris[2]) and int(tris[0]) <= 600, "%s LOD triangles %s" % [model["id"], tris])
	print("hb5: biome source %s" % outcrops.biome_source)
	var sites := {"Mont-Blanc": MONT_BLANC, "Ecrins": ECRINS, "Sancy": SANCY, "Cantal": CANTAL, "Beauce": BEAUCE, "Biscay": BISCAY}
	var found := {}
	for site: String in sites:
		var at: Vector2 = sites[site]
		await _settle(rig, at, 150.0, data, outcrops)
		var list := _count(outcrops, at, 40.0)
		found[site] = list
		var ids := {}
		for item: Dictionary in list:
			ids[item["id"]] = int(ids.get(item["id"], 0)) + 1
		print("hb5: %s d=150 r=40: %d outcrops %s" % [site, list.size(), ids])
	_check((found["Mont-Blanc"] as Array).size() + (found["Ecrins"] as Array).size() >= 30, "outcrops in the Alps")
	_check((found["Sancy"] as Array).size() + (found["Cantal"] as Array).size() >= 8, "outcrops in the Massif central")
	_check((found["Beauce"] as Array).is_empty(), "no outcrop in the Beauce")
	_check((found["Biscay"] as Array).is_empty(), "no outcrop at sea")
	var alpine := 0
	for item: Dictionary in found["Mont-Blanc"]:
		alpine += 1 if item["id"] == "alpine_spire" else 0
	_check(alpine > 0, "alpine spires around the Mont-Blanc")
	# Hauteur : pied au plus bas de l'emprise, donc sous la surface au centre, sans s'enfoncer
	# plus que le relief de l'emprise.
	await _settle(rig, MONT_BLANC, 60.0, data, outcrops)
	var worst := 0.0
	var checked := 0
	for item: Dictionary in _count(outcrops, MONT_BLANC, 20.0):
		var ground := data.surface_world_at(item["x"], item["y"])
		var r: float = item["size"]
		var low := ground
		for offset: Vector2 in [Vector2(r, 0), Vector2(-r, 0), Vector2(0, r), Vector2(0, -r)]:
			low = minf(low, data.surface_world_at(item["x"] + offset.x, item["y"] + offset.y))
		var h: float = item["height"]
		var ok := h <= ground + 0.25 and h >= low - 0.5
		worst = maxf(worst, maxf(h - ground, low - h))
		checked += 1
		if not ok:
			_check(false, "outcrop grounded at (%.1f, %.1f): %.2f vs ground %.2f / low %.2f" % [item["x"], item["y"], h, ground, low])
			break
	_check(checked > 0, "outcrops to ground-check")
	print("hb5: grounding checked %d, worst gap %.3f" % [checked, worst])
	for d: float in [60.0, 250.0, 400.0]:
		await _settle(rig, MONT_BLANC, d, data, outcrops)
		print("hb5: Alps d=%d stats %s" % [int(d), JSON.stringify(outcrops.stats)])
		_check(int(outcrops.stats["triangles"]) <= int(outcrops.settings["max_visible_triangles"]), "triangles bounded at d=%d" % int(d))
	await _settle(rig, MONT_BLANC, 1250.0, data, outcrops)
	_check(not outcrops.visible, "hidden beyond the 3D view (d=1250)")
	map.queue_free()
	await process_frame
