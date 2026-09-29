extends SceneTree

## Sonde du lot DC4 (ADR 0082, carte plus dense) : lisibilité et fluidité de la carte de campagne
## avec ~1 200 colonies. Fenêtre obligatoire (captures, temps d'image réels ; vsync coupée).
## Pour chaque vue (Europe, région, comté, près) centrée sur une zone dense : temps d'image
## (médiane, p95), temps d'un `declutter()`, marqueurs affichés à l'écran, paires de marqueurs qui
## se recouvrent à plus de moitié, étiquettes affichées et paires d'étiquettes qui se chevauchent
## (doit être 0), capture. Puis contrôles statiques : maquettes voisines qui se chevauchent après
## `_fit_models`, hameaux posés sur une colonie ; enfin un balayage au palier vallée (villes 1:1
## en streaming, ZG6) : temps d'image pendant un travelling sur la zone dense.
## Usage : godot --path game --script res://tests/dc4_density_probe.gd -- --out=<dossier> --tag=<nom>
##   [--views=1400,550,250,120] [--center=x,y] [--no-shots] [--no-valley]
## Sortie : une ligne `DC4_JSON {...}` ; code 1 si des étiquettes se chevauchent.

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")
const DEFAULT_VIEWS: Array[float] = [1400.0, 550.0, 250.0, 120.0]
const VIEW_NAMES := {1400: "europe", 550: "region", 250: "comte", 120: "pres", 20: "rapproche", 12: "vallee-haut"}
const SAMPLE_FRAMES := 90
const SETTLE_TIMEOUT_MS := 20000
const DECLUTTER_RUNS := 20
const VALLEY_DISTANCE := 6.0
const VALLEY_FRAMES := 360

var _result: Dictionary = {}
var _out := ""
var _tag := "probe"
var _shots := true
var _valley := true
## Point visé imposé (`--center=x,y`, pour comparer deux jeux de données), sinon zone la plus dense.
var _center := Vector2(-1.0, -1.0)


func _init() -> void:
	_run.call_deferred()


func _run() -> void:
	DisplayServer.window_set_vsync_mode(DisplayServer.VSYNC_DISABLED)
	Engine.max_fps = 0
	root.size = Vector2i(1600, 900)
	var views := DEFAULT_VIEWS.duplicate()
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--views="):
			views.clear()
			for token in arg.trim_prefix("--views=").split(","):
				views.append(float(token))
		elif arg.begins_with("--out="):
			_out = arg.trim_prefix("--out=")
		elif arg.begins_with("--tag="):
			_tag = arg.trim_prefix("--tag=")
		elif arg.begins_with("--center="):
			var xy := arg.trim_prefix("--center=").split(",")
			_center = Vector2(float(xy[0]), float(xy[1]))
		elif arg == "--no-shots":
			_shots = false
		elif arg == "--no-valley":
			_valley = false
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
	await process_frame
	var t_load := Time.get_ticks_msec()
	var map: Node3D = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	root.add_child(map)
	while not map.get("load_ok"):
		if Time.get_ticks_msec() - t_load > 90000:
			print("DC4_JSON ", JSON.stringify({"ok": false, "error": "map load timeout"}))
			quit(1)
			return
		await process_frame
	_result["map_load_ms"] = Time.get_ticks_msec() - t_load
	var layer: SettlementLayer = map.settlement_layer
	var vegetation: Vegetation = map.get_node("Vegetation")
	var terrain: TerrainBuilder = map.get("terrain")
	var rig: CampaignCamera = map.camera_rig
	var data: MapData = map.map_data
	var settled := func() -> bool: return vegetation.pending_jobs() == 0 and terrain.fine_ready()
	_result["settlements"] = layer.data.settlements.size()
	_result["hamlets"] = layer.data.hamlets.size()
	_result["towns_1to1"] = layer.towns.stats.get("towns", 0) if layer.towns != null else 0
	_result["ranks"] = _rank_histogram(layer)
	var center := _densest_point(layer, 40.0) if _center.x < 0.0 else _center
	_result["center"] = [snappedf(center.x, 0.1), snappedf(center.y, 0.1)]
	var surface_y := data.surface_world_at(center.x, center.y)
	await _settle(settled)
	var overlaps_total := 0
	var per_view: Dictionary = {}
	for d in views:
		rig.look_at_point(Vector3(center.x, surface_y, center.y), d)
		rig.snap()
		await _settle(settled)
		layer.flush()
		for i in 30:
			await process_frame
		var stats := await _measure(SAMPLE_FRAMES)
		var t0 := Time.get_ticks_usec()
		for k in DECLUTTER_RUNS:
			layer.declutter()
		stats["declutter_us"] = (Time.get_ticks_usec() - t0) / DECLUTTER_RUNS
		t0 = Time.get_ticks_usec()
		for k in DECLUTTER_RUNS:
			layer.pick_screen_scored(Vector2(800.0 + k, 450.0))
		stats["pick_us"] = (Time.get_ticks_usec() - t0) / DECLUTTER_RUNS
		stats.merge(_screen_stats(layer, map.camera))
		stats["models_shown"] = _shown_models(layer)  # DC6c : à l'échelle affichée de cette vue
		overlaps_total += int(stats["label_overlaps"])
		var view_name: String = VIEW_NAMES.get(int(d), str(int(d)))
		per_view[view_name] = stats
		if _shots and _out != "":
			await _shot(_out.path_join("dc4-%s-%s.png" % [_tag, view_name]))
	_result["views"] = per_view
	print("dc4_probe: views done")
	_result["models"] = _model_overlaps(layer)
	_result["hamlets_on_settlements"] = _hamlets_on_settlements(layer)
	print("dc4_probe: static checks done")
	if _valley:
		_result["valley"] = await _valley_sweep(layer, rig, data, center)
	_result["ok"] = overlaps_total == 0
	print("DC4_JSON ", JSON.stringify(_result))
	map.queue_free()
	await process_frame
	quit(0 if overlaps_total == 0 else 1)


## Rangs des marqueurs par type : {kind: {rank: count}}.
func _rank_histogram(layer: SettlementLayer) -> Dictionary:
	var result := {}
	for i in layer.data.settlements.size():
		var kind := str(layer.data.settlements[i]["kind"])
		if not result.has(kind):
			result[kind] = {}
		var rank := str(layer._marker_rank[i])
		result[kind][rank] = int(result[kind].get(rank, 0)) + 1
	return result


## Colonie (hors ville emblématique) ayant le plus de voisines dans `radius` unités.
func _densest_point(layer: SettlementLayer, radius: float) -> Vector2:
	var best := Vector2(2213, 3204)
	var best_count := -1
	var settlements: Array = layer.data.settlements
	for i in settlements.size():
		if layer._is_landmark(i):
			continue
		var p: Vector2 = settlements[i]["px"]
		var count := 0
		for j in settlements.size():
			if p.distance_squared_to(settlements[j]["px"]) < radius * radius:
				count += 1
		if count > best_count:
			best_count = count
			best = p
	_result["center_neighbours"] = best_count
	return best


func _screen_stats(layer: SettlementLayer, camera: Camera3D) -> Dictionary:
	var screen := camera.get_viewport().get_visible_rect()
	var icons: Array = []  # [centre, taille]
	for i in layer.data.settlements.size():
		if not layer.marker_visible(i):
			continue
		var px: Vector2 = layer.data.settlements[i]["px"]
		var world := Vector3(px.x, layer.map_data.surface_world_at(px.x, px.y) + 0.5, px.y)
		if camera.is_position_behind(world):
			continue
		var at := camera.unproject_position(world)
		if screen.has_point(at):
			icons.append([at, layer.marker_size(i)])
	var icon_overlaps := 0
	for a in icons.size():
		for b in range(a + 1, icons.size()):
			var limit := 0.5 * minf(icons[a][1], icons[b][1])
			if (icons[a][0] as Vector2).distance_to(icons[b][0]) < limit:
				icon_overlaps += 1
	var rects: Array[Rect2] = []
	for label in layer._labels:
		if label.visible:
			rects.append(LabelPlacer.label3d_screen_rect(label, camera, screen.size.y))
	var label_overlaps := 0
	for a in rects.size():
		for b in range(a + 1, rects.size()):
			if rects[a].intersects(rects[b]):
				label_overlaps += 1
	return {"markers_on_screen": icons.size(), "marker_overlaps": icon_overlaps, "labels": rects.size(), "label_overlaps": label_overlaps}


## Paires de maquettes (hors villes emblématiques) dont les emprises se recouvrent de plus de 20 %
## du plus petit rayon, après `_fit_models`.
func _model_overlaps(layer: SettlementLayer) -> Dictionary:
	var count := layer.data.settlements.size()
	var pairs := 0
	var worst := 0.0
	var examples: Array = []
	for i in count:
		if not _has_model(layer, i):
			continue
		for j in range(i + 1, count):
			if not _has_model(layer, j):
				continue
			var d := layer.model_px(i).distance_to(layer.model_px(j))
			var ri := layer.model_radius(i)
			var rj := layer.model_radius(j)
			var overlap := ri + rj - d
			if overlap > 0.2 * minf(ri, rj):
				pairs += 1
				worst = maxf(worst, overlap / minf(ri, rj))
				if examples.size() < 20:
					examples.append("%s%s/%s%s d=%.2f r=%.2f/%.2f" % [layer.data.settlements[i]["id"], "*" if layer._is_landmark(i) else "",
						layer.data.settlements[j]["id"], "*" if layer._is_landmark(j) else "", d, ri, rj])
	var hidden := 0
	var shrunk := 0
	for i in count:
		var holder: Node3D = layer._models[i]
		if holder != null and not layer._is_landmark(i):
			hidden += 0 if holder.visible else 1
			var fit: Variant = layer.get("_fit_scale")  # absent avant DC4 (mesures « avant »)
			shrunk += 1 if fit != null and float(fit[i]) < 0.999 else 0
	return {"overlapping_pairs": pairs, "worst_ratio": snappedf(worst, 0.01), "examples": examples, "absorbed": hidden, "shrunk": shrunk}


## DC6c : maquettes à l'échelle affichée de la vue courante (réduction DC4 × échelle SZ4b) :
## masquées, réduites, paires affichées qui se recouvrent à plus de 20 % du plus petit rayon.
func _shown_models(layer: SettlementLayer) -> Dictionary:
	var count := layer.data.settlements.size()
	var dc6c := layer.has_method("shown_radius")
	var radius := PackedFloat32Array()
	radius.resize(count)
	var hidden := 0
	var shrunk := 0
	for i in count:
		var holder: Node3D = layer._models[i]
		if holder == null:
			continue
		if layer._is_landmark(i):
			radius[i] = layer.model_radius(i)
			continue
		if not holder.visible:
			hidden += 1
			continue
		radius[i] = layer.call("shown_radius", i) if dc6c else layer.model_radius(i) * layer.model_scale(i)
		var fit: float = layer.call("shown_fit", i) if dc6c else float(layer._fit_scale[i])
		shrunk += 1 if fit < 0.999 else 0
	var pairs := 0
	for i in count:
		if radius[i] <= 0.0:
			continue
		for j in range(i + 1, count):
			if radius[j] <= 0.0:
				continue
			var overlap := radius[i] + radius[j] - layer.model_px(i).distance_to(layer.model_px(j))
			if overlap > 0.2 * minf(radius[i], radius[j]):
				pairs += 1
	return {"hidden": hidden, "shrunk": shrunk, "overlapping_pairs": pairs}


## Maquette affichée (générique ou ville emblématique) pour la colonie `i`.
func _has_model(layer: SettlementLayer, i: int) -> bool:
	var holder: Node3D = layer._models[i]
	return holder != null and holder.visible


## Hameaux dont la position de rendu tombe dans l'emprise d'une maquette de colonie.
func _hamlets_on_settlements(layer: SettlementLayer) -> Dictionary:
	var count := 0
	var examples: Array = []
	var cells := {}
	var cell := 16.0
	for i in layer.data.settlements.size():
		var p := layer.model_px(i)
		var key := Vector2i(floori(p.x / cell), floori(p.y / cell))
		if not cells.has(key):
			cells[key] = []
		cells[key].append(i)
	for h in layer.data.hamlets.size():
		var hp := layer.hamlet_px(h)
		var key := Vector2i(floori(hp.x / cell), floori(hp.y / cell))
		var hit := -1
		for dy in [-1, 0, 1]:
			for dx in [-1, 0, 1]:
				for i in cells.get(key + Vector2i(dx, dy), []):
					if hp.distance_to(layer.model_px(i)) < layer.model_radius(i) + ModelLibrary.HAMLET_SCALE * 0.3:
						hit = i
		if hit >= 0:
			count += 1
			if examples.size() < 8:
				examples.append("%s@%s" % [layer.data.hamlets[h]["name"], layer.data.settlements[hit]["id"]])
	return {"count": count, "examples": examples}


## Travelling au palier vallée sur la zone dense : villes 1:1 en streaming (ZG6).
func _valley_sweep(layer: SettlementLayer, rig: CampaignCamera, data: MapData, center: Vector2) -> Dictionary:
	var start := center - Vector2(30.0, 0.0)
	rig.look_at_point(Vector3(start.x, data.surface_world_at(start.x, start.y), start.y), VALLEY_DISTANCE)
	rig.snap()
	for i in 120:
		await process_frame
	var times: Array[float] = []
	var last := Time.get_ticks_usec()
	var max_loaded := 0
	for i in VALLEY_FRAMES:
		var p := start + Vector2(60.0 * float(i) / float(VALLEY_FRAMES - 1), 0.0)
		rig.target_focus = Vector3(p.x, data.surface_world_at(p.x, p.y), p.y)
		rig.target_distance = VALLEY_DISTANCE
		await process_frame
		var now := Time.get_ticks_usec()
		times.append((now - last) / 1000.0)
		last = now
		if layer.towns != null:
			max_loaded = maxi(max_loaded, int(layer.towns.stats.get("loaded", 0)))
	times.sort()
	# Coût CPU du tri des villes (streaming, finages) sur le fil principal, toutes les 10 images.
	var stream_us := 0
	var finage_us := 0
	if layer.towns != null:
		var t0 := Time.get_ticks_usec()
		for k in 10:
			layer.towns._stream(VALLEY_DISTANCE, center + Vector2(k, 0.0))
		stream_us = (Time.get_ticks_usec() - t0) / 10
		t0 = Time.get_ticks_usec()
		for k in 10:
			layer.towns._finage_key = ""
			layer.towns._push_finage(center + Vector2(k, 0.0))
		finage_us = (Time.get_ticks_usec() - t0) / 10
	var result := {
		"stream_us": stream_us,
		"finage_us": finage_us,
		"distance": rig.distance,
		"active": layer.towns != null and layer.towns.active,
		"median_ms": snappedf(times[times.size() / 2], 0.01),
		"p95_ms": snappedf(times[int(times.size() * 0.95)], 0.01),
		"max_ms": snappedf(times[-1], 0.01),
		"max_loaded_towns": max_loaded,
	}
	if _shots and _out != "":
		layer.flush()
		await _shot(_out.path_join("dc4-%s-vallee.png" % _tag))
	return result


func _settle(condition: Callable) -> void:
	var t0 := Time.get_ticks_msec()
	await process_frame
	while not condition.call() and Time.get_ticks_msec() - t0 < SETTLE_TIMEOUT_MS:
		await process_frame


func _measure(count: int) -> Dictionary:
	var times: Array[float] = []
	var last := Time.get_ticks_usec()
	for i in count:
		await process_frame
		var now := Time.get_ticks_usec()
		times.append((now - last) / 1000.0)
		last = now
	times.sort()
	return {
		"median_ms": snappedf(times[times.size() / 2], 0.01),
		"p95_ms": snappedf(times[int(times.size() * 0.95)], 0.01),
	}


func _shot(path: String) -> void:
	for i in 12:
		await process_frame
	var image := root.get_texture().get_image()
	print("dc4_probe: %s → %s" % [path, error_string(image.save_png(path))])
