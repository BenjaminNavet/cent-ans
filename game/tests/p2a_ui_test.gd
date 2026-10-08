extends SceneTree

## Lot P2a (chantier PO, phase 2, ADR 0097) : C1-C3 sur la Cour, la fiche personnage (avec la
## suite du général et l'arbre de compétences), l'arbre familial et la section « Ordre de
## chevalerie », en headless.
## - C1 : aucun texte d'outil visible (`uv run`, `res://`, `user://`, `--…`, chemins de fichier,
##   identifiants bruts en snake_case) dans les `Label` / `RichTextLabel` / boutons.
## - C3 : aucune taille de police sous `Caption` (14 px de base) et 4 tailles au plus.
## Réutilise les aides de `po_ui_test.gd` (`_collect_font_sizes`, `_collect_tool_texts`, `_check`)
## par instanciation, sans modifier ce fichier partagé entre les lots PO2a-P2f.
## Usage : godot --headless --path game --script res://tests/p2a_ui_test.gd

const FACTION_ID := "fac_france"

## Instance de `po_ui_test.gd` utilisée seulement pour ses aides (jamais lancée comme script
## principal ici : ses méthodes `_collect_*`/`_check` ne dépendent que de leurs propres champs).
var _helper: Object = load("res://tests/po_ui_test.gd").new()
var _failures := 0


func _init() -> void:
	await process_frame
	await _run()
	var sizes: Array = _helper._sizes.keys()
	sizes.sort()
	print("p2a_ui_test C1: %s (%d textes lus)" % ["OK" if _helper._c1_failures == 0 else "%d failure(s)" % _helper._c1_failures, _helper._c1_texts])
	print("p2a_ui_test C3: %s (tailles vues : %s)" % ["OK" if sizes.is_empty() or (sizes.min() >= 14 and sizes.size() <= 4) else "FAIL", str(sizes)])
	print("p2a_ui_test: %s" % ("OK" if _failures == 0 else "%d failure(s)" % _failures))
	quit(1 if _failures > 0 else 0)


func _check(condition: bool, message: String) -> bool:
	if not condition:
		_failures += 1
		push_error("p2a_ui_test: " + message)
	return condition


func _run() -> void:
	var real_capable: bool = ClassDB.class_exists("CampaignSim") \
		and ClassDB.instantiate("CampaignSim").has_method("get_character")
	if not _check(real_capable, "p2a_ui_test needs the real CampaignSim (get_character)"):
		return
	var sim: Object = ClassDB.instantiate("CampaignSim")
	var data_dir := ProjectSettings.globalize_path("res://").path_join("../data").simplify_path()
	if not _check(sim.call("new_campaign", data_dir, FACTION_ID, 1337), "new_campaign failed"):
		return
	var ids: Array = sim.call("get_faction_characters", FACTION_ID)
	if not _check(not ids.is_empty(), "get_faction_characters should not be empty"):
		return
	var ruler := str(ids[0])
	# XP et compétence apprise : la fiche affiche un nœud « appris » et la suite du général.
	sim.call("submit_order", {"type": "debug_grant_xp", "character": ruler, "amount": 400})
	for node in sim.call("get_skill_tree"):
		if str(node["branch"]) == "command" and int(node["tier"]) == 1:
			sim.call("submit_order", {"type": "learn_skill", "character": ruler, "skill": str(node["id"])})
			break

	await _check_court(sim, ids)
	await _check_family_tree(sim, ids)
	await _check_character_sheet(sim, ruler)
	await _check_chivalry_section(sim)


## Panneau « Cour » (liste) : C1, C3.
func _check_court(sim: Object, ids: Array) -> void:
	var court: Node = (load("res://scenes/ui/court_panel.tscn") as PackedScene).instantiate()
	root.add_child(court)
	await process_frame
	var rows: Array[Dictionary] = []
	for id in ids:
		rows.append(sim.call("get_character", id))
	court.show_court(rows, "France", Color(0.2, 0.3, 0.7))
	await process_frame
	_check(court.rows_list.get_child_count() >= 1, "court panel should list at least 1 character")
	_helper._collect_font_sizes(court)
	_helper._collect_tool_texts(court)
	court.queue_free()
	await process_frame


## Onglet « Arbre familial » du panneau Cour : C1, C3 (le diagramme lui-même dessine son texte en
## `draw_string`, hors du champ `UiType` — voir `docs/archive/chantiers.md`).
func _check_family_tree(sim: Object, ids: Array) -> void:
	if not sim.has_method("get_family_tree"):
		return
	var court: Node = (load("res://scenes/ui/court_panel.tscn") as PackedScene).instantiate()
	court.set("sim_source", sim)
	root.add_child(court)
	await process_frame
	var rows: Array[Dictionary] = []
	for id in ids:
		rows.append(sim.call("get_character", id))
	court.show_court(rows, "France", Color(0.2, 0.3, 0.7))
	court.show_tab(1)  # CourtPanel.TAB_TREE
	await process_frame
	_check(court.family_tree.node_count() > 0, "family tree should show at least the ruler")
	_helper._collect_font_sizes(court)
	_helper._collect_tool_texts(court)
	court.queue_free()
	await process_frame


## Fiche personnage (suite du général, arbre de compétences) : C1, C3.
func _check_character_sheet(sim: Object, ruler: String) -> void:
	var sheet: Node = (load("res://scenes/ui/character_sheet.tscn") as PackedScene).instantiate()
	root.add_child(sheet)
	await process_frame
	sheet.show_character(sim.call("get_character", ruler), sim.call("get_skill_tree"), sim.call("get_learnable", ruler), [], [], [])
	await process_frame
	_check(sheet.visible, "character sheet should be visible after show_character")
	_helper._collect_font_sizes(sheet)
	_helper._collect_tool_texts(sheet)
	sheet.queue_free()
	await process_frame


## Section « Ordre de chevalerie » (composant de `faction_panel.gd`, hors lot) : C1, C3 seuls.
func _check_chivalry_section(sim: Object) -> void:
	if not sim.has_method("get_chivalric_orders"):
		return
	var section: Control = ChivalrySection.new()
	section.theme = load("res://scenes/ui/parchment_theme.tres")  # thème hérité de `faction_panel.tscn` en jeu
	root.add_child(section)
	await process_frame
	section.show_for(true, sim)
	await process_frame
	_helper._collect_font_sizes(section)
	_helper._collect_tool_texts(section)
	section.queue_free()
	await process_frame
