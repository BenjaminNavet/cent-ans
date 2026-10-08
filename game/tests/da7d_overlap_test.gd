extends SceneTree

## Test headless du lot DA7d (ADR 0066) : plus aucun chevauchement des marqueurs de lieux et de
## leurs noms dans les régions denses, aux zooms types de la carte de campagne.
##  1. `MarkerDeclutter` sur des rectangles synthétiques (priorité, épinglés, grille) ;
##  2. vraie carte : Flandre, Île-de-France, Normandie à 4 distances caméra : 0 paire de
##     rectangles écran visibles qui se recouvrent (marqueurs + noms, y compris un marqueur et son nom),
##     épinglés (capitale du joueur, sélection) toujours affichés ;
##  3. borne sur le temps d'un recalcul complet.
## Usage : godot --headless --path game --script res://tests/da7d_overlap_test.gd [-- --measure]
## (`--measure` imprime le tableau sans échouer : mesures « avant ».)

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")
## Régions denses (coordonnées carte 4096) et zooms types (distance caméra) : palier Europe,
## province, province rapprochée, moyen près du palier comté.
const REGIONS := {"flandre": Vector2(2330, 2910), "ile_de_france": Vector2(2213, 3203), "normandie": Vector2(2030, 3120)}
const DISTANCES := [1100.0, 600.0, 330.0, 200.0]
## Plafond du meilleur temps d'un recalcul complet (ms). Large exprès : sur machine chargée le
## temps mesuré varie du simple au triple ; la borne ne sert qu'à attraper une dérive
## algorithmique (quadratique), pas un réglage fin. Le chevauchement, lui, reste exact.
const MAX_DECLUTTER_MS := 30.0

var _failures := 0
var _measure := false


func _init() -> void:
	_measure = CmdArgs.has("--measure")
	await process_frame
	_test_synthetic()
	await _test_map()
	if _failures == 0:
		print("da7d OK")
	quit(1 if _failures > 0 else 0)


func _check(condition: bool, message: String) -> bool:
	if not condition:
		_failures += 1
		push_error("da7d: " + message)
	return condition


func _test_synthetic() -> void:
	var placer := MarkerDeclutter.new()
	placer.reset(32.0)
	_check(placer.try_place(Rect2(100, 100, 40, 40), 0), "first rect placed")
	_check(not placer.try_place(Rect2(120, 120, 40, 40), 1), "overlapping lower-priority rect yields")
	_check(placer.try_place(Rect2(141, 100, 40, 40), 2), "touching-free rect placed")
	_check(placer.try_place(Rect2(110, 110, 10, 10), 3, true), "pinned rect always placed")
	_check(placer.overlaps(Rect2(100, 90, 40, 12)), "rect over the first one overlaps")
	_check(not placer.overlaps(Rect2(100, 90, 40, 12), 0), "own rects are ignored for their owner")
	_check(not placer.overlaps(Rect2(100, 60, 40, 30)), "free space is free")
	_check(placer.placed_count() == 3, "three rects placed")
	# Grille : un grand nombre de rectangles, résultat identique à la recherche exhaustive.
	var rng := RandomNumberGenerator.new()
	rng.seed = 7
	var rects: Array[Rect2] = []
	for k in 600:
		rects.append(Rect2(rng.randf_range(-50, 1650), rng.randf_range(-50, 950), rng.randf_range(20, 60), rng.randf_range(16, 60)))
	placer.reset(64.0)
	var kept: Array[Rect2] = []
	for k in rects.size():
		var free := true
		for other in kept:
			if other.intersects(rects[k]):
				free = false
				break
		if free:
			kept.append(rects[k])
		_check(placer.try_place(rects[k], k) == free, "grid agrees with brute force (%d)" % k)
	_check(MarkerDeclutter.count_overlaps(placer.placed_rects()) == 0, "no overlap among placed rects")


func _test_map() -> void:
	root.size = Vector2i(1600, 900)
	var settings: Node = root.get_node_or_null("/root/Settings")
	if settings != null:
		settings.call("use_test_file")
		settings.call("set_value", "game/autosave_interval", 0, false)
		settings.call("set_value", "tutorial/enabled", false, false)
	var facade: Node = root.get_node("/root/SimFacade")
	facade.set_data_dir(MAP_PATHS.default_data_dir())
	facade.pending_faction = "fac_france"
	facade.pending_seed = 1337
	facade.pending_load_path = ""
	var map: Node3D = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	root.add_child(map)
	await process_frame
	await process_frame
	if not _check(map.load_ok and map.sim != null, "campaign map failed to start"):
		map.queue_free()
		return
	var layer: SettlementLayer = map.settlement_layer
	var camera: Camera3D = root.get_viewport().get_camera_3d()
	var total := 0
	print("da7d: viewport %s" % [root.get_visible_rect().size])
	print("da7d: region        distance  markers  labels  overlaps")
	for region: String in REGIONS:
		var at: Vector2 = REGIONS[region]
		for distance: float in DISTANCES:
			map.camera_rig.look_at_point(Vector3(at.x, map.map_data.surface_world_at(at.x, at.y), at.y), distance)
			map.camera_rig.snap()
			for f in 4:
				await process_frame
			for f2 in 2:  # UX1 : les plaques d'armée se replacent avec un léger retard.
				map.armies._update_plates()
				await process_frame
			layer.declutter()
			var occupancy := layer.screen_occupancy(camera)
			# Toutes les paires, y compris un marqueur et son propre nom.
			var overlaps := MarkerDeclutter.count_overlaps(occupancy["rects"])
			total += overlaps
			print("da7d: %-13s %8.0f %8d %7d %9d" % [region, distance, occupancy["markers"], occupancy["labels"], overlaps])
			if not _measure:
				_check(overlaps == 0, "%s at %.0f: %d overlapping pairs" % [region, distance, overlaps])
				# CV3-0 (#7) : les NOMS de colonies (lisibilité, le défaut rapporté : Saint-Denis,
				# Paris, compteur d'armée qui se chevauchaient) ne doivent pas chevaucher les
				# plaques/étendards d'armée. Un marqueur épinglé (capitale) peut toucher une
				# plaque d'armée toute proche (icônes, pas du texte) : hors de portée de ce lot.
				var army_rects: Array = map.armies.screen_label_rects(camera)
				var cross_overlaps := 0
				var occ_rects: Array = occupancy["rects"]
				var occ_kinds: Array = occupancy["kinds"]
				for k in occ_rects.size():
					if occ_kinds[k] != "label":
						continue
					for army_rect: Rect2 in army_rects:
						if (occ_rects[k] as Rect2).intersects(army_rect):
							cross_overlaps += 1
				_check(cross_overlaps == 0, "%s at %.0f: %d army/settlement label overlaps" % [region, distance, cross_overlaps])
				var capital := layer.capital_index()
				if region == "ile_de_france":
					_check(capital >= 0 and layer.marker_visible(capital), "player capital always shown (%.0f)" % distance)
	print("da7d: total overlapping pairs %d" % total)
	if not _measure:
		await _test_selection_pinned(map, layer)
	# Temps d'un recalcul complet (Flandre, palier province : le plus chargé).
	var flandre: Vector2 = REGIONS["flandre"]
	map.camera_rig.look_at_point(Vector3(flandre.x, 0.0, flandre.y), 330.0)
	map.camera_rig.snap()
	for f in 4:
		await process_frame
	# Meilleur de 5 séries de 10 (machine partagée : on borne le coût propre, pas la charge).
	var ms := INF
	for batch in 5:
		var t0 := Time.get_ticks_usec()
		for r in 10:
			layer.declutter()
		ms = minf(ms, float(Time.get_ticks_usec() - t0) / 1000.0 / 10.0)
	print("da7d: declutter %.3f ms per full pass" % ms)
	if not _measure:
		_check(ms < MAX_DECLUTTER_MS, "declutter too slow: %.2f ms" % ms)
	map.queue_free()
	await process_frame


## Une petite colonie sélectionnée à côté de Paris reste affichée (épinglée), à toute distance.
func _test_selection_pinned(map: Node3D, layer: SettlementLayer) -> void:
	var paris: Vector2 = REGIONS["ile_de_france"]
	var small := ""
	var data: SettlementData = layer.data
	for distance: float in [330.0, 1100.0]:
		map.camera_rig.look_at_point(Vector3(paris.x, 0.0, paris.y), distance)
		map.camera_rig.snap()
		for f in 3:
			await process_frame
		layer.declutter()
		if small == "":
			# Une colonie proche de Paris masquée par le dé-encombrement.
			for i in data.settlements.size():
				if layer.marker_in_tier(i) and not layer.marker_visible(i) and (data.settlements[i]["px"] as Vector2).distance_to(paris) < 80.0:
					small = str(data.settlements[i]["id"])
					break
			if not _check(small != "", "a hidden settlement near Paris at 330"):
				return
			layer.select(small)
			layer.declutter()
		var index: int = data.index_by_id[small]
		_check(layer.marker_visible(index), "selected %s stays shown at %.0f" % [small, distance])
		_check(layer.marker_visible(layer.capital_index()), "capital still shown with a selection at %.0f" % distance)
	layer.select("")
	layer.declutter()
