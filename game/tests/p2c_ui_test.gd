extends SceneTree

## Chantier PO phase 2, lot P2c (ADR 0097, bible DA § 12) : critères C1-C3 sur le Codex,
## l'encyclopédie, les bulles et les infobulles riches, en headless.
## - C1 : aucun texte d'outil visible (motifs de `po_ui_test.gd`, reconstruits ici car
##   `_tool_patterns` ne se remplit que dans son propre `_run`).
## - C2 : `CodexHub` (onglets Histoire et Règles) et la fenêtre `CodexWindow` autonome
##   (`CodexBubbles.window`, utilisée hors de la carte) vivent dans `UiZones.Zone.MODAL`, sans
##   déborder de l'écran à 1280×720 et 1920×1080.
## - C3 : aucune taille de police sous 14 px (base 900 px, bible § 12.2) et 4 tailles distinctes
##   au plus parmi les contrôles visibles de mes écrans (Codex, encyclopédie, bulles, infobulles).
## Réutilise `_collect_font_sizes` et `_collect_tool_texts` de `game/tests/po_ui_test.gd`, chargé
## et instancié pour son API interne, sans le modifier.
## Usage : godot --headless --path game --script res://tests/p2c_ui_test.gd

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")
const MIN_SIZE := 14
const MAX_DISTINCT_SIZES := 4
const C2_RESOLUTIONS := [Vector2i(1280, 720), Vector2i(1920, 1080)]
## Fiches connues (H8/H8b) : une bien remplie (corps, sources, "voir aussi") pour couvrir la
## page du Codex, une unité pour l'encyclopédie (liens internes).
const CODEX_ENTRY := "cdx_charles_v"
const ENCYCLOPEDIA_ENTRY := "unit_longbowmen"

var _helper: Object
var _failures := 0
var _sizes: Dictionary = {}


func _init() -> void:
	await process_frame
	# Fichier de test et échelle d'interface neutre avant le premier écran (sinon un
	# `interface/ui_size` du joueur dans `user://settings.cfg` fausse les tailles mesurées —
	# cause d'échecs vus ailleurs, ex. cb6/po_ui_test).
	var settings: Node = root.get_node_or_null("/root/Settings")
	if settings != null:
		settings.call("use_test_file")
		settings.call("_apply_ui_scale")
	_helper = load("res://tests/po_ui_test.gd").new()
	_helper.set("_tool_patterns", _tool_patterns())
	await _check_bubbles_and_tooltip()
	await _check_hub()
	await _check_standalone_window()
	var c1: int = _helper.get("_c1_failures")
	var c1_texts: int = _helper.get("_c1_texts")
	print("p2c_ui_test C1: %s (%d textes lus)" % ["OK" if c1 == 0 else "%d failure(s)" % c1, c1_texts])
	_check(_sizes.is_empty() or _sizes.keys().min() >= MIN_SIZE,
		"C3: aucune taille sous %d px (base 900 px) — vues : %s" % [MIN_SIZE, str(_sizes)])
	_check(_sizes.size() <= MAX_DISTINCT_SIZES,
		"C3: au plus %d tailles distinctes, %d vues — %s" % [MAX_DISTINCT_SIZES, _sizes.size(), str(_sizes)])
	var values := _sizes.keys()
	values.sort()
	print("p2c_ui_test C3: %s (tailles vues : %s)" % ["OK" if _failures == 0 else "voir plus haut", str(values)])
	_helper.free()
	var total := _failures + c1
	print("p2c_ui_test: %s" % ("OK" if total == 0 else "%d failure(s)" % total))
	quit(1 if total > 0 else 0)


func _check(condition: bool, message: String) -> bool:
	if not condition:
		_failures += 1
		push_error("p2c_ui_test: " + message)
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


# --- Bulles et infobulle riche (autonomes, sans carte) ----------------------------------------


func _check_bubbles_and_tooltip() -> void:
	var store: Node = root.get_node_or_null("/root/CodexStore")
	var bubbles: Node = root.get_node_or_null("/root/CodexBubbles")
	if not _check(store != null and bubbles != null, "CodexStore / CodexBubbles autoloads missing"):
		return
	store.call("use_test_file")
	var ids: Array = store.call("ids_in_family", -1)
	if not _check(not ids.is_empty(), "the test codex file should have entries"):
		return
	var bubble: PanelContainer = bubbles.call("open", str(ids[0]))
	await process_frame
	_check(bubble != null and bubble.visible, "a bubble should open on a known entry")
	if bubble != null:
		_collect(bubble)
	bubbles.call("close_all")
	await process_frame
	var panel: Control = RichTooltip.make_panel("[b]%s[/b]\nTexte d'infobulle riche." % "Test")
	root.add_child(panel)
	await process_frame
	_collect(panel)
	panel.queue_free()
	await process_frame


# --- CodexHub (carte) -------------------------------------------------------------------------


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
	return map


func _check_hub() -> Node3D:
	var map := await _load_map()
	if not _check(map.get("load_ok") and map.get("sim") != null, "campaign map with the real simulation failed to start"):
		map.queue_free()
		return null
	var bubbles: Node = root.get_node("/root/CodexBubbles")
	var hub: CodexHub = map.ui.codex_hub
	if not _check(hub != null, "the campaign map should carry a CodexHub"):
		map.queue_free()
		return null
	bubbles.call("open_entry", CODEX_ENTRY)
	for i in 3:
		await process_frame
	var layout: Node = root.get_node("/root/UiLayout")
	await _check_modal_occupant(layout, hub, "CodexHub (Histoire)")
	_collect(hub)
	# Onglet Règles : l'encyclopédie est adoptée en différé par `map_ui.gd`.
	if hub.encyclopedia != null:
		hub.open_tab(CodexHub.TAB_RULES)
		hub.encyclopedia.open_entry(ENCYCLOPEDIA_ENTRY)
		await process_frame
		await _check_modal_occupant(layout, hub, "CodexHub (Règles)")
		_collect(hub)
	else:
		print("p2c_ui_test: encyclopedia not adopted yet, Rules tab skipped")
	hub.hide()
	await process_frame
	_check(not layout.visible_occupants(layout.Zone.MODAL).has(hub), "hiding the hub should leave the modal zone")
	map.queue_free()
	# Laisse `UiZones` défaire l'hôte de la carte (plusieurs appels différés en attente) avant que
	# la fenêtre autonome n'en réclame un nouveau : sinon `_check_standalone_window` la centre sur
	# un hôte à moitié sorti et déborde de l'écran (constaté en pratique, pas un défaut réel : les
	# transitions de scène du jeu passent par plusieurs images avant tout nouvel affichage).
	for i in 6:
		await process_frame
	return null


## La fenêtre est un occupant visible de `UiZones.Zone.MODAL`, sans déborder de l'écran, à
## 1280×720 et 1920×1080 (redimensionnement possible seulement — voir `po_ui_test._resize`).
func _check_modal_occupant(layout: Node, control: Control, label: String) -> void:
	_check(control.visible and layout.visible_occupants(layout.Zone.MODAL).has(control),
		"%s should be a visible UiZones.MODAL occupant" % label)
	for resolution in C2_RESOLUTIONS:
		root.size = resolution
		await process_frame
		await process_frame
		if root.size != resolution:
			print("p2c_ui_test: window cannot be resized here (%s)" % root.size)
			continue
		var view: Vector2 = root.get_visible_rect().size
		var rect := control.get_global_rect()
		_check(rect.position.x >= -0.5 and rect.position.y >= -0.5 and rect.end.x <= view.x + 0.5 and rect.end.y <= view.y + 0.5,
			"%s at %s: %s overflows the screen %s" % [label, resolution, rect, view])


# --- CodexWindow autonome (hors de la carte, ex. en bataille) ---------------------------------


func _check_standalone_window() -> void:
	var bubbles: Node = root.get_node("/root/CodexBubbles")
	# Aucune carte en jeu (le groupe "codex_hub" est vide) : `window()` construit sa propre
	# fenêtre, désormais dans `UiZones.Zone.MODAL` (P2c).
	_check(get_first_node_in_group("codex_hub") == null, "no CodexHub should remain from the previous check")
	var window: CodexWindow = bubbles.call("window")
	window.call("open", CODEX_ENTRY)
	await process_frame
	var layout: Node = root.get_node("/root/UiLayout")
	await _check_modal_occupant(layout, window, "CodexWindow (autonome)")
	_collect(window)
	window.hide()
	await process_frame
	root.size = Vector2i(1280, 720)
	await process_frame
