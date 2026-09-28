extends SceneTree

## Chantier PO phase 2 (P2g, ADR 0097) : Techniques, Diplomatie, Cour, Fiche de personnage et
## `SaveLoadDialog` (carte et menu d'accueil) rejoignent la zone `MODAL` de `UiLayout`.
## Même méthode que `p2b_ui_test.gd` / `p2c_ui_test.gd` :
## - C1 : aucun texte d'outil visible hors mode dev.
## - C2 : chaque fenêtre est un occupant visible de `UiZones.Zone.MODAL` (voile allumé), reste
##   inscrite dans la pile de la carte (Échap la ferme) et tient dans l'écran à 1280×720,
##   1280×640 et 1920×1080 ; les grandes fenêtres restent sous la barre du haut ; la fiche se
##   range à droite de la Cour ; le voile s'éteint à la fermeture.
## - C3 : aucune taille de police sous `Caption` (14 px de base), 4 tailles au plus.
## Usage : godot --headless --path game --script res://tests/p2g_ui_test.gd

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")
const START_MENU := "res://scenes/start_menu.tscn"
const MIN_SIZE := 14
const MAX_DISTINCT_SIZES := 4
const C2_RESOLUTIONS := [Vector2i(1280, 720), Vector2i(1280, 640), Vector2i(1920, 1080)]
const _TEXT_CLASSES := ["Label", "RichTextLabel", "Button", "CheckBox", "CheckButton",
	"LinkButton", "MenuButton", "OptionButton", "LineEdit"]

var _failures := 0
var _c1_failures := 0
var _c1_texts := 0
var _c2_failures := 0
var _c2_checks := 0
var _tool_patterns: Array[RegEx] = []
var _sizes: Dictionary = {}
var _resizable := true


func _init() -> void:
	await process_frame
	# Fichier de test et échelle d'interface neutre avant le premier écran.
	var settings: Node = root.get_node_or_null("/root/Settings")
	if settings != null:
		settings.call("use_test_file")
		settings.call("_apply_ui_scale")
		settings.call("set_value", "game/autosave_interval", 0, false)
		settings.call("set_value", "tutorial/enabled", false, false)
	for pattern in ["uv run", "res://", "user://", "(^|\\s)--[a-z]", "[\\w-]+/[\\w./-]+\\.(json|gd|tscn|png|bin|ogg|md)\\b",
			"\\b[a-z]+(_[a-z0-9]+)+\\b"]:
		var regex := RegEx.new()
		regex.compile(pattern)
		_tool_patterns.append(regex)
	var initial: Vector2i = root.size
	await _check_campaign()
	root.size = initial
	await process_frame
	await _check_start_menu()
	root.size = initial
	await process_frame
	_check(_sizes.is_empty() or _sizes.keys().min() >= MIN_SIZE,
		"C3: aucune taille sous %d px (base 900 px) — vues : %s" % [MIN_SIZE, str(_sizes)])
	_check(_sizes.size() <= MAX_DISTINCT_SIZES,
		"C3: au plus %d tailles distinctes, %d vues — %s" % [MAX_DISTINCT_SIZES, _sizes.size(), str(_sizes)])
	var values := _sizes.keys()
	values.sort()
	print("p2g_ui_test C1: %s (%d textes lus)" % ["OK" if _c1_failures == 0 else "%d failure(s)" % _c1_failures, _c1_texts])
	print("p2g_ui_test C2: %s (%d contrôles)%s" % ["OK" if _c2_failures == 0 else "%d failure(s)" % _c2_failures, _c2_checks,
		"" if _resizable else " — fenêtre non redimensionnable ici"])
	var c3 := _failures - _c1_failures - _c2_failures
	print("p2g_ui_test C3: %s (tailles vues : %s)" % ["OK" if c3 == 0 else "%d failure(s)" % c3, str(values)])
	print("p2g_ui_test: %s" % ("OK" if _failures == 0 else "%d failure(s)" % _failures))
	quit(1 if _failures > 0 else 0)


func _check(condition: bool, message: String) -> bool:
	if not condition:
		_failures += 1
		push_error("p2g_ui_test: " + message)
	return condition


func _check_c2(condition: bool, message: String) -> bool:
	_c2_checks += 1
	if not condition:
		_c2_failures += 1
	return _check(condition, "C2: " + message)


# --- Carte de campagne ---------------------------------------------------------------------


func _check_campaign() -> void:
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
		return
	var layout: UiZones = root.get_node("/root/UiLayout")
	var ui: Node = map.ui

	# Techniques.
	if _check(map.call("_tech_available"), "technologies unavailable with this simulation"):
		map.call("_on_tech_panel_requested")
		await _settle()
		await _check_window(map, layout, ui.tech_panel, "TechPanel", true)
		ui.tech_panel.close_button.pressed.emit()
		await _settle()
		_check_closed(layout, ui.tech_panel, "TechPanel (bouton ×)")

	# Diplomatie (panneau ajouté par le contrôleur, réclamé en différé par la carte).
	var diplomacy: Node = map.diplomacy
	if _check(diplomacy != null and diplomacy.call("available"), "diplomacy unavailable with this simulation"):
		diplomacy.call("open_panel")
		await _settle()
		await _check_window(map, layout, diplomacy.panel, "DiplomacyPanel", true)
		_check_c2(ui.panels.close_top(), "Échap should close the diplomacy screen")
		await _settle()
		_check_closed(layout, diplomacy.panel, "DiplomacyPanel (Échap)")

	# Cour, puis fiche du souverain à côté.
	map.call("_on_court_panel_requested")
	await _settle()
	var court: Control = ui.court_panel
	if _check_c2(court.visible, "the court should open"):
		await _check_window(map, layout, court, "CourtPanel", true)
		var ids: Array = map.sim.call("get_faction_characters", "fac_france")
		if _check(not ids.is_empty(), "the court should list characters"):
			ui.character_selected.emit(str(ids[0]))
			await _settle()
			var sheet: Control = ui.character_sheet
			await _check_window(map, layout, sheet, "CharacterSheet", true)
			for resolution in C2_RESOLUTIONS:
				if not await _resize(map, resolution):
					continue
				var view := Vector2(resolution)
				var sheet_rect := sheet.get_global_rect()
				var court_rect := court.get_global_rect()
				_check_c2(is_equal_approx(court_rect.position.x, CharacterSheet.SCREEN_MARGIN),
					"%s: the court should stay on the left edge (%s)" % [resolution, court_rect])
				_check_c2(absf(sheet_rect.end.x - (view.x - CharacterSheet.SCREEN_MARGIN)) < 1.0,
					"%s: the sheet should stay on the right edge (%s)" % [resolution, sheet_rect])
				_check_c2(sheet_rect.position.x >= court_rect.end.x - 0.5,
					"%s: the sheet %s should sit beside the court %s" % [resolution, sheet_rect, court_rect])
			# Échap ferme d'abord la fiche (dessus), puis la Cour.
			_check_c2(ui.panels.close_top(), "Échap should close the character sheet")
			await _settle()
			_check_c2(not sheet.visible and court.visible, "Échap should close the sheet first, then the court")
		_check_c2(ui.panels.close_top(), "Échap should close the court")
		await _settle()
		_check_closed(layout, court, "CourtPanel (Échap)")

	# Dialogue de sauvegarde de la carte.
	var dialog: Control = ui.save_load_dialog
	dialog.call("open_save", "partie_test")
	await _settle()
	await _check_window(map, layout, dialog, "SaveLoadDialog (carte)", false)
	dialog.call("close")
	await _settle()
	_check_closed(layout, dialog, "SaveLoadDialog (carte)")

	map.queue_free()
	await process_frame
	await process_frame


## `panel` : occupant visible de la zone `MODAL`, voile allumé, inscrit dans la pile de la carte,
## opaque (fondu `UiMotion` terminé) et dans l'écran à chaque résolution ; `below_top_bar` : sous
## la barre du haut (grandes fenêtres, `map_ui._keep_on_screen`).
func _check_window(map: Node3D, layout: UiZones, panel: Control, label: String, below_top_bar: bool) -> void:
	if not _check_c2(panel.visible, "%s should be open" % label):
		return
	_check_c2(layout.visible_occupants(UiZones.Zone.MODAL).has(panel), "%s should be a visible UiZones.MODAL occupant" % label)
	_check_c2(layout.modal_open(), "%s: the modal veil should be on" % label)
	_check_c2(map.ui.panels.is_registered(panel), "%s should stay registered in the panel stack" % label)
	_check_c2(is_equal_approx(panel.modulate.a, 1.0), "%s should be opaque once open (UiMotion), alpha %.2f" % [label, panel.modulate.a])
	for resolution in C2_RESOLUTIONS:
		if not await _resize(map, resolution):
			continue
		var view := Vector2(resolution)
		var rect := panel.get_global_rect()
		_check_c2(rect.position.x >= -0.5 and rect.position.y >= -0.5 and rect.end.x <= view.x + 0.5 and rect.end.y <= view.y + 0.5,
			"%s at %s: %s overflows the screen %s" % [label, resolution, rect, view])
		if below_top_bar:
			var top: float = UiZones.rect(UiZones.Zone.TOP_BAR).end.y
			_check_c2(rect.position.y >= top - 0.5, "%s at %s: %s over the top bar (%.0f)" % [label, resolution, rect, top])
		_collect_font_sizes(panel)
		_collect_tool_texts(panel)


func _check_closed(layout: UiZones, panel: Control, label: String) -> void:
	_check_c2(not panel.visible, "%s should close" % label)
	_check_c2(not layout.modal_open(), "%s: the modal veil should be off once closed (%s)" % [label,
		layout.visible_occupants(UiZones.Zone.MODAL).map(func(c: Control) -> String: return str(c.name))])


func _resize(map: Node3D, resolution: Vector2i) -> bool:
	root.size = resolution
	await process_frame
	await process_frame
	if root.size != resolution:
		_resizable = false
		return false
	if map != null:
		map.ui.layout_hud()
	await process_frame
	return true


func _settle() -> void:
	for i in 4:
		await process_frame


# --- Menu d'accueil ------------------------------------------------------------------------


func _check_start_menu() -> void:
	var menu: Control = (load(START_MENU) as PackedScene).instantiate()
	root.add_child(menu)
	await _settle()
	var layout: UiZones = root.get_node("/root/UiLayout")
	var dialog: Control = menu.get("save_load_dialog")
	if not _check(dialog != null, "start menu without SaveLoadDialog"):
		menu.queue_free()
		return
	dialog.call("open_load")
	await _settle()
	if _check_c2(dialog.visible, "the start menu load dialog should open"):
		_check_c2(layout.visible_occupants(UiZones.Zone.MODAL).has(dialog), "start menu SaveLoadDialog should be a visible UiZones.MODAL occupant")
		_check_c2(layout.modal_open(), "start menu SaveLoadDialog: the modal veil should be on")
		for resolution in C2_RESOLUTIONS:
			if not await _resize(null, resolution):
				continue
			var view := Vector2(resolution)
			var rect := dialog.get_global_rect()
			_check_c2(rect.position.x >= -0.5 and rect.position.y >= -0.5 and rect.end.x <= view.x + 0.5 and rect.end.y <= view.y + 0.5,
				"start menu SaveLoadDialog at %s: %s overflows the screen %s" % [resolution, rect, view])
			_collect_font_sizes(dialog)
			_collect_tool_texts(dialog)
		dialog.call("close")
		await _settle()
		_check_closed(layout, dialog, "SaveLoadDialog (menu d'accueil)")
	menu.queue_free()
	await _settle()
	_check_c2(not is_instance_valid(dialog), "the start menu SaveLoadDialog should be freed with the menu")


# --- Aides C1 / C3 (mêmes règles que `po_ui_test.gd`) --------------------------------------


func _collect_font_sizes(node: Node) -> void:
	if node is Control and (node as Control).is_visible_in_tree():
		var control := node as Control
		if str(control.name).ends_with("List"):
			return
		if control.get_class() in _TEXT_CLASSES:
			var key := "normal_font_size" if control is RichTextLabel else "font_size"
			var size := control.get_theme_font_size(key)
			_sizes[size] = int(_sizes.get(size, 0)) + 1
	for child in node.get_children():
		_collect_font_sizes(child)


func _collect_tool_texts(node: Node) -> void:
	if node is CanvasItem and not (node as CanvasItem).is_visible_in_tree():
		return
	var text := ""
	if node is RichTextLabel:
		text = (node as RichTextLabel).get_parsed_text()
	elif node is Label:
		text = (node as Label).text
	elif node is Button:
		text = (node as Button).text
	if text.strip_edges() != "":
		_c1_texts += 1
		for regex in _tool_patterns:
			var found := regex.search(text)
			if found != null:
				_c1_failures += 1
				_check(false, "C1: tool text « %s » in %s: %s" % [found.get_string(), node.get_path(), text.substr(0, 120)])
				break
	for child in node.get_children():
		_collect_tool_texts(child)
