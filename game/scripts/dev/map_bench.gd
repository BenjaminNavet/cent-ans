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
## SZ6 : `--bench-probe` attribue les pics aux sections de `PerfProbe` (`probe` dans le rapport).
## VT-I : `--bench-pan-only` s'arrête après le panoramique (mesure à une seule distance) ; le rapport
## donne aussi `startup_total_ms` (chargement de la carte) et `town_far` (statistiques du lointain).

## Étapes (x, y carte, distance) : Caen → Rouen → Paris → Chartres → Évreux, puis zoom sur Paris.
const PAN_PATH: Array[Vector2] = [
	Vector2(1880.0, 3125.0), Vector2(2097.0, 3099.0), Vector2(2213.0, 3204.0),
	Vector2(2150.0, 3285.0), Vector2(2080.0, 3170.0),
]
const WARMUP_FRAMES := 90
## ZG4 : lieux de la descente (x, y carte) : Rouen (zone E7), Grande Chartreuse (E4), Paris (E7).
const DESCENT_SITES: Array[Vector2] = [Vector2(2096.5, 3099.7), Vector2(2537.6, 3772.0), Vector2(2212.9, 3204.5)]
## ZG6 : `--bench-towns` descend plutôt sur des villes ordinaires rendues à l'échelle 1:1
## (Amiens, Troyes, Poitiers, Gand ; Chartres et Lincoln ne sont pas dans les données), pause
## allongée pour laisser la construction progressive se faire sous la caméra.
const TOWN_DESCENT_SITES: Array[Vector2] = [Vector2(2224.0, 3043.6), Vector2(2381.4, 3306.4), Vector2(1964.9, 3529.3), Vector2(2380.6, 2878.2)]
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
var _pan_only := false
var _descent_site := 0
var _descent_sites: Array[Vector2] = DESCENT_SITES
var _descent_hold := DESCENT_HOLD
var _descent_ms: PackedFloat32Array = PackedFloat32Array()
var _descent_t_start := 0
var _descent_us := 0
## ZG7a : attribution des pics de la descente (images > 50 ms) : temps des scripts de l'image,
## le reste étant rendu, attente GPU ou système. ZG7c : `Performance.TIME_PROCESS` ne couvrait pas
## l'image mesurée ; on chronomètre désormais du début de l'itération (nœud `FrameStart`, priorité
## minimale, première `_physics_process` ou `_process` de l'itération) à ce banc (traité en dernier).
static var frame_start_usec := 0
static var _frame_start_tag := -1
var _starter: Node
var _spike_process_ms: PackedFloat32Array = PackedFloat32Array()
var _spike_frame_ms: PackedFloat32Array = PackedFloat32Array()
var _process_ms_all: PackedFloat32Array = PackedFloat32Array()


## Marque le début des scripts de l'itération (physique comprise) pour `process_ms`.
class FrameStart:
	extends Node

	func _physics_process(_delta: float) -> void:
		_mark()

	func _process(_delta: float) -> void:
		_mark()

	func _mark() -> void:
		var tag := Engine.get_process_frames()
		if MapBench._frame_start_tag != tag:
			MapBench._frame_start_tag = tag
			MapBench.frame_start_usec = Time.get_ticks_usec()


var _bench_sets: PackedStringArray = []
## PF `--bench-ab=base;noshadow;tparam:pf_skip=4` : configurations (listes `--bench-set`) alternées
## toutes les `AB_PERIOD` images pendant le panoramique, dans le même processus (même charge
## machine pour toutes) ; médianes par configuration dans le rapport (`ab`).
const AB_PERIOD := 12
const AB_SETTLE := 4
var _ab_configs: PackedStringArray = []
var _ab_ms: Dictionary = {}
var _ab_current := -1
var _ab_since := 0
var _ab_defaults: Dictionary = {}


func _ready() -> void:
	_starter = FrameStart.new()
	_starter.name = "MapBenchFrameStart"
	_starter.process_priority = -1000000
	_starter.process_physics_priority = -1000000
	get_tree().root.add_child.call_deferred(_starter)
	process_priority = 1000000
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--bench-distance="):
			pan_distance = float(arg.trim_prefix("--bench-distance="))
		elif arg.begins_with("--bench-seconds="):
			pan_seconds = float(arg.trim_prefix("--bench-seconds="))
		elif arg == "--bench-descent-only":
			_descent_only = true
		elif arg == "--bench-pan-only":
			_pan_only = true
		elif arg == "--bench-towns":
			_descent_sites = TOWN_DESCENT_SITES
			_descent_hold = TOWN_DESCENT_HOLD
	if OS.get_cmdline_user_args().has("--bench-listeners"):
		_wrap_listeners.call_deferred()
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--bench-hide="):  # PF : coût d'une couche dans le scénario du banc
			_hide_layers.call_deferred(arg.trim_prefix("--bench-hide=").split(","))
		elif arg.begins_with("--bench-set="):
			_bench_sets = arg.trim_prefix("--bench-set=").split(",")
		elif arg.begins_with("--bench-ab="):
			_ab_configs = arg.trim_prefix("--bench-ab=").split(";")
	PerfProbe.enabled = OS.get_cmdline_user_args().has("--bench-probe")
	DisplayServer.window_set_vsync_mode(DisplayServer.VSYNC_DISABLED)
	Engine.max_fps = 0
	camera_rig.edge_pan_enabled = false
	RenderingServer.viewport_set_measure_render_time(get_viewport().get_viewport_rid(), true)
	_place(PAN_PATH[0], pan_distance)


## PF `--bench-set=scale:0.25,noshadow,relief_cast:1,tparam:nom=valeur` : réglages de rendu
## imposés à chaque image du banc (échelle 3D, ombres du soleil, cascades du relief, paramètre du
## shader du terrain).
func _apply_bench_sets() -> void:
	for item in _bench_sets:
		if item.begins_with("scale:"):
			get_viewport().scaling_3d_scale = float(item.trim_prefix("scale:"))
		elif item == "noshadow":
			var sun := get_parent().find_child("Sun", true, false) as DirectionalLight3D
			if sun != null:
				sun.shadow_enabled = false
		elif item.begins_with("relief_cast:"):
			terrain.relief_shadow_override = int(item.trim_prefix("relief_cast:"))
		elif item.begins_with("hide:"):
			var node := get_parent().find_child(item.trim_prefix("hide:"), true, false)
			if node != null and "visible" in node:
				if not _ab_defaults.is_empty():
					_ab_defaults["hidden"][node] = true
				node.set("visible", false)
		elif item == "msaa_off":
			get_viewport().msaa_3d = Viewport.MSAA_DISABLED
		elif item.begins_with("terrain:"):
			var kv := item.trim_prefix("terrain:").split("=")
			if not _ab_defaults.is_empty() and not (_ab_defaults["terrain"] as Dictionary).has(kv[0]):
				_ab_defaults["terrain"][kv[0]] = terrain.get(kv[0])
			terrain.set(kv[0], kv[1] == "true" if kv[1] in ["true", "false"] else (float(kv[1]) if "." in kv[1] else int(kv[1])))
		elif item.begins_with("qt:") and terrain.quadtree != null:
			var kv := item.trim_prefix("qt:").split("=")
			if not _ab_defaults.is_empty() and not (_ab_defaults["qt"] as Dictionary).has(kv[0]):
				_ab_defaults["qt"][kv[0]] = terrain.quadtree.get(kv[0])
			terrain.quadtree.set(kv[0], float(kv[1]) if "." in kv[1] else int(kv[1]))
		elif item.begins_with("tparam:") and terrain.material != null:
			var kv := item.trim_prefix("tparam:").split("=")
			if not _ab_defaults.is_empty() and not (_ab_defaults["params"] as Dictionary).has(kv[0]):
				_ab_defaults["params"][kv[0]] = terrain.material.get_shader_parameter(kv[0])
			terrain.material.set_shader_parameter(kv[0], float(kv[1]) if "." in kv[1] else int(kv[1]))


## PF : une image de l'A/B : temps de l'image précédente rangé sous la configuration courante
## (hors `AB_SETTLE` images après une bascule), puis bascule toutes les `AB_PERIOD` images.
func _ab_tick(now: int) -> void:
	if _ab_current >= 0 and _ab_since >= AB_SETTLE:
		var key := _ab_configs[_ab_current]
		var values: PackedFloat32Array = _ab_ms.get(key, PackedFloat32Array())
		values.append((now - _last_us) / 1000.0)
		_ab_ms[key] = values  # tableau compacté : copie, réécrite
	_ab_since += 1
	if _ab_current < 0 or _ab_since >= AB_PERIOD:
		_ab_current = (_ab_current + 1) % _ab_configs.size()
		_ab_since = 0
		_restore_bench_sets()
		_bench_sets = _ab_configs[_ab_current].split(",")
	_apply_bench_sets()


func _restore_bench_sets() -> void:
	if _ab_defaults.is_empty():
		var sun := get_parent().find_child("Sun", true, false) as DirectionalLight3D
		_ab_defaults = {"scale": get_viewport().scaling_3d_scale, "shadow": sun.shadow_enabled if sun != null else true, "params": {}, "qt": {}, "hidden": {}, "terrain": {}, "msaa": get_viewport().msaa_3d}
	get_viewport().scaling_3d_scale = float(_ab_defaults["scale"])
	var sun := get_parent().find_child("Sun", true, false) as DirectionalLight3D
	if sun != null:
		sun.shadow_enabled = bool(_ab_defaults["shadow"])
	terrain.relief_shadow_override = 0
	var params: Dictionary = _ab_defaults["params"]
	for name: String in params:
		terrain.material.set_shader_parameter(name, params[name])
	get_viewport().msaa_3d = _ab_defaults["msaa"]
	for node: Node in _ab_defaults["hidden"]:
		if is_instance_valid(node):
			node.set("visible", true)
	_ab_defaults["hidden"] = {}
	var props: Dictionary = _ab_defaults["terrain"]
	for name: String in props:
		terrain.set(name, props[name])
	var qt: Dictionary = _ab_defaults["qt"]
	for name: String in qt:
		terrain.quadtree.set(name, qt[name])


## PF `--bench-hide=Rivers,Terrain` : masque ces nœuds de la carte (recherche par nom) pendant le
## banc, pour chiffrer une couche dans le scénario réel.
func _hide_layers(names: PackedStringArray) -> void:
	for layer_name in names:
		var node := get_parent().find_child(layer_name, true, false)
		if node != null and "visible" in node:
			node.set("visible", false)
		else:
			push_warning("bench-hide: %s introuvable" % layer_name)


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


## SZ6 `--bench-probe` : par section de `PerfProbe`, temps cumulé dans les pics (> 50 ms), nombre
## de pics où elle domine, pire durée sur tout le parcours ; les pires images et leurs sections.
var _probe_spike_ms: Dictionary = {}
var _probe_top: Dictionary = {}
var _probe_max_ms: Dictionary = {}
var _probe_worst: Array = []
## RS-K : durées (ms) par section et par image mesurée (0 si absente) → médiane, p95, p99.
var _probe_samples: Dictionary = {}
var _probe_frames := 0


func _probe_frame(now: int) -> void:
	var sections := PerfProbe.take_frame()
	if _phase == "warmup" or _phase == "done" or _last_us == 0:
		return
	var frame_ms := (now - _last_us) / 1000.0
	var process_ms := (now - frame_start_usec) / 1000.0 if frame_start_usec > 0 and frame_start_usec <= now else 0.0
	var attributed := 0
	for label: String in sections:
		if not label.contains("/"):  # sous-sections (« a/b ») déjà comptées dans leur section
			attributed += int(sections[label])
	sections["(unattributed)"] = maxi(0, int(process_ms * 1000.0) - attributed)
	for label: String in sections:
		if not _probe_samples.has(label):
			var series: Array = []
			series.resize(_probe_frames)
			series.fill(0.0)  # section absente des images précédentes
			_probe_samples[label] = series
	for label: String in _probe_samples:
		(_probe_samples[label] as Array).append(int(sections.get(label, 0)) / 1000.0)
	_probe_frames += 1
	var top := ""
	var top_ms := 0.0
	for label: String in sections:
		var ms := int(sections[label]) / 1000.0
		_probe_max_ms[label] = maxf(float(_probe_max_ms.get(label, 0.0)), ms)
		if ms > top_ms and not label.contains("/"):
			top_ms = ms
			top = label
	if frame_ms <= 50.0:
		return
	for label: String in sections:
		_probe_spike_ms[label] = float(_probe_spike_ms.get(label, 0.0)) + int(sections[label]) / 1000.0
	_probe_top[top] = int(_probe_top.get(top, 0)) + 1
	var parts: Array = []
	for label: String in sections:
		parts.append([int(sections[label]) / 1000.0, label])
	parts.sort_custom(func(a: Array, b: Array) -> bool: return a[0] > b[0])
	var summary := {"frame_ms": snappedf(frame_ms, 0.1), "process_ms": snappedf(process_ms, 0.1), "phase": _phase}
	for part: Array in parts.slice(0, 5):
		summary[part[1]] = snappedf(part[0], 0.1)
	_probe_worst.append(summary)
	_probe_worst.sort_custom(func(a: Dictionary, b: Dictionary) -> bool: return a["frame_ms"] > b["frame_ms"])
	if _probe_worst.size() > 12:
		_probe_worst.resize(12)


func _probe_report() -> Dictionary:
	var rows: Array = []
	for label: String in _probe_max_ms:
		rows.append([float(_probe_spike_ms.get(label, 0.0)), label])
	rows.sort_custom(func(a: Array, b: Array) -> bool: return a[0] > b[0])
	var sections := {}
	for row: Array in rows:
		sections[row[1]] = {"spike_ms": snappedf(row[0], 0.1), "top": int(_probe_top.get(row[1], 0)), "max_ms": snappedf(float(_probe_max_ms[row[1]]), 0.1)}
		var series: Array = _probe_samples.get(row[1], [])
		if not series.is_empty():
			series.sort()
			var n := series.size()
			sections[row[1]]["mean_ms"] = snappedf(_sum(series) / n, 0.01)
			sections[row[1]]["p50_ms"] = snappedf(series[n / 2], 0.01)
			sections[row[1]]["p95_ms"] = snappedf(series[mini(n - 1, int(n * 0.95))], 0.01)
			sections[row[1]]["p99_ms"] = snappedf(series[mini(n - 1, int(n * 0.99))], 0.01)
	return {"sections": sections, "worst": _probe_worst}


static func _sum(values: Array) -> float:
	var total := 0.0
	for v in values:
		total += v
	return total


func _place(p: Vector2, distance: float) -> void:
	camera_rig.target_focus = Vector3(p.x, map_data.surface_world_at(p.x, p.y), p.y)
	camera_rig.target_distance = distance
	camera_rig.snap()


func _process(delta: float) -> void:
	_frame += 1
	if not _ab_configs.is_empty() and _phase == "pan":
		_ab_tick(Time.get_ticks_usec())
	elif not _bench_sets.is_empty():
		_apply_bench_sets()
	var now := Time.get_ticks_usec()
	if PerfProbe.enabled:
		_probe_frame(now)
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
				if _pan_only:
					_report(now)
					_phase = "done"
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
			# Scripts de cette itération (physique comprise) jusqu'à ce banc : ils tombent dans
			# l'intervalle mesuré (fin des scripts de l'image précédente → maintenant).
			var process_ms := (now - frame_start_usec) / 1000.0 if frame_start_usec > 0 and frame_start_usec <= now else 0.0
			_process_ms_all.append(process_ms)
			if (now - _last_us) / 1000.0 > 50.0:
				_spike_frame_ms.append((now - _last_us) / 1000.0)
				_spike_process_ms.append(process_ms)
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


func _ab_report() -> Dictionary:
	var out := {}
	for key: String in _ab_ms:
		var values: PackedFloat32Array = _ab_ms[key]
		out[key] = {"median_ms": _median(values), "n": values.size()}
	return out


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
		var process_dominated := 0
		for k in _spike_frame_ms.size():
			if _spike_process_ms[k] > _spike_frame_ms[k] * 0.5:
				process_dominated += 1
		var sorted_p := _process_ms_all.duplicate()
		sorted_p.sort()
		descent = {
			"frames": n, "fps_avg": snappedf(n / maxf(_descent_us / 1000000.0, 0.001), 0.1),
			"frame_ms_p50": snappedf(sorted_d[n / 2], 0.01), "frame_ms_p99": snappedf(sorted_d[int(n * 0.99)], 0.01),
			"frame_ms_max": snappedf(sorted_d[n - 1], 0.01), "spikes_over_50ms": spikes_d,
			"process_ms_p50": _median(_process_ms_all), "process_ms_p99": snappedf(sorted_p[int(sorted_p.size() * 0.99)], 0.01) if not sorted_p.is_empty() else 0.0,
			"spike_process_ms_p50": _median(_spike_process_ms), "spikes_process_dominated": process_dominated,
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
		"ab": _ab_report(),
		"window": DisplayServer.window_get_size(),
		"scale_3d": get_viewport().scaling_3d_scale,
		"scale_mode": get_viewport().scaling_3d_mode,
		"quadtree": terrain.quadtree != null,
		"render_cpu_ms_p50": _median(_cpu_render_ms),
		"primitives_p50": _median(_primitives),
		"draw_calls_p50": _median(_draw_calls),
	}
	var campaign := terrain.get_parent()
	if campaign != null and "startup_stats" in campaign:
		report["startup_total_ms"] = (campaign.get("startup_stats") as Dictionary).get("total_ms", 0)
	var layer: Node = campaign.get("settlement_layer") if campaign != null and "settlement_layer" in campaign else null
	if layer != null and "town_far" in layer and layer.get("town_far") != null:
		report["town_far"] = (layer.get("town_far") as Node).get("stats")
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
	if PerfProbe.enabled:
		report["probe"] = _probe_report()
	print("CampaignMap: bench_map %s" % JSON.stringify(report))
	get_tree().quit()


func _exit_tree() -> void:
	PerfProbe.enabled = false
	if is_instance_valid(_starter):
		_starter.queue_free()
