extends TestCase

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

const MIN_SIZE := 14
const MAX_DISTINCT_SIZES := 4
const C2_RESOLUTIONS := [Vector2i(1280, 720), Vector2i(1280, 640)]

var _helper: Object
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
	check(_sizes.is_empty() or _sizes.keys().min() >= MIN_SIZE,
		"C3: aucune taille sous %d px (base 900 px) — vues : %s" % [MIN_SIZE, str(_sizes)])
	check(_sizes.size() <= MAX_DISTINCT_SIZES,
		"C3: au plus %d tailles distinctes, %d vues — %s" % [MAX_DISTINCT_SIZES, _sizes.size(), str(_sizes)])
	var values := _sizes.keys()
	values.sort()
	print("p2d_ui_test C3: %s (tailles vues : %s)" % ["OK" if failures == 0 else "voir plus haut", str(values)])
	_helper.free()
	var total := failures + c1
	print("p2d_ui_test: %s" % ("OK" if total == 0 else "%d failure(s)" % total))
	quit(1 if total > 0 else 0)


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
		check(rect.position.x >= -0.5 and rect.position.y >= -0.5 and rect.end.x <= view.x + 0.5 and rect.end.y <= view.y + 0.5,
			"%s at %s: %s overflows the screen %s" % [label, resolution, rect, view])


func _run() -> void:
	if not ClassDB.class_exists("CampaignSim"):
		check(false, "CampaignSim missing (run core/build.sh)")
		return
	# Chaque écran a son propre `CampaignSim` (même approche que `ub1_ui_test.gd`) :
	# `debug_stage_siege`/`debug_stage_naval` déplacent des armées et
	# entrent en guerre, ce qui invaliderait les batailles mises en scène par les autres écrans
	# si le même `CampaignSim` (ou la même carte) était réutilisé.
	await _check_siege_dialog()
	await _check_naval_dialog()
	var map := await _load_map()
	if map == null:
		return
	await _check_capture_window(map)
	map.queue_free()
	await process_frame


func _data_dir() -> String:
	return ProjectSettings.globalize_path("res://").path_join("../data").simplify_path()


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
	if not check(map.get("load_ok") and map.get("sim") != null, "campaign map with the real simulation failed to start"):
		map.queue_free()
		return null
	var sim: Object = map.sim
	if sim.has_method("set_chronicle_enabled"):
		sim.call("set_chronicle_enabled", false)
	return map


# --- 1. Sort de la ville prise (CaptureController / ChronicleWindow) --------------------------


func _check_capture_window(map: Node) -> void:
	var sim: Object = map.sim
	if not check(sim.has_method("debug_capture_place"), "debug_capture_place missing (run core/build.sh)"):
		return
	var controller: CaptureController = map.get("capture_fate")
	if not check(controller != null, "campaign map has no CaptureController"):
		return
	if not check(sim.call("debug_capture_place", "prov_guyenne"), "debug capture of Guyenne refused"):
		return
	map.refresh_all()
	await process_frame
	await process_frame
	var window: ChronicleWindow = controller.window
	if not check(window.visible, "capture window should open after the capture"):
		return
	_collect(window)
	await _check_fits_screen(window, "ChronicleWindow (sort de la ville prise)")
	# Q7 : les choix restent visibles sans défiler (en vue 720 l'enluminure les cachait).
	var options: VBoxContainer = window.get("_options_box")
	var view_rect := Rect2(Vector2.ZERO, root.get_visible_rect().size)
	for row in options.get_children():
		var choice := (row as Control).get_child(0) as Control
		check(view_rect.encloses(choice.get_global_rect().grow(-0.5)) and window.get_global_rect().encloses(choice.get_global_rect().grow(-0.5)),
			"Q7: capture choice %s at %s outside the screen / window" % [_text_of_button(choice), choice.get_global_rect()])
	# Referme la décision (« Occuper », premier choix) pour ne pas gêner la suite du test.
	var box: VBoxContainer = window.get("_options_box")
	if box.get_child_count() > 0:
		(box.get_child(0).get_child(0) as Button).pressed.emit()
		await process_frame
		await process_frame


## C2 des dialogues d'avant-bataille : le panneau (pas le voile plein écran) tient entièrement
## dans l'écran — rectangle global, position comprise, et sa taille n'est pas remontée au-dessus
## du budget par le minimum combiné de ses enfants (colonnes de régiments défilantes).
func _check_panel_fits(dialog: PreBattleDialog, label: String, resolution: Vector2i) -> void:
	var view: Vector2 = root.get_visible_rect().size
	var rect := dialog.panel.get_global_rect()
	var minimum := dialog.panel.get_combined_minimum_size()
	check(rect.position.x >= -0.5 and rect.position.y >= -0.5 and rect.end.x <= view.x + 0.5 and rect.end.y <= view.y + 0.5,
		"C2: %s at %s: panel %s overflows the screen %s" % [label, resolution, rect, view])
	check(minimum.x <= view.x + 0.5 and minimum.y <= view.y + 0.5,
		"C2: %s at %s: panel minimum size %s exceeds the screen %s" % [label, resolution, minimum, view])
	# Rangée d'actions (bas du panneau) : jamais rognée, les régiments défilent à sa place.
	for button: Button in [dialog.withdraw_button, dialog.auto_button, dialog.fight_button]:
		if not button.is_visible_in_tree():
			continue
		var button_rect := button.get_global_rect()
		check(rect.encloses(button_rect.grow(-0.5)) and button_rect.end.y <= view.y + 0.5,
			"C2: %s at %s: button « %s » %s outside the panel %s" % [label, resolution, button.text, button_rect, rect])
	print("p2d_ui_test C2: %s at %s: panel %s, minimum %s" % [label, resolution, rect, minimum])


# --- 2. Fenêtre de siège (PreBattleDialog, siege: true) ----------------------------------------


## `_layout()` cale `panel` sur la taille de l'écran à l'instant de `show_battle()`, sans écouter
## de façon fiable un redimensionnement ultérieur en tête headless (pas de serveur d'affichage
## réel) : chaque résolution de `C2_RESOLUTIONS` met donc en scène son propre siège, fenêtre
## fraîche créée après avoir posé `root.size` — au plus près de ce que voit le joueur au
## lancement, plutôt qu'un redimensionnement à la volée.
##
## C2 : la fenêtre tient à l'écran. Les colonnes d'armées défilent (grosses armées du départ) et
## `_layout()` réaffecte la taille à chaque changement de minimum (un premier calcul, libellés
## repliés sans largeur, gonflait le panneau à plus de 3000 px sans jamais le rétrécir).
func _check_siege_dialog() -> void:
	var first := true
	for resolution in C2_RESOLUTIONS:
		root.size = resolution
		await process_frame
		var sim: Object = ClassDB.instantiate("CampaignSim")
		if not check(sim.has_method("debug_stage_siege"), "debug_stage_siege missing (run core/build.sh)"):
			return
		if not check(sim.call("new_campaign", _data_dir(), "fac_france", 1337), "new_campaign failed"):
			return
		var armies: Array = BattleScene.main_armies(sim, "fac_france", "fac_england")
		if not check(not armies.is_empty(), "no French army to stage a siege"):
			continue
		var index: int = sim.call("debug_stage_siege", armies[0], "prov_guyenne")
		if not check(index >= 0, "debug_stage_siege refused"):
			continue
		var dialog: PreBattleDialog = (load("res://scenes/battle/pre_battle_dialog.tscn") as PackedScene).instantiate()
		root.add_child(dialog)
		await process_frame
		var pending: Array = sim.call("get_pending_battles")
		dialog.show_battle(sim, pending[0])
		await process_frame
		await process_frame
		check(dialog.visible and bool(pending[0].get("siege", false)), "the staged battle should be a siege")
		check(dialog.fight_button.text == "Donner l'assaut", "siege dialog: fight button should read « Donner l'assaut »")
		if first:
			_collect(dialog)
			first = false
		# C2 : bloquant depuis que les colonnes d'armées défilent (voir le commentaire de fonction).
		_check_panel_fits(dialog, "PreBattleDialog (siège)", resolution)
		var withdrawn := [-1]
		dialog.withdraw_requested.connect(func(i: int) -> void: withdrawn[0] = i)
		dialog.withdraw_button.emit_signal("pressed")
		check(withdrawn[0] == index, "« Maintenir le siège » should emit withdraw_requested")
		dialog.queue_free()
		await process_frame
	root.size = Vector2i(1280, 720)
	await process_frame


# --- 3. Résultat naval auto-résolu (NavalPreBattleDialog) --------------------------------------


## Même remarque que `_check_siege_dialog` (pas de redimensionnement à la volée en headless) :
## une interception fraîche par résolution.
func _check_naval_dialog() -> void:
	var first := true
	for resolution in C2_RESOLUTIONS:
		root.size = resolution
		await process_frame
		var sim: Object = ClassDB.instantiate("CampaignSim")
		if not check(sim.has_method("debug_stage_naval"), "debug_stage_naval missing (run core/build.sh)"):
			return
		if not check(sim.call("new_campaign", _data_dir(), "fac_england", 1337), "new_campaign failed"):
			return
		var armies: Array = BattleScene.main_armies(sim, "fac_england", "fac_france")
		if not check(not armies.is_empty(), "no English army to stage a naval interception"):
			continue
		var index: int = sim.call("debug_stage_naval", armies[0], "set_calais", "fac_france")
		if not check(index >= 0, "debug_stage_naval refused"):
			continue
		var pending: Array = sim.call("get_pending_naval_battles")
		if not check(pending.size() == 1, "one pending naval battle expected"):
			continue
		var dialog := NavalPreBattleDialog.new()
		root.add_child(dialog)
		await process_frame
		dialog.show_naval(sim, pending[0])
		await process_frame
		await process_frame
		check(dialog.visible and not dialog.fight_button.visible, "naval dialog: only auto-resolve is offered")
		check(not dialog.chance_label.text.is_empty(), "naval dialog: estimated chances missing")
		if first:
			_collect(dialog)
			first = false
		# C2 : bloquant, même mise en page que `_check_siege_dialog`.
		_check_panel_fits(dialog, "NavalPreBattleDialog (résultat naval)", resolution)
		dialog.queue_free()
		await process_frame
		# Résolution automatique : rend des évènements et vide l'attente (`NavalCampaign._on_auto`).
		var result: Dictionary = sim.call("auto_resolve_naval_battle", index)
		check(bool(result.get("ok", false)), "auto_resolve_naval_battle failed: %s" % [result])
		check((sim.call("get_pending_naval_battles") as Array).is_empty(), "the naval interception should be resolved")
	root.size = Vector2i(1280, 720)
	await process_frame


func _text_of_button(control: Control) -> String:
	return (control as Button).text if control is Button else str(control.name)
