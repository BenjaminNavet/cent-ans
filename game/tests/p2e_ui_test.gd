extends TestCase

## Lot P2e (chantier PO, phase 2) : critères C1-C3 (ADR 0097) sur les menus secondaires migrés
## (démos, batailles historiques, rejeux, réglages, pause, crédits, sauvegarder/charger).
## - C1 : aucun texte d'outil visible (chemin, option `--…`, identifiant brut en snake_case).
## - C2 (adapté aux fenêtres modales) : chaque menu migré rejoint effectivement la zone `MODAL`
##   de `UiLayout` (occupant visible de la zone — au lieu d'un test de rectangle : la fenêtre
##   headless est fixée à 64×64 dans ce script, `po_ui_test.gd -- --resolution=1280x720` en
##   sous-processus est la seule façon d'obtenir une vraie taille, hors de portée d'un test
##   direct).
## - C3 : aucune taille de police sous `Caption` (14 px de base), 4 tailles au plus.
##
## Écrit pour réutiliser les aides de `po_ui_test.gd` (`_collect_font_sizes`,
## `_collect_tool_texts`, motifs C1, `MIN_SIZE`/`MAX_DISTINCT_SIZES`) sans le modifier — mais
## `po_ui_test.gd extends SceneTree` et son `_init()` lance et *termine* (`quit()`) tout seul dès
## `HELPER_SCRIPT.new()` : l'instancier aurait avorté ce script avant même le premier test. Les
## mêmes aides sont donc reproduites ici (mêmes motifs, mêmes constantes `MIN_SIZE` = 14 et
## `MAX_DISTINCT_SIZES` = 4) plutôt qu'appelées sur une instance. Point ouvert signalé au wip.
##
## Usage : godot --headless --path game --script res://tests/p2e_ui_test.gd

const PAUSE_MENU_SCENE := "res://scenes/ui/pause_menu.tscn"
const SAVE_DIALOG_SCENE := "res://scenes/ui/save_load_dialog.tscn"
## Bible DA § 12.2, mêmes valeurs que `po_ui_test.gd`.
const MIN_SIZE := 14
const MAX_DISTINCT_SIZES := 4
const _TEXT_CLASSES := ["Label", "RichTextLabel", "Button", "CheckBox", "CheckButton",
	"LinkButton", "MenuButton", "OptionButton", "LineEdit"]

var _c1_failures := 0
var _c1_texts := 0
var _modal_checks := 0
var _modal_failures := 0
var _sizes: Dictionary = {}
var _tool_patterns: Array[RegEx] = []


func _init() -> void:
	for pattern in ["uv run", "res://", "user://", "(^|\\s)--[a-z]", "[\\w-]+/[\\w./-]+\\.(json|gd|tscn|png|bin|ogg|md)\\b",
			"\\b[a-z]+(_[a-z0-9]+)+\\b"]:
		var regex := RegEx.new()
		regex.compile(pattern)
		_tool_patterns.append(regex)
	await process_frame
	await _run()
	var sizes := _sizes.keys()
	sizes.sort()
	var c3_ok: bool = (_sizes.is_empty() or int(sizes.min()) >= MIN_SIZE) and _sizes.size() <= MAX_DISTINCT_SIZES
	print("p2e_ui_test C1: %s (%d textes lus)" % ["OK" if _c1_failures == 0 else "%d failure(s)" % _c1_failures, _c1_texts])
	print("p2e_ui_test C2 (zone MODAL): %s (%d contrôles)" % ["OK" if _modal_failures == 0 else "%d failure(s)" % _modal_failures, _modal_checks])
	print("p2e_ui_test C3: %s (tailles vues : %s)" % ["OK" if c3_ok else "FAIL", str(sizes)])
	var failed := failures + _c1_failures + _modal_failures + (0 if c3_ok else 1)
	print("p2e_ui_test done, %d failure(s)" % failed)
	quit(1 if failed > 0 else 0)


func _run() -> void:
	await _check_standalone_menu(BattleDemosMenu.new(), "BattleDemosMenu")
	await _check_standalone_menu(HistoricalBattlesMenu.new(), "HistoricalBattlesMenu")
	ReplaysMenu.dir_override = ProjectSettings.globalize_path("user://p2e_ui_test_replays")
	await _check_standalone_menu(ReplaysMenu.new(), "ReplaysMenu")
	await _check_standalone_menu(SettingsMenu.new(), "SettingsMenu")
	await _check_credits()
	await _check_pause()
	await _check_save_dialog()


## Un menu qui rejoint `MODAL` seul dans `_ready()` (démos, batailles historiques, rejeux, réglages).
func _check_standalone_menu(menu: Control, label: String) -> void:
	root.add_child(menu)
	await process_frame
	await process_frame
	_collect_font_sizes(menu)
	_collect_tool_texts(menu)
	_check_in_modal(menu, label)
	menu.queue_free()
	await process_frame


## `CreditsScreen` : `self` (fond) reste hors zone, `_panel` (cadre) rejoint `MODAL`.
func _check_credits() -> void:
	var screen: Control = (load("res://scenes/ui/credits_screen.tscn") as PackedScene).instantiate()
	root.add_child(screen)
	await process_frame
	await process_frame
	_collect_font_sizes(screen)
	_collect_tool_texts(screen)
	_check_in_modal(screen.get("_panel"), "CreditsScreen._panel")
	screen.queue_free()
	await process_frame


## `PauseMenu` : `_menu_panel`, `_confirm_panel` et `save_dialog` rejoignent `MODAL` chacun.
func _check_pause() -> void:
	var pause: Control = (load(PAUSE_MENU_SCENE) as PackedScene).instantiate()
	pause.unsaved_turns = 3
	root.add_child(pause)
	await process_frame
	await process_frame
	_collect_font_sizes(pause)
	_collect_tool_texts(pause)
	_check_in_modal(pause.get("_menu_panel"), "PauseMenu._menu_panel")
	pause._request_exit("main_menu")
	await process_frame
	_collect_font_sizes(pause.get("_confirm_panel"))
	_check_in_modal(pause.get("_confirm_panel"), "PauseMenu._confirm_panel")
	pause.queue_free()
	await process_frame


## `SaveLoadDialog` seul (hors `PauseMenu`) : tailles de police seulement — son rattachement à une
## zone reste celui de son hôte (`start_menu.gd` / `map_ui.gd`, hors lot), pas de `MODAL` propre.
func _check_save_dialog() -> void:
	var dialog: Control = (load(SAVE_DIALOG_SCENE) as PackedScene).instantiate()
	root.add_child(dialog)
	await process_frame
	dialog.open_save("Partie de démonstration")
	await process_frame
	await process_frame
	_collect_font_sizes(dialog)
	_collect_tool_texts(dialog)
	dialog.queue_free()
	await process_frame


## C2 adapté : `control` est un occupant visible de la zone `MODAL` (topologie plutôt que
## géométrie — la fenêtre headless de ce script ne peut pas être mise à une taille réelle, voir
## la note en tête de fichier).
func _check_in_modal(control: Control, label: String) -> void:
	_modal_checks += 1
	if control == null or not is_instance_valid(control):
		_modal_failures += 1
		push_error("p2e_ui_test: %s introuvable" % label)
		return
	var layout: Node = root.get_node_or_null("/root/UiLayout")
	if layout == null:
		_modal_failures += 1
		push_error("p2e_ui_test: autoload UiLayout absent")
		return
	var zoned: bool = control.has_meta(&"ui_layout_zone") and int(control.get_meta(&"ui_layout_zone")) == int(layout.Zone.MODAL)
	var listed: bool = layout.visible_occupants(layout.Zone.MODAL).has(control)
	if not (control.visible and zoned and listed):
		_modal_failures += 1
		push_error("p2e_ui_test: %s pas un occupant visible de MODAL (zoned=%s, listed=%s, visible=%s)" % [label, zoned, listed, control.visible])


## Reprise de `po_ui_test._collect_font_sizes` (mêmes classes de texte, même exclusion des listes
## `*List` de `PanelWidgets`, hors lot).
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


## Reprise de `po_ui_test._collect_tool_texts` (mêmes motifs C1, bible DA § 12.5).
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
				check(false, "C1: tool text « %s » in %s: %s" % [found.get_string(), node.get_path(), text.substr(0, 120)])
				break
	for child in node.get_children():
		_collect_tool_texts(child)
