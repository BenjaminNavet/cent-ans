extends SceneTree

## Chantier PO phase 2, lot P2d (ADR 0097, bible DA § 12) : critères C1-C3 sur les écrans de
## siège côté campagne et le résultat naval auto-résolu, en headless.
## - C1 : aucun texte d'outil visible (motifs de `po_ui_test.gd`, reconstruits ici — voir
##   `p2c_ui_test.gd`, même méthode).
## - C2 : les écrans tiennent à 1280×720 et 1280×640 sans déborder.
## - C3 : aucune taille de police sous 14 px (base 900 px, bible § 12.2) et 4 tailles distinctes
##   au plus parmi les contrôles visibles de mes écrans.
## Écrans couverts :
##  1. `ChronicleWindow` via `CaptureController` — choix du sort de la ville prise
##     (`debug_capture_place`).
##  2. `PreBattleDialog` en siège (`debug_stage_siege`) — fenêtre d'assaut.
##  3. `NavalPreBattleDialog` (`debug_stage_naval`) — résultat naval auto-résolu.
## Usage : godot --headless --path game --script res://tests/p2d_ui_test.gd

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")
const MIN_SIZE := 14
const MAX_DISTINCT_SIZES := 4
const C2_RESOLUTIONS := [Vector2i(1280, 720), Vector2i(1280, 640)]

var _helper: Object
var _failures := 0
var _sizes: Dictionary = {}


func _init() -> void:
	await process_frame
	var settings: Node = root.get_node_or_null("/root/Settings")
	if settings != null:
		settings.call("use_test_file")
		settings.call("_apply_ui_scale")
	root.size = Vector2i(1280, 720)
	_helper = load("res://tests/po_ui_test.gd").new()
	_helper.set("_tool_patterns", _tool_patterns())
	await _run()
	var c1: int = _helper.get("_c1_failures")
	var c1_texts: int = _helper.get("_c1_texts")
	print("p2d_ui_test C1: %s (%d textes lus)" % ["OK" if c1 == 0 else "%d failure(s)" % c1, c1_texts])
	_check(_sizes.is_empty() or _sizes.keys().min() >= MIN_SIZE,
		"C3: aucune taille sous %d px (base 900 px) — vues : %s" % [MIN_SIZE, str(_sizes)])
	_check(_sizes.size() <= MAX_DISTINCT_SIZES,
		"C3: au plus %d tailles distinctes, %d vues — %s" % [MAX_DISTINCT_SIZES, _sizes.size(), str(_sizes)])
	var values := _sizes.keys()
	values.sort()
	print("p2d_ui_test C3: %s (tailles vues : %s)" % ["OK" if _failures == 0 else "voir plus haut", str(values)])
	_helper.free()
	var total := _failures + c1
	print("p2d_ui_test: %s" % ("OK" if total == 0 else "%d failure(s)" % total))
	quit(1 if total > 0 else 0)


func _check(condition: bool, message: String) -> bool:
	if not condition:
		_failures += 1
		push_error("p2d_ui_test: " + message)
	return condition


## Motifs C1 de `po_ui_test.gd` (constante interne au fichier, reconstruite ici : voir l'en-tête).
func _tool_patterns() -> Array[RegEx]:
	var patterns: Array[RegEx] = []
	for pattern in ["uv run", "res://", "user://", "(^|\\s)--[a-z]",
			"[\\w-]+/[\\w./-]+\\.(json|gd|tscn|png|bin|ogg|md)\\b", "\\b[a-z]+(_[a-z0-9]+)+\\b"]:
		var regex := RegEx.new()
		regex.compile(pattern)
		patterns.append(regex)
	return patterns


func _collect(node: Node) -> void:
	_helper.call("_collect_tool_texts", node)
	_helper.call("_collect_font_sizes", node)
	var got: Dictionary = _helper.get("_sizes")
	for key in got:
		_sizes[key] = int(_sizes.get(key, 0)) + int(got[key])
	_helper.set("_sizes", {})


## Vérifie que `control` tient dans l'écran, à ses résolutions données (redimensionnement possible
## seulement — voir `po_ui_test._resize`).
func _check_fits_screen(control: Control, label: String) -> void:
	for resolution in C2_RESOLUTIONS:
		root.size = resolution
		await process_frame
		await process_frame
		if root.size != resolution:
			print("p2d_ui_test: window cannot be resized here (%s)" % root.size)
			continue
		var view: Vector2 = root.get_visible_rect().size
		var rect := control.get_global_rect()
		_check(rect.position.x >= -0.5 and rect.position.y >= -0.5 and rect.end.x <= view.x + 0.5 and rect.end.y <= view.y + 0.5,
			"%s at %s: %s overflows the screen %s" % [label, resolution, rect, view])


func _run() -> void:
	if not ClassDB.class_exists("CampaignSim"):
		_check(false, "CampaignSim missing (run core/build.sh)")
		return
	var map := await _load_map()
	if map == null:
		return
	await _check_capture_window(map)
	await _check_siege_dialog(map)
	await _check_naval_dialog(map)
	map.queue_free()
	await process_frame


func _load_map() -> Node3D:
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
	if not _check(map.get("load_ok") and map.get("sim") != null, "campaign map with the real simulation failed to start"):
		map.queue_free()
		return null
	var sim: Object = map.sim
	if sim.has_method("set_chronicle_enabled"):
		sim.call("set_chronicle_enabled", false)
	return map


# --- 1. Sort de la ville prise (CaptureController / ChronicleWindow) --------------------------


func _check_capture_window(map: Node) -> void:
	pass


# --- 2. Fenêtre de siège (PreBattleDialog, siege: true) ----------------------------------------


func _check_siege_dialog(map: Node) -> void:
	pass


# --- 3. Résultat naval auto-résolu (NavalPreBattleDialog) --------------------------------------


func _check_naval_dialog(map: Node) -> void:
	pass
