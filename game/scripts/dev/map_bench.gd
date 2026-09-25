class_name MapBench
extends Node

## Banc de la carte de campagne (lot ZG2, `--bench-map`) : panoramique puis zoom scriptés
## au-dessus de la Normandie et de l'Île-de-France, en fenêtré (le GPU compte), synchronisation
## verticale coupée. Mesure la durée réelle de chaque image (i/s moyen, 1 % des pires, pire image,
## images > 50 ms) et, si le quadtree de relief est actif, ses statistiques (nœuds, pages,
## décodage, téléversement). Imprime `CampaignMap: bench_map {...}` puis quitte.
##
## Options : `--bench-distance=N` (distance du panoramique, 30 par défaut), `--bench-seconds=N`
## (durée du panoramique, 20 s), `--camera-min=N` (distance minimale de la caméra, essais seulement).
## Lot ZG4 : parcours « descente » ensuite (`descent` dans le rapport) : au-dessus de Rouen, de la
## Grande Chartreuse et de Paris, stratégique (150) → vallée (5) → site (distance minimale du lieu)
## en `DESCENT_SECONDS` (logarithme de la distance), pause, remontée ; `--bench-descent-only` saute
## panoramique et zoom. Rapporte aussi les recalages d'échelle verticale et les cuissons des
## maquettes.

## Étapes (x, y carte, distance) : Caen → Rouen → Paris → Chartres → Évreux, puis zoom sur Paris.
const PAN_PATH: Array[Vector2] = [
	Vector2(1880.0, 1845.0), Vector2(2097.0, 1819.0), Vector2(2213.0, 1924.0),
	Vector2(2150.0, 2005.0), Vector2(2080.0, 1890.0),
]
const WARMUP_FRAMES := 90
## ZG4 : lieux de la descente (x, y carte) : Rouen (zone E7), Grande Chartreuse (E4), Paris (E7).
const DESCENT_SITES: Array[Vector2] = [Vector2(2096.5, 1819.7), Vector2(2537.6, 2492.0), Vector2(2212.9, 1924.5)]
## ZG6 : `--bench-towns` descend plutôt sur des villes ordinaires rendues à l'échelle 1:1
## (Amiens, Troyes, Poitiers, Gand ; Chartres et Lincoln ne sont pas dans les données), pause
## allongée pour laisser la construction progressive se faire sous la caméra.
const TOWN_DESCENT_SITES: Array[Vector2] = [Vector2(2224.0, 1763.6), Vector2(2381.4, 2026.4), Vector2(1964.9, 2249.3), Vector2(2380.6, 1598.2)]
const TOWN_DESCENT_HOLD := 4.0
const DESCENT_SECONDS := 6.0
const DESCENT_HOLD := 1.5

var camera_rig: CampaignCamera
var terrain: TerrainBuilder
var map_data: MapData
var pan_distance: float = 30.0
var pan_seconds: float = 20.0

var _frame := 0
var _t_start := 0
var _last_us := 0
var _frame_ms: PackedFloat32Array = PackedFloat32Array()
var _phase := "warmup"
var _phase_t := 0.0
var _cpu_render_ms: PackedFloat32Array = PackedFloat32Array()
var _primitives: PackedFloat32Array = PackedFloat32Array()
var _draw_calls: PackedFloat32Array = PackedFloat32Array()
var _descent_only := false
var _descent_site := 0
var _descent_sites: Array[Vector2] = DESCENT_SITES
var _descent_hold := DESCENT_HOLD
var _descent_ms: PackedFloat32Array = PackedFloat32Array()
var _descent_t_start := 0
var _descent_us := 0


func _ready() -> void:
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--bench-distance="):
			pan_distance = float(arg.trim_prefix("--bench-distance="))
		elif arg.begins_with("--bench-seconds="):
			pan_seconds = float(arg.trim_prefix("--bench-seconds="))
		elif arg == "--bench-descent-only":
			_descent_only = true
		elif arg == "--bench-towns":
			_descent_sites = TOWN_DESCENT_SITES
			_descent_hold = TOWN_DESCENT_HOLD
	if OS.get_cmdline_user_args().has("--bench-listeners"):
		_wrap_listeners.call_deferred()
	DisplayServer.window_set_vsync_mode(DisplayServer.VSYNC_DISABLED)
	Engine.max_fps = 0
	camera_rig.edge_pan_enabled = false
	RenderingServer.viewport_set_measure_render_time(get_viewport().get_viewport_rid(), true)
	_place(PAN_PATH[0], pan_distance)


## `--bench-listeners` : mesure le temps passé dans chaque écouteur de `chunk_surface_changed`.
var _listener_ms: Dictionary = {}


func _wrap_listeners() -> void:
	for connection: Dictionary in terrain.chunk_surface_changed.get_connections():
		var callable: Callable = connection["callable"]
		var label := "%s.%s" % [str(callable.get_object().get_script().resource_path.get_file()) if callable.get_object() != null and callable.get_object().get_script() != null else "?", callable.get_method()]
		terrain.chunk_surface_changed.disconnect(callable)
		terrain.chunk_surface_changed.connect(func(index: int) -> void:
			var t0 := Time.get_ticks_usec()
			callable.call(index)
			_listener_ms[label] = float(_listener_ms.get(label, 0.0)) + (Time.get_ticks_usec() - t0) / 1000.0)


func _place(p: Vector2, distance: float) -> void:
	camera_rig.target_focus = Vector3(p.x, map_data.surface_world_at(p.x, p.y), p.y)
	camera_rig.target_distance = distance
	camera_rig.snap()


func _process(delta: float) -> void:
	_frame += 1
	var now := Time.get_ticks_usec()
	match _phase:
		"warmup":
			if _frame >= WARMUP_FRAMES and (terrain.fine_ready() or _frame > WARMUP_FRAMES * 8):
				_phase = "descent" if _descent_only else "pan"
				_phase_t = 0.0
				_t_start = now
				_descent_t_start = now
		"pan":
			_frame_ms.append((now - _last_us) / 1000.0)
			_sample()
			_phase_t += delta
			var t := clampf(_phase_t / pan_seconds, 0.0, 1.0) * (PAN_PATH.size() - 1)
			var i := mini(int(t), PAN_PATH.size() - 2)
			_place(PAN_PATH[i].lerp(PAN_PATH[i + 1], t - i), pan_distance)
			if _phase_t >= pan_seconds:
				_phase = "zoom"
				_phase_t = 0.0
		"zoom":
			_frame_ms.append((now - _last_us) / 1000.0)
			_sample()
			_phase_t += delta
			# Aller-retour de 150 à la distance minimale au-dessus de Paris en 8 s.
			var u := 0.5 - 0.5 * cos(_phase_t / 8.0 * TAU)
			_place(PAN_PATH[2], lerpf(150.0, camera_rig.min_distance_at(Vector3(PAN_PATH[2].x, 0.0, PAN_PATH[2].y)), u))
			if _phase_t >= 8.0:
				_phase = "descent"
				_phase_t = 0.0
				_descent_t_start = now
		"descent":
			_descent_ms.append((now - _last_us) / 1000.0)
			_frame_ms.append((now - _last_us) / 1000.0)
			_sample()
			_phase_t += delta
			var site := _descent_sites[_descent_site]
			var lowest := camera_rig.min_distance_at(Vector3(site.x, 0.0, site.y))
			# Descente en logarithme de la distance (150 → 5 → minimum), pause, remontée.
			var u := 1.0
			if _phase_t < DESCENT_SECONDS:
				u = smoothstep(0.0, 1.0, _phase_t / DESCENT_SECONDS)
			elif _phase_t >= DESCENT_SECONDS + _descent_hold:
				u = 1.0 - smoothstep(0.0, 1.0, (_phase_t - DESCENT_SECONDS - _descent_hold) / (DESCENT_SECONDS * 0.5))
			_place(site, exp(lerpf(log(150.0), log(lowest), u)))
			if _phase_t >= DESCENT_SECONDS * 1.5 + _descent_hold:
				_descent_site += 1
				_phase_t = 0.0
				if _descent_site >= _descent_sites.size():
					_descent_us = now - _descent_t_start
					_report(now)
					_phase = "done"
	_last_us = now


## Coût CPU du rendu (mesuré par le serveur de rendu, image précédente ; la mesure GPU vaut 0 sous
## Metal), primitives et appels de dessin.
func _sample() -> void:
	var rid := get_viewport().get_viewport_rid()
	_cpu_render_ms.append(RenderingServer.viewport_get_measured_render_time_cpu(rid))
	_primitives.append(Performance.get_monitor(Performance.RENDER_TOTAL_PRIMITIVES_IN_FRAME))
	_draw_calls.append(Performance.get_monitor(Performance.RENDER_TOTAL_DRAW_CALLS_IN_FRAME))


static func _median(values: PackedFloat32Array) -> float:
	if values.is_empty():
		return 0.0
	var sorted := values.duplicate()
	sorted.sort()
	return snappedf(sorted[sorted.size() / 2], 0.01)


func _report(now: int) -> void:
	var sorted := _frame_ms.duplicate()
	sorted.sort()
	var count := sorted.size()
	var spikes := 0
	for ms in sorted:
		if ms > 50.0:
			spikes += 1
	var seconds := (now - _t_start) / 1000000.0
	var descent := {}
	if not _descent_ms.is_empty():
		var sorted_d := _descent_ms.duplicate()
		sorted_d.sort()
		var n := sorted_d.size()
		var spikes_d := 0
		for ms in sorted_d:
			if ms > 50.0:
				spikes_d += 1
		descent = {
			"frames": n, "fps_avg": snappedf(n / maxf(_descent_us / 1000000.0, 0.001), 0.1),
			"frame_ms_p50": snappedf(sorted_d[n / 2], 0.01), "frame_ms_p99": snappedf(sorted_d[int(n * 0.99)], 0.01),
			"frame_ms_max": snappedf(sorted_d[n - 1], 0.01), "spikes_over_50ms": spikes_d,
		}
	var bakes := 0
	var bake_frame_max := 0.0
	var bake_total := 0.0
	for node in get_tree().root.find_children("Landmark_*", "LandmarkModel", true, false):
		var landmark := node as LandmarkModel
		bakes += int(landmark.stats.get("bakes", 0))
		bake_frame_max = maxf(bake_frame_max, float(landmark.stats.get("bake_frame_ms_max", 0.0)))
		bake_total += float(landmark.stats.get("bake_ms_total", 0.0))
	var towns := {}
	var town_layer := get_tree().root.find_child("Towns", true, false) as TownLayer
	if town_layer != null:
		towns = town_layer.stats.duplicate()
	var report := {
		"descent": descent,
		"towns": towns,
		"landmark_bakes": bakes, "landmark_bake_frame_ms_max": snappedf(bake_frame_max, 0.01),
		"landmark_bake_ms_total": snappedf(bake_total, 0.01),
		"frames": count,
		"fps_avg": snappedf(count / maxf(seconds, 0.001), 0.1),
		"frame_ms_p50": snappedf(sorted[count / 2], 0.01) if count > 0 else 0.0,
		"frame_ms_p99": snappedf(sorted[int(count * 0.99)], 0.01) if count > 0 else 0.0,
		"frame_ms_max": snappedf(sorted[count - 1], 0.01) if count > 0 else 0.0,
		"spikes_over_50ms": spikes,
		"pan_distance": pan_distance,
		"viewport": get_viewport().get_visible_rect().size,
		"quadtree": terrain.quadtree != null,
		"render_cpu_ms_p50": _median(_cpu_render_ms),
		"primitives_p50": _median(_primitives),
		"draw_calls_p50": _median(_draw_calls),
	}
	if terrain.quadtree != null:
		report.merge(terrain.quadtree.perf_stats())
	# ZG5b : réseau fin (mise à jour par image, maillages, pages creusées).
	var rivers := terrain.get_parent().get_node_or_null("Rivers") as RiversRenderer if terrain.get_parent() != null else null
	if rivers != null and rivers.fine != null:
		report.merge(rivers.fine.perf_stats())
	for key in ["surface_emits", "surface_page_emits", "surface_emit_ms_max", "surface_emit_ms_total", "qt_update_ms_max",
			"vertical_rescales", "vertical_signal_ms_max", "rescale_emits", "rescale_ms_total", "rescale_ms_max"]:
		if terrain.build_stats.has(key):
			report[key] = snappedf(float(terrain.build_stats[key]), 0.01)
	if not _listener_ms.is_empty():
		report["listener_ms"] = _listener_ms
	print("CampaignMap: bench_map %s" % JSON.stringify(report))
	get_tree().quit()
