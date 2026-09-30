extends SceneTree

## Chantier PO (ADR 0097) : critères C1-C3 sur la tranche verticale, en headless.
## - C1 (PO1) : aucun texte d'outil visible hors mode dev (`uv run`, `res://`, `user://`, `--…`,
##   chemins de fichier, identifiants bruts en snake_case) dans les `Label` / `RichTextLabel`.
## - C2 (PO1) : zones `UiLayout` sans chevauchement à 1280×720 et 1920×1080 ; un seul occupant de
##   `SIDE_PANEL` après l'ouverture successive de la province et du registre (chronique : zone modale).
## - C3 (PO2) : aucune taille de police sous `Caption` (14 px de base) et 4 tailles au plus dans
##   les contrôles visibles de la tranche (menu, choix de faction, carte de campagne : barre du
##   haut, panneau de province, panneau de colonie, bandeau d'ost, cloche de fin de tour, rapport
##   de saison). Bataille et disposition fixe : hors lot, activés par PO1/PO4/PO5.
## Usage : godot --headless --path game --script res://tests/po_ui_test.gd

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")
const PICK_PROVINCE_INDEX := 3
## Bible DA § 12.2 : Title 26, Heading 20, Body 17, Caption 14 (base 900 px). Rien en dessous ;
## 4 valeurs au plus.
const MIN_SIZE := 14
const MAX_DISTINCT_SIZES := 4
## Classes dont le texte compte pour C3 (les autres — Panel, TextureRect… — n'affichent rien).
const _TEXT_CLASSES := ["Label", "RichTextLabel", "Button", "CheckBox", "CheckButton",
	"LinkButton", "MenuButton", "OptionButton", "LineEdit"]
var _failures := 0
var _layout_failures := 0
var _c1_failures := 0
var _c1_texts := 0
var _c2_failures := 0
var _c2_checks := 0
## C1 : motifs d'un texte d'outil (bible DA § 12.5).
var _tool_patterns: Array[RegEx] = []
## C2 : résolutions de fenêtre contrôlées.
const C2_RESOLUTIONS := [Vector2i(1280, 720), Vector2i(1920, 1080)]
## Tailles vues (valeur → nombre d'occurrences), toutes vues confondues, pour le message final.
var _sizes: Dictionary = {}


func _init() -> void:
	await process_frame
	await _run()
	print("po_ui_test UiLayout: %s" % ("OK" if _layout_failures == 0 else "%d failure(s)" % _layout_failures))
	print("po_ui_test C1: %s (%d textes lus)" % ["OK" if _c1_failures == 0 else "%d failure(s)" % _c1_failures, _c1_texts])
	print("po_ui_test C2: %s (%d contrôles)" % ["OK" if _c2_failures == 0 else "%d failure(s)" % _c2_failures, _c2_checks])
	if _failures - _layout_failures - _c1_failures - _c2_failures == 0:
		var values := _sizes.keys()
		values.sort()
		print("po_ui_test C3: OK (tailles vues : %s)" % str(values))
	else:
		print("po_ui_test C3: %d failure(s)" % (_failures - _layout_failures - _c1_failures - _c2_failures))
	quit(1 if _failures > 0 else 0)


func _check(condition: bool, message: String) -> bool:
	if not condition:
		_failures += 1
		push_error("po_ui_test: " + message)
	return condition


## Parcourt les `Control` visibles sous `node` et ajoute leur taille de police à `_sizes`.
## Les listes remplies par `PanelWidgets` (garnison, constructions, recrutement, colonies —
## nommées `*List`, hors liste de fichiers du lot PO2) ne sont pas descendues : leur nettoyage
## est un suivi de phase 2 (voir `docs/wip/po2-typo.md`), pas la responsabilité de ce lot.
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


func _run() -> void:
	# Réglages de test dès le départ : le menu titre ne doit pas dépendre de la taille
	# d'interface choisie par le joueur dans son `settings.cfg`.
	var settings: Node = root.get_node_or_null("/root/Settings")
	if settings != null:
		settings.call("use_test_file")
		settings.call("_apply_ui_scale")
	for pattern in ["uv run", "res://", "user://", "(^|\\s)--[a-z]", "[\\w-]+/[\\w./-]+\\.(json|gd|tscn|png|bin|ogg|md)\\b",
			"\\b[a-z]+(_[a-z0-9]+)+\\b"]:
		var regex := RegEx.new()
		regex.compile(pattern)
		_tool_patterns.append(regex)
	await _check_ui_layout()
	await _check_start_menu()
	await _check_campaign_map()
	await _check_campaign_layout()
	_check(_sizes.is_empty() or _sizes.keys().min() >= MIN_SIZE,
		"C3: aucune taille sous %d px (base 900 px) — vues : %s" % [MIN_SIZE, str(_sizes)])
	_check(_sizes.size() <= MAX_DISTINCT_SIZES,
		"C3: au plus %d tailles distinctes, %d vues — %s" % [MAX_DISTINCT_SIZES, _sizes.size(), str(_sizes)])


## Menu-titre et choix de faction (`start_menu.gd`, `faction_select.gd`).
func _check_start_menu() -> void:
	var scene: PackedScene = load("res://scenes/start_menu.tscn")
	if not _check(scene != null, "cannot load start_menu.tscn"):
		return
	var menu: Control = scene.instantiate()
	root.add_child(menu)
	await process_frame
	_collect_font_sizes(menu)
	_collect_tool_texts(menu)
	# C2 : la colonne du menu tient à l'écran (« Crédits », « Quitter » coupés avant PO1).
	for resolution in C2_RESOLUTIONS:
		if not await _resize(resolution):
			continue
		var view: Vector2 = root.get_visible_rect().size
		var quit_rect: Rect2 = (menu.get("quit_button") as Control).get_global_rect()
		_check_c2(quit_rect.end.y <= view.y, "start menu at %s: « Quitter » ends at %.0f, screen %.0f" % [resolution, quit_rect.end.y, view.y])
	menu.show_faction_select(true)
	await process_frame
	_collect_font_sizes(menu.faction_select)
	_collect_tool_texts(menu.faction_select)
	menu.queue_free()
	await process_frame


## Carte de campagne : barre du haut, panneau de province, panneau de colonie, bandeau d'ost,
## cloche de fin de tour, rapport de saison (`map_ui.gd`, `province_panel.gd`,
## `settlement_panel.gd`, `army_strip.gd`, `end_turn_cluster.gd`, `season_report.gd`).
func _check_campaign_map() -> void:
	var settings: Node = root.get_node_or_null("/root/Settings")
	if settings != null:
		settings.call("use_test_file")
		settings.call("set_value", "game/autosave_interval", 0, false)
		settings.call("set_value", "tutorial/enabled", false, false)
		settings.call("set_value", "interface/season_report", true, false)
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
	# Barre du haut seulement : `map.ui` porte aussi des panneaux d'autres lots (lettres, sceau du
	# chef, chronique…) hors tranche PO2.
	_collect_font_sizes(map.ui.get_node("TopBar"))

	# Province.
	var picker: Node = map.get("picker")
	picker.call("select_index", PICK_PROVINCE_INDEX)
	await process_frame
	_collect_font_sizes(map.ui.province_panel)

	# Colonie : première de la province sélectionnée.
	var overview: Dictionary = map.sim.call("get_holdings_overview", map.player_faction)
	var provinces: Array = overview.get("provinces", [])
	if _check(not provinces.is_empty(), "the player should start with provinces"):
		var settlements: Array = provinces[0].get("settlements", [])
		if _check(not settlements.is_empty(), "the first province should have a settlement"):
			var settlement_id := str((settlements[0] as Dictionary).get("id", ""))
			map.settlements_ctl.open_settlement(settlement_id, true)
			await process_frame
			_collect_font_sizes(map.settlements_ctl.panel)

	# Armée sélectionnée : bandeau d'ost.
	var army_ids: PackedStringArray = map.player_army_ids()
	if _check(not army_ids.is_empty(), "the player should start with an army"):
		map.select_army(str(army_ids[0]))
		await process_frame
		_collect_font_sizes(map.ui.army_strip)
		_collect_tool_texts(map.ui)

	# Fin de tour : cloche puis rapport de saison.
	map._on_end_turn()
	await process_frame
	_collect_font_sizes(map.ui.end_turn_cluster)
	if map.flow != null and map.flow.season_report != null:
		_collect_font_sizes(map.flow.season_report)

	map.queue_free()
	await process_frame


# --- UiLayout (lot PO1) ----------------------------------------------------------------


func _check_layout(condition: bool, message: String) -> bool:
	if not condition:
		_layout_failures += 1
	return _check(condition, "UiLayout: " + message)


## Zones, exclusivité du panneau latéral, voile modal, pile d'avis.
func _check_ui_layout() -> void:
	var layout: Node = root.get_node("/root/UiLayout")
	var host := CanvasLayer.new()
	host.name = "LayoutTestHost"
	root.add_child(host)
	layout.attach_host(host)
	await process_frame
	var view: Vector2 = root.get_visible_rect().size
	# Rectangles : proportions de l'écran, zones du bord de l'écran sans chevauchement.
	var top: Rect2 = layout.zone_rect(layout.Zone.TOP_BAR)
	_check_layout(top.is_equal_approx(Rect2(0, 0, view.x, view.y * 0.08)), "TOP_BAR rect %s" % top)
	var zone_node: Control = layout.zone_node(layout.Zone.SIDE_PANEL)
	_check_layout(zone_node.get_global_rect().is_equal_approx(layout.zone_rect(layout.Zone.SIDE_PANEL)),
		"SIDE_PANEL node %s vs rect %s" % [zone_node.get_global_rect(), layout.zone_rect(layout.Zone.SIDE_PANEL)])
	# La zone ne grandit pas avec un occupant trop grand.
	var big := PanelContainer.new()
	big.custom_minimum_size = Vector2(3000, 3000)
	layout.claim(layout.Zone.SIDE_PANEL, big)
	await process_frame
	_check_layout(zone_node.get_global_rect().is_equal_approx(layout.zone_rect(layout.Zone.SIDE_PANEL)),
		"SIDE_PANEL must not grow with its occupant")
	_check_layout(zone_node.clip_contents, "zones clip their occupants")
	# Exclusivité : ouvrir un second panneau ferme le premier, même par `show()`.
	var changes: Array = []
	var on_change := func(control: Control) -> void: changes.append(control)
	layout.side_panel_changed.connect(on_change)
	var second := PanelContainer.new()
	layout.claim(layout.Zone.SIDE_PANEL, second)
	_check_layout(not big.visible and second.visible, "claiming a side panel closes the previous one")
	big.show()
	_check_layout(big.visible and not second.visible, "showing a side panel closes the other occupant")
	_check_layout(layout.visible_occupants(layout.Zone.SIDE_PANEL).size() == 1, "a single side panel")
	_check_layout(changes.size() >= 2 and changes[-1] == big, "side_panel_changed emitted (%d)" % changes.size())
	layout.side_panel_changed.disconnect(on_change)
	# Modale : voile et blocage.
	var modal := PanelContainer.new()
	layout.claim(layout.Zone.MODAL, modal)
	var dim: Control = host.get_node("UiModalDim")
	_check_layout(dim.visible and dim.mouse_filter == Control.MOUSE_FILTER_STOP, "modal dims and blocks")
	_check_layout(is_equal_approx((dim as ColorRect).color.a, 0.45), "modal dim is 45 %")
	modal.hide()
	_check_layout(not dim.visible, "dim disappears with the modal")
	# Avis : 3 visibles au plus, « + N », clic et délai.
	for i in 5:
		layout.toast("Avis %d" % i, "", 0.05)
	var shown := 0
	for entry in layout.toasts():
		if entry.visible:
			shown += 1
	_check_layout(shown == 3 and layout.folded_toasts() == 2, "3 toasts visible, 2 folded (%d / %d)" % [shown, layout.folded_toasts()])
	await create_timer(0.2).timeout
	_check_layout(layout.toasts().is_empty(), "toasts vanish after their delay")
	var clicked: Control = layout.toast("Cliquer", "", 0.0)
	var click := InputEventMouseButton.new()
	click.button_index = MOUSE_BUTTON_LEFT
	click.pressed = true
	clicked.gui_input.emit(click)
	_check_layout(layout.toasts().is_empty(), "a click closes a toast")
	# Libération.
	layout.release(second)
	_check_layout(second.get_parent() == null and not layout.occupants(layout.Zone.SIDE_PANEL).has(second), "release")
	second.free()
	host.queue_free()
	await process_frame


# --- C1 : textes d'outil (lot PO1) -----------------------------------------------------


## Parcourt les `Label` / `RichTextLabel` / boutons visibles sous `node` : aucun motif d'outil.
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


# --- C2 : disposition fixe (lot PO1) ---------------------------------------------------


func _check_c2(condition: bool, message: String) -> bool:
	_c2_checks += 1
	if not condition:
		_c2_failures += 1
	return _check(condition, "C2: " + message)


## Fenêtre à `resolution` (faux si l'environnement refuse le redimensionnement).
func _resize(resolution: Vector2i) -> bool:
	root.size = resolution
	await process_frame
	await process_frame
	if root.size != resolution:
		print("po_ui_test C2: window cannot be resized here (%s)" % root.size)
		return false
	return true


## Rectangles de la carte de campagne à 1280×720 et 1920×1080 ; un seul panneau latéral après
## l'ouverture successive de la province, de la chronique et du registre.
func _check_campaign_layout() -> void:
	var layout: Node = root.get_node("/root/UiLayout")
	var facade: Node = root.get_node("/root/SimFacade")
	facade.pending_faction = "fac_france"
	facade.pending_seed = 1337
	facade.pending_load_path = ""
	var map: Node3D = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	root.add_child(map)
	await process_frame
	await process_frame
	if not _check(map.get("load_ok") and map.get("sim") != null, "C2: campaign map failed to start"):
		map.queue_free()
		return
	var initial: Vector2i = root.size
	var army_ids: PackedStringArray = map.player_army_ids()
	if not army_ids.is_empty():
		map.select_army(str(army_ids[0]))
	var edge_zones := [layout.Zone.TOP_BAR, layout.Zone.BOTTOM_SELECTION, layout.Zone.MINIMAP,
		layout.Zone.SIDE_PANEL, layout.Zone.TOASTS]
	for resolution in C2_RESOLUTIONS:
		if not await _resize(resolution):
			continue
		map.ui.layout_hud()
		await process_frame
		# Zones du bord de l'écran : sans chevauchement deux à deux.
		for i in edge_zones.size():
			for j in range(i + 1, edge_zones.size()):
				var a: Rect2 = layout.zone_rect(edge_zones[i])
				var b: Rect2 = layout.zone_rect(edge_zones[j])
				_check_c2(not a.intersects(b), "%s: zones %d and %d overlap (%s / %s)" % [resolution, edge_zones[i], edge_zones[j], a, b])
		var top: Rect2 = layout.zone_rect(layout.Zone.TOP_BAR)
		var side: Rect2 = layout.zone_rect(layout.Zone.SIDE_PANEL)
		var toasts: Rect2 = layout.zone_rect(layout.Zone.TOASTS)
		var mini: Rect2 = layout.zone_rect(layout.Zone.MINIMAP)
		var bar: Rect2 = (map.ui.get_node("TopBar") as Control).get_global_rect()
		_check_c2(bar.end.y <= top.end.y + 0.5, "%s: top bar %s taller than TOP_BAR %s" % [resolution, bar, top])
		# HUD non coupé (barre, sceau, bandeau, cloche, minicarte) : jamais sur le panneau latéral
		# ni sur les avis.
		var hud := {"top bar": bar, "seal": map.ui.general_seal.get_global_rect(),
			"army strip": map.ui.army_strip.get_global_rect(), "bell": map.ui.end_turn_cluster.get_global_rect()}
		if map.ui.minimap != null and map.ui.minimap.visible:
			var mini_rect: Rect2 = map.ui.minimap.get_global_rect()
			hud["minimap"] = mini_rect
			_check_c2(mini.encloses(mini_rect.grow(-0.5)), "%s: minimap %s outside its zone %s" % [resolution, mini_rect, mini])
			_check_c2(not mini_rect.intersects(hud["bell"]), "%s: bell over the minimap" % resolution)
		for key in hud:
			var rect: Rect2 = hud[key]
			if rect.size == Vector2.ZERO:
				continue
			_check_c2(not rect.grow(-0.5).intersects(side), "%s: %s %s over SIDE_PANEL %s" % [resolution, key, rect, side])
			_check_c2(not rect.grow(-0.5).intersects(toasts), "%s: %s %s over TOASTS %s" % [resolution, key, rect, toasts])
	# Un seul occupant du panneau latéral : province, chronique, registre, puis province.
	map.picker.call("select_index", PICK_PROVINCE_INDEX)
	await process_frame
	_check_side_single(layout, map.ui.province_panel, "province")
	map.chronicle.window.show()
	await process_frame
	# Q8 : la chronique est en zone modale (620 px de large ne tiennent pas dans la zone latérale) ;
	# elle ferme toujours la fiche de province.
	_check_c2(layout.visible_occupants(layout.Zone.MODAL).has(map.chronicle.window), "chronicle should open in the MODAL zone")
	_check_c2(not map.ui.province_panel.is_visible_in_tree(), "the chronicle should close the province panel")
	map.units_ctl.toggle()
	await process_frame
	_check_side_single(layout, map.units_ctl.panel, "roster")
	map.picker.call("select_index", PICK_PROVINCE_INDEX + 1)
	await process_frame
	_check_side_single(layout, map.ui.province_panel, "province again")
	root.size = initial
	await process_frame
	map.queue_free()
	await process_frame


func _check_side_single(layout: Node, expected: Control, label: String) -> void:
	var shown: Array = layout.visible_occupants(layout.Zone.SIDE_PANEL)
	_check_c2(shown.size() == 1 and shown[0] == expected,
		"after opening the %s: side panel occupants %s" % [label, shown.map(func(c: Control) -> String: return str(c.name))])
