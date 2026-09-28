extends SceneTree

## Chantier PO phase 2 (P2b, ADR 0097) : critères C1-C3 sur les écrans de techniques
## (`TechPanel` / `TechTreeView`) et de diplomatie (`DiplomacyPanel` / `DiplomaticStances`),
## migrés vers `UiLayout` (zone `MODAL`), `UiType` (quatre tailles) et `UiMotion`.
## Même méthode que `po_ui_test.gd` (non modifié par ce lot, gardé comme référence pour le reste
## de la tranche) :
## - C1 : aucun texte d'outil visible hors mode dev.
## - C2 (ici : zone `MODAL`) : le panneau ouvert est un occupant visible de `UiLayout.MODAL`, le
##   voile s'affiche, et il disparaît une fois le panneau refermé.
## - C3 : aucune taille de police sous `Caption` (14 px de base) et 4 tailles au plus dans les
##   contrôles visibles des deux écrans.
## Usage : godot --headless --path game --script res://tests/p2b_ui_test.gd

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")
## Bible DA § 12.2 : Title 26, Heading 20, Body 17, Caption 14 (base 900 px). Rien en dessous ;
## 4 valeurs au plus (mêmes seuils que `po_ui_test.gd`).
const MIN_SIZE := 14
const MAX_DISTINCT_SIZES := 4
const _TEXT_CLASSES := ["Label", "RichTextLabel", "Button", "CheckBox", "CheckButton",
	"LinkButton", "MenuButton", "OptionButton", "LineEdit"]

var _failures := 0
var _c1_failures := 0
var _c1_texts := 0
var _c2_failures := 0
var _c2_checks := 0
var _tool_patterns: Array[RegEx] = []
## Tailles vues (valeur → nombre d'occurrences), les deux écrans confondus.
var _sizes: Dictionary = {}


func _init() -> void:
	await process_frame
	await _run()
	print("p2b_ui_test C1: %s (%d textes lus)" % ["OK" if _c1_failures == 0 else "%d failure(s)" % _c1_failures, _c1_texts])
	print("p2b_ui_test C2: %s (%d contrôles)" % ["OK" if _c2_failures == 0 else "%d failure(s)" % _c2_failures, _c2_checks])
	if _failures - _c1_failures - _c2_failures == 0:
		var values := _sizes.keys()
		values.sort()
		print("p2b_ui_test C3: OK (tailles vues : %s)" % str(values))
	else:
		print("p2b_ui_test C3: %d failure(s)" % (_failures - _c1_failures - _c2_failures))
	quit(1 if _failures > 0 else 0)


func _check(condition: bool, message: String) -> bool:
	if not condition:
		_failures += 1
		push_error("p2b_ui_test: " + message)
	return condition


func _check_c2(condition: bool, message: String) -> bool:
	_c2_checks += 1
	if not condition:
		_c2_failures += 1
	return _check(condition, "C2: " + message)


## Parcourt les `Control` visibles sous `node` et ajoute leur taille de police à `_sizes` (mêmes
## classes et la même exclusion des listes `PanelWidgets` — nommées `*List` — que `po_ui_test.gd`).
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


## Parcourt les `Label` / `RichTextLabel` / boutons visibles sous `node` : aucun motif d'outil
## (mêmes motifs que `po_ui_test.gd`, bible DA § 12.5).
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


func _run() -> void:
	for pattern in ["uv run", "res://", "user://", "(^|\\s)--[a-z]", "[\\w-]+/[\\w./-]+\\.(json|gd|tscn|png|bin|ogg|md)\\b",
			"\\b[a-z]+(_[a-z0-9]+)+\\b"]:
		var regex := RegEx.new()
		regex.compile(pattern)
		_tool_patterns.append(regex)

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
		return

	await _check_tech_panel(map)
	await _check_diplomacy_panel(map)

	map.queue_free()
	await process_frame

	_check(_sizes.is_empty() or _sizes.keys().min() >= MIN_SIZE,
		"C3: aucune taille sous %d px (base 900 px) — vues : %s" % [MIN_SIZE, str(_sizes)])
	_check(_sizes.size() <= MAX_DISTINCT_SIZES,
		"C3: au plus %d tailles distinctes, %d vues — %s" % [MAX_DISTINCT_SIZES, _sizes.size(), str(_sizes)])


## Panneau « Technologies » (`tech_panel.gd`, `tech_tree_view.gd`) : zone `MODAL`, tailles,
## textes d'outil, fermeture animée (`UiMotion`, instantanée en headless).
func _check_tech_panel(map: Node3D) -> void:
	var layout: Node = root.get_node("/root/UiLayout")
	if not _check(map.has_method("_tech_available") and map.call("_tech_available"), "technologies indisponibles avec cette simulation"):
		return
	map.call("_on_tech_panel_requested")
	await process_frame
	var panel: Control = map.ui.tech_panel
	if not _check(panel.visible, "le panneau des technologies devrait s'ouvrir"):
		return
	_check_c2(layout.visible_occupants(layout.Zone.MODAL).has(panel), "le panneau des technologies devrait occuper la zone MODAL")
	var dim: Control = layout.host().get_node("UiModalDim")
	_check_c2(dim.visible, "le voile modal devrait s'afficher avec le panneau des technologies")
	_collect_font_sizes(panel)
	_collect_tool_texts(panel)
	panel.close_button.pressed.emit()
	await process_frame
	_check_c2(not panel.visible, "le panneau des technologies devrait se refermer (bouton ×, UiMotion)")
	_check_c2(not layout.visible_occupants(layout.Zone.MODAL).has(panel), "le panneau fermé ne devrait plus occuper la zone MODAL")


## Écran de diplomatie (`diplomacy_panel.gd`, `diplomatic_stances.gd`) : zone `MODAL`, tailles,
## textes d'outil, ouverture et fermeture animées (`UiMotion`).
func _check_diplomacy_panel(map: Node3D) -> void:
	var layout: Node = root.get_node("/root/UiLayout")
	var controller: Node = map.diplomacy
	if not _check(controller != null and controller.call("available"), "diplomatie indisponible avec cette simulation"):
		return
	controller.call("open_panel")
	await process_frame
	var panel: Control = controller.panel
	if not _check(panel.visible, "l'écran de diplomatie devrait s'ouvrir"):
		return
	_check_c2(layout.visible_occupants(layout.Zone.MODAL).has(panel), "l'écran de diplomatie devrait occuper la zone MODAL")
	var dim: Control = layout.host().get_node("UiModalDim")
	_check_c2(dim.visible, "le voile modal devrait s'afficher avec l'écran de diplomatie")
	_collect_font_sizes(panel)
	_collect_tool_texts(panel)
	# Fermeture par le bouton « × » du panneau (mêmes recherches que `HudStyle` : premier bouton
	# de 36 px de large dans l'en-tête).
	var close_button := _find_close_button(panel)
	if _check(close_button != null, "bouton de fermeture de l'écran de diplomatie introuvable"):
		close_button.pressed.emit()
		await process_frame
		_check_c2(not panel.visible, "l'écran de diplomatie devrait se refermer (bouton ×, UiMotion)")
		_check_c2(not layout.visible_occupants(layout.Zone.MODAL).has(panel), "l'écran fermé ne devrait plus occuper la zone MODAL")


func _find_close_button(node: Node) -> Button:
	if node is Button and (node as Button).text == "×":
		return node
	for child in node.get_children():
		var found := _find_close_button(child)
		if found != null:
			return found
	return null
