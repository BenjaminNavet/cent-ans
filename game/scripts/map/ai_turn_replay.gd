class_name AiTurnReplay
extends Node

## Lot CT1 — relecture du tour de l'IA à la Total War (ADR 0073).
##
## Le cœur résout tout le tour de l'IA dans `end_turn` ; quand l'enregistrement est actif
## (`set_ai_turn_recording`), il rend chaque mouvement d'armée IA (`get_ai_turn_moves` : trajet
## réel, issue, visibilité pour le joueur, intérêt). Ce nœud ne fait que les rejouer, entre la
## résolution et le rapport de saison :
##
## - « Suivre » : les armées IA vues marchent le long de leur trajet ; la caméra se porte sur les
##   mouvements qui concernent le joueur (bataille, siège, son territoire, ses armées ou colonies :
##   `max_followed_moves` au plus, les plus importants), puis revient où elle était ;
## - « Montrer » : les marches seules, sans bouger la caméra ;
## - « Masquer » : rien d'enregistré ni de rejoué (coût nul, comme avant CT1).
##
## Vitesse ×1 / ×2 / ×4 (Réglages), Espace passe le reste. Mise en scène réglée dans
## `data/ui/ai_turn_replay.json` (schéma `ai_turn_replay.schema.json`). Sans écran (tests
## headless), le mode est « Masquer » sauf si `allow_headless` est vrai.

signal replay_started(shown: int, followed: int)
signal replay_finished

const DATA_PATH := "ui/ai_turn_replay.json"
const MAP_PATHS_SCRIPT := preload("res://scripts/map/map_paths.gd")
const MODES: Array[String] = ["follow", "show", "hide"]
const MODE_LABELS: Array[String] = ["Suivre", "Montrer", "Masquer"]
const MODE_KEY := "map/ai_moves"
const SPEED_KEY := "map/ai_moves_speed"
## Réglages par défaut si le fichier manque (jeux de données réduits des tests).
const FALLBACK := {
	"max_followed_moves": 6, "notable_radius_km": 60.0, "march_speed_px_per_s": 55.0,
	"min_move_duration_s": 0.8, "max_move_duration_s": 3.0, "camera_pan_duration_s": 0.7,
	"camera_hold_s": 0.5, "camera_follow_distance_share": 0.05, "camera_return_duration_s": 0.6,
	"background_batch_duration_s": 2.0, "speeds": [1.0, 2.0, 4.0],
}
const HOLD_KINDS := ["battle", "siege_started", "settlement_taken", "landing"]

## Tests : rejouer même sans écran.
static var allow_headless := false
static var _tuning: Dictionary = {}

var map: Node = null
var playing := false
## Dernière relecture : {mode, moves, shown, followed, skipped, record_ms, replay_ms}.
var last_stats: Dictionary = {}
var _skip := false
var _jobs: Dictionary = {}  # army_id → {points, elapsed, duration}
var _follow_army := ""
var _caption: PanelContainer = null
var _caption_label: Label = null


static func tuning() -> Dictionary:
	if _tuning.is_empty():
		var path := _data_dir().path_join(DATA_PATH)
		if not FileAccess.file_exists(path):
			path = MAP_PATHS_SCRIPT.project_root().path_join("data").path_join(DATA_PATH)
		if FileAccess.file_exists(path):
			var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(path))
			if parsed is Dictionary:
				_tuning = FALLBACK.duplicate()
				_tuning.merge(parsed, true)
		if _tuning.is_empty():
			push_warning("AiTurnReplay: %s missing or invalid" % path)
			_tuning = FALLBACK.duplicate()
	return _tuning


static func _data_dir() -> String:
	var tree := Engine.get_main_loop() as SceneTree
	if tree != null and tree.root != null:
		var map_paths := tree.root.get_node_or_null("MapPaths")
		if map_paths != null:
			return str(map_paths.get("data_dir"))
	return MAP_PATHS_SCRIPT.default_data_dir()


## Vitesses proposées dans les Réglages (flottants).
static func speeds() -> Array:
	return Array(tuning().get("speeds", [1.0, 2.0, 4.0])).map(func(value: Variant) -> float: return float(value))


func setup(campaign_map: Node) -> void:
	map = campaign_map
	set_process(false)


func _setting(key: String, fallback: Variant) -> Variant:
	var settings := get_node_or_null("/root/Settings")
	return settings.call("get_value", key) if settings != null else fallback


## Mode effectif : « hide » sans écran (sauf tests), sinon le réglage.
func mode() -> String:
	if DisplayServer.get_name() == "headless" and not allow_headless:
		return "hide"
	var value := str(_setting(MODE_KEY, "follow"))
	return value if MODES.has(value) else "follow"


func speed() -> float:
	return maxf(float(_setting(SPEED_KEY, 1.0)), 0.1)


## Avant `end_turn` : le cœur n'enregistre les mouvements que s'ils seront rejoués.
func before_end_turn() -> void:
	var sim: Object = map.get("sim")
	if sim != null and sim.has_method("set_ai_turn_recording"):
		sim.call("set_ai_turn_recording", mode() != "hide", float(tuning()["notable_radius_km"]))


## Après `end_turn` et `refresh_all` (marqueurs à leur position finale) : rejoue les marches.
## Rend la main tout de suite en « Masquer » ou sans mouvement visible.
func play() -> void:
	var started := Time.get_ticks_usec()
	var current_mode := mode()
	last_stats = {"mode": current_mode, "moves": 0, "shown": 0, "followed": 0, "skipped": false, "record_ms": 0.0, "replay_ms": 0.0}
	var sim: Object = map.get("sim")
	if current_mode == "hide" or sim == null or not sim.has_method("get_ai_turn_moves"):
		return
	var moves: Array = sim.call("get_ai_turn_moves")
	var shown := _shown_moves(moves)
	var followed := _followed(shown) if current_mode == "follow" else {}
	last_stats["moves"] = moves.size()
	last_stats["shown"] = shown.size()
	last_stats["followed"] = followed.size()
	last_stats["record_ms"] = (Time.get_ticks_usec() - started) / 1000.0
	if shown.is_empty():
		return
	playing = true
	_skip = false
	replay_started.emit(shown.size(), followed.size())
	var rig: CampaignCamera = map.get("camera_rig")
	var saved := {"focus": rig.target_focus, "distance": rig.target_distance, "yaw": rig.target_yaw} if rig != null else {}
	var ui: Node = map.get("ui")
	var banner: Control = ui.get("turn_banner") if ui != null else null
	var banner_was_visible := banner != null and banner.visible
	if banner_was_visible:
		banner.hide()
	_show_caption()
	var armies: ArmyMarkers = map.get("armies")
	# Chaque armée rejouée repart de son point de départ (les marqueurs sont déjà à l'arrivée).
	var placed: Dictionary = {}
	for move: Dictionary in shown:
		var army_id := str(move["army"])
		if not placed.has(army_id):
			placed[army_id] = true
			var start: PackedVector2Array = move["points"]
			armies.place_marker(army_id, start[0], start[1] - start[0] if start.size() > 1 else Vector2.ZERO)
			_idle(army_id)
	set_process(true)
	for move: Dictionary in shown:
		if _skip:
			break
		_set_caption(str(move["faction"]))
		var army_id := str(move["army"])
		var points: PackedVector2Array = move["points"]
		if not followed.has(int(move["sequence"])):
			_start_job(army_id, points, float(tuning()["background_batch_duration_s"]))
			continue
		if rig != null:
			rig.look_at_point(_world(points[0]), _follow_distance(rig))
			await _wait(float(tuning()["camera_pan_duration_s"]))
		if _skip:
			break
		_follow_army = army_id
		_start_job(army_id, points, float(tuning()["max_move_duration_s"]))
		while _jobs.has(army_id) and not _skip:
			await get_tree().process_frame
		_follow_army = ""
		if rig != null and not _skip and HOLD_KINDS.has(str(move["kind"])):
			rig.target_focus = _world(points[points.size() - 1])
			await _wait(float(tuning()["camera_hold_s"]))
	while not _jobs.is_empty() and not _skip:
		await get_tree().process_frame
	_finish_jobs()
	set_process(false)
	_hide_caption()
	if rig != null and not saved.is_empty():
		rig.look_at_point(saved["focus"], saved["distance"])
		rig.target_yaw = saved["yaw"]
		if _skip:
			rig.snap()
		elif not followed.is_empty():
			await _wait(float(tuning()["camera_return_duration_s"]))
	if banner_was_visible and ui != null:
		ui.call("show_turn_banner")
		ui.call("finish_turn_banner")
	last_stats["skipped"] = _skip
	last_stats["replay_ms"] = (Time.get_ticks_usec() - started) / 1000.0
	playing = false
	_skip = false
	replay_finished.emit()


## Passe le reste de la relecture (Espace, tests).
func skip() -> void:
	if playing:
		_skip = true


## Mouvements montrés : vus par le joueur (brouillard coupé : tous) et dont l'armée a un
## marqueur, ou batailles (l'armée a pu périr). Trajet réduit à sa partie vue (± 1 point).
func _shown_moves(moves: Array) -> Array:
	var fog := bool(_setting("map/fog_of_war", true))
	var armies: ArmyMarkers = map.get("armies")
	var result: Array = []
	for move: Dictionary in moves:
		var path: PackedVector2Array = move.get("path", PackedVector2Array())
		if path.size() < 2:
			continue
		if fog and not bool(move.get("visible", false)):
			continue
		var kind := str(move.get("kind", ""))
		if armies == null or not (armies.has_army(str(move.get("army", ""))) or kind == "battle"):
			continue
		var points := path
		if fog:
			var from := maxi(int(move.get("visible_from", 0)) - 1, 0)
			var to := mini(int(move.get("visible_to", path.size() - 1)) + 1, path.size() - 1)
			points = path.slice(from, to + 1)
			if points.size() < 2:
				continue
		var entry := move.duplicate()
		entry["points"] = points
		result.append(entry)
	return result


## Numéros (`sequence`) des mouvements suivis : les `max_followed_moves` plus importants.
func _followed(shown: Array) -> Dictionary:
	var notable := shown.filter(func(move: Dictionary) -> bool: return int(move.get("priority", 0)) > 0)
	notable.sort_custom(func(a: Dictionary, b: Dictionary) -> bool:
		if int(a["priority"]) != int(b["priority"]):
			return int(a["priority"]) > int(b["priority"])
		return int(a["sequence"]) < int(b["sequence"]))
	var result: Dictionary = {}
	for move: Dictionary in notable.slice(0, int(tuning()["max_followed_moves"])):
		result[int(move["sequence"])] = true
	return result


func _start_job(army_id: String, points: PackedVector2Array, max_duration: float) -> void:
	var length := 0.0
	for i in range(1, points.size()):
		length += points[i - 1].distance_to(points[i])
	var t := tuning()
	var duration := clampf(length / float(t["march_speed_px_per_s"]), float(t["min_move_duration_s"]), maxf(max_duration, float(t["min_move_duration_s"])))
	var armies: ArmyMarkers = map.get("armies")
	if length < 0.5 or not armies.has_army(army_id):
		# Bataille d'une armée disparue : pas de marqueur, seule la caméra s'y porte.
		_jobs[army_id] = {"points": points, "length": length, "elapsed": 0.0, "duration": duration if length >= 0.5 else 0.0, "ghost": true}
		return
	_jobs[army_id] = {"points": points, "length": length, "elapsed": 0.0, "duration": duration, "ghost": false}


func _process(delta: float) -> void:
	var step := delta * speed()
	for army_id: String in _jobs.keys():
		_step(army_id, step)


func _step(army_id: String, delta: float) -> void:
	var job: Dictionary = _jobs[army_id]
	job["elapsed"] = minf(float(job["elapsed"]) + delta, float(job["duration"]))
	var t := float(job["elapsed"]) / maxf(float(job["duration"]), 0.001)
	var points: PackedVector2Array = job["points"]
	var target := t * float(job["length"])
	var point := points[points.size() - 1]
	var heading := Vector2.ZERO
	for i in range(1, points.size()):
		var segment := points[i - 1].distance_to(points[i])
		if target <= segment or i == points.size() - 1:
			point = points[i - 1].lerp(points[i], clampf(target / maxf(segment, 0.001), 0.0, 1.0))
			heading = points[i] - points[i - 1]
			break
		target -= segment
	var armies: ArmyMarkers = map.get("armies")
	if not bool(job["ghost"]):
		armies.place_marker(army_id, point, heading)
	if army_id == _follow_army:
		var rig: CampaignCamera = map.get("camera_rig")
		if rig != null:
			rig.target_focus = _world(point)
	if float(job["elapsed"]) >= float(job["duration"]):
		_jobs.erase(army_id)
		if not bool(job["ghost"]):
			armies.place_marker(army_id, Vector2(-1, -1), heading)


## Au point de départ, l'armée attend son tour sans marcher sur place.
func _idle(army_id: String) -> void:
	var markers: Dictionary = map.get("armies").get("_markers")
	var marker: Node = markers.get(army_id) if markers != null else null
	if marker != null and marker.has_method("set_walking"):
		marker.call("set_walking", false)


## Toutes les armées à leur position finale (fin, Espace).
func _finish_jobs() -> void:
	var armies: ArmyMarkers = map.get("armies")
	for army_id: String in _jobs.keys():
		if not bool(_jobs[army_id]["ghost"]):
			armies.place_marker(army_id, Vector2(-1, -1))
	_jobs.clear()


func _wait(seconds: float) -> void:
	var remaining := seconds / speed()
	while remaining > 0.0 and not _skip:
		await get_tree().process_frame
		remaining -= get_process_delta_time()


func _world(point: Vector2) -> Vector3:
	var map_data: MapData = map.get("map_data")
	var height := map_data.surface_world_at(point.x, point.y) if map_data != null else 0.0
	return Vector3(point.x, height, point.y)


func _follow_distance(rig: CampaignCamera) -> float:
	var map_data: MapData = map.get("map_data")
	var span := maxf(map_data.size.x, map_data.size.y) if map_data != null else 4096.0
	return minf(span * float(tuning()["camera_follow_distance_share"]), rig.max_distance)


func _unhandled_input(event: InputEvent) -> void:
	if not playing:
		return
	if event is InputEventKey and event.pressed and not event.echo and (event as InputEventKey).keycode == KEY_SPACE:
		skip()
		get_viewport().set_input_as_handled()


# --- Légende « Tour de … — Espace : passer » ---------------------------------------------


func _show_caption() -> void:
	var ui: Node = map.get("ui")
	if ui == null:
		return
	if _caption == null:
		_caption = PanelContainer.new()
		_caption.name = "AiTurnCaption"
		_caption.mouse_filter = Control.MOUSE_FILTER_IGNORE
		_caption.add_theme_stylebox_override("panel", HudStyle.illuminated_box(10))
		_caption_label = HudStyle.label("", HudStyle.FONT_BODY + 2, HudStyle.RUBRIC)
		_caption_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
		_caption.add_child(_caption_label)
		ui.add_child(_caption)
	_caption.show()


func _set_caption(faction: String) -> void:
	if _caption_label == null:
		return
	var facade := get_node_or_null("/root/SimFacade")
	var name_fr: String = str(facade.call("faction_short_name", faction)) if facade != null else faction
	_caption_label.text = "Tour de l'IA : %s  —  Espace : passer" % name_fr
	var view := get_viewport().get_visible_rect().size
	_caption.reset_size()
	_caption.position = Vector2((view.x - _caption.size.x) * 0.5, view.y - _caption.size.y - 120.0)


func _hide_caption() -> void:
	if _caption != null:
		_caption.hide()


## Texte courant de la légende (tests).
func caption_text() -> String:
	return _caption_label.text if _caption_label != null and _caption != null and _caption.visible else ""
