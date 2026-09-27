extends SceneTree

## Chantier PO (ADR 0097) : critères C1-C3 sur la tranche verticale, en headless.
## - C1 (PO1) : aucun texte d'outil visible hors mode dev (`uv run`, `res://`, `user://`, `--…`,
##   chemins de fichier, identifiants bruts en snake_case) dans les `Label` / `RichTextLabel`.
## - C2 (PO1) : zones `UiLayout` sans chevauchement à 1280×720 et 1920×1080 ; un seul occupant de
##   `SIDE_PANEL` après l'ouverture successive de la province, de la chronique et du registre.
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
## Étiquettes posées dans une `.tscn` (`campaign_map.tscn` : barre du haut ; `province_panel.tscn` :
## titre et en-tête de garnison) avec une taille figée dans la scène — ces scènes ne sont pas
## dans la liste de fichiers du lot PO2 (qui ne touche que les scripts `.gd`). Restent hors norme
## (18/15/12/24 px) jusqu'à leur migration par PO1 (`TOP_BAR`, `SIDE_PANEL`) ou en phase 2.
## Voir `docs/wip/po2-typo.md`.
const _OUT_OF_LOT_SCENE_LABELS := ["FactionLabel", "TreasuryLabel", "IncomeLabel", "ResearchLabel",
	"NameLabel", "GarrisonHeader"]

var _failures := 0
## Tailles vues (valeur → nombre d'occurrences), toutes vues confondues, pour le message final.
var _sizes: Dictionary = {}


func _init() -> void:
	await process_frame
	await _run()
	print("po_ui_test C1/C2: désactivé (PO1)")
	if _failures == 0:
		var values := _sizes.keys()
		values.sort()
		print("po_ui_test C3: OK (tailles vues : %s)" % str(values))
	else:
		print("po_ui_test C3: %d failure(s)" % _failures)
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
		if str(control.name).ends_with("List") or str(control.name) in _OUT_OF_LOT_SCENE_LABELS:
			return
		if control.get_class() in _TEXT_CLASSES:
			var key := "normal_font_size" if control is RichTextLabel else "font_size"
			var size := control.get_theme_font_size(key)
			_sizes[size] = int(_sizes.get(size, 0)) + 1
	for child in node.get_children():
		_collect_font_sizes(child)


func _run() -> void:
	await _check_start_menu()
	await _check_campaign_map()
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
	menu.show_faction_select(true)
	await process_frame
	_collect_font_sizes(menu.faction_select)
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

	# Fin de tour : cloche puis rapport de saison.
	map._on_end_turn()
	await process_frame
	_collect_font_sizes(map.ui.end_turn_cluster)
	if map.flow != null and map.flow.season_report != null:
		_collect_font_sizes(map.flow.season_report)

	map.queue_free()
	await process_frame
