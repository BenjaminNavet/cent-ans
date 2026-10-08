class_name FeudalController
extends Node

## Lot FE6 (spec FE § 6) : interface de la féodalité sur la carte de campagne.
## - Menu → « Arbre féodal » (la barre du haut est pleine en 1280 px) et bouton « Arbre féodal… » de
##   la section « Féodalité » du panneau de faction : panneau « Arbre féodal » (zone `SIDE_PANEL`), ouvert sur la
##   position du joueur — chaîne de ses suzerains, ses vassaux, pastilles d'état (loyal,
##   mécontent, félon, en révolte), loyauté au survol ; actions : se révolter, prêter hommage,
##   prononcer la commise, concéder un titre ; appels féodaux (protection, arbitrage).
## - Fil d'Ariane de la fiche de province : un clic ouvre l'arbre sur le détenteur du titre.
## - Section « Féodalité » du panneau de faction (`FeudalSection`) : son bouton ouvre l'arbre.
## - Fin de tour : événements féodaux notés (liste du panneau) et signalés quand ils touchent le
##   joueur ; appels féodaux en attente signalés.
## - Guide de trois étapes au premier lancement (`feudal_tutorial/done`).
## Tout vient du cœur (`CampaignSim.get_feudal_*`, `feudal_*`) : aucune règle ici.

const MENU_FEUDAL_ID := 930
const TREE_HEIGHT := 250.0
const RECENT_MAX := 12
## Kinds des événements du cœur portant la féodalité (hommage, commise, félonie, révolte d'un
## vassal, héritage des titres, guerre de succession).
const FEUDAL_KINDS := ["vassalage", "vassal_rebellion", "succession"]
const FEUDAL_WORDS := ["commise", "félonie", "suzerain", "vassal", "hommage", "l'ost", "protection", "déshérence", "arbitr"]
const STATUS_COLORS := {
	"loyal": Color(0.20, 0.45, 0.22), "discontent": Color(0.70, 0.45, 0.08),
	"felon": Color(0.62, 0.10, 0.08), "in_revolt": Color(0.45, 0.05, 0.30),
}
const STATUS_GLYPHS := {"loyal": "●", "discontent": "◐", "felon": "✖", "in_revolt": "⚑"}
const TUTORIAL_SCENE := "res://scenes/ui/tutorial.tscn"
const TUTORIAL_STEPS := [
	{"id": "feudal_tree", "title": "Votre place dans la féodalité",
		"text": "Au-dessus des factions, des titres : royaumes, duchés, comtés. Cliquez sur votre écu, en haut à gauche : la section « Féodalité » du panneau de faction donne vos obligations et vos objectifs, et son bouton [b]« Arbre féodal… »[/b] (ou Menu → Arbre féodal) montre votre suzerain, vos vassaux et leur loyauté. Un vassal mécontent peut refuser l'ost, se révolter ou prêter hommage ailleurs.",
		"objective": "Ouvrir l'arbre féodal.", "target": "feudal_button"},
	{"id": "feudal_filter", "title": "La carte des fiefs",
		"text": "Le filtre de carte [b]« Féodalité »[/b] (bouton « Filtres ») peint chaque royaume ; les hachures montrent les terres des grands vassaux, l'écu parti les doubles allégeances, comme la Guyenne anglaise tenue du roi de France. Dans la fiche d'une province, le fil d'Ariane donne ses titres, du royaume au comté.",
		"objective": "Choisir le filtre « Féodalité ».", "target": "filters_button"},
	{"id": "feudal_war", "title": "Avant de déclarer la guerre",
		"text": "Attaquer un vassal appelle son suzerain, puis le suzerain de celui-ci. Avant toute déclaration, le cadre [b]« Qui peut entrer en guerre »[/b] estime chaque maillon (probable, incertain, improbable). Un vassal félon peut être frappé de [b]commise[/b] : vous reprenez ses fiefs si vous gagnez. Voir le Codex, « Vassalité ».",
		"objective": "", "target": "", "manual": true},
]

var map: Node = null  # CampaignMap
var panel: PanelContainer
## Bouton « Arbre féodal… » de la section « Féodalité » du panneau de faction (cible du guide).
var button: Button
## Faction mise en avant dans l'arbre (le joueur par défaut).
var focus: String = ""
## Faction sélectionnée dans l'arbre (actions du suzerain).
var selected: String = ""
var tree: Tree
## Événements féodaux récents (plus récents en tête) : `[{text, date}]`.
var recent: Array = []
var tutorial: TutorialOverlay = null
var tutorial_index: int = -1
var persist_tutorial := true
var _box: VBoxContainer
var _position_box: VBoxContainer
var _calls_box: VBoxContainer
var _selection_box: VBoxContainer
var _recent_box: VBoxContainer
var _title: Label
var _homage_choice: OptionButton
var _grant_choice: OptionButton
var _items: Dictionary = {}  # faction → TreeItem
var _tutorial_timer := 0.0
## « Plus tard » : le guide ne se relance plus de lui-même pendant cette partie.
var _tutorial_postponed := false


func setup(campaign_map: Node) -> void:
	map = campaign_map
	_build_panel()
	UiZones.put(UiZones.Zone.SIDE_PANEL, panel)
	map.ui.register_panel(panel, PanelStack.Kind.CENTRAL)
	var menu_button: MenuButton = map.ui.get("menu_button")
	if menu_button != null:
		var popup := menu_button.get_popup()
		popup.add_item("Arbre féodal", MENU_FEUDAL_ID)
		popup.id_pressed.connect(func(id: int) -> void:
			if id == MENU_FEUDAL_ID:
				toggle())
	var province_panel: Node = map.ui.get("province_panel")
	if province_panel != null and province_panel.has_signal("breadcrumb_clicked"):
		province_panel.connect("breadcrumb_clicked", func(faction: String) -> void: open_for(faction))
	var faction_panel: Node = map.ui.get("faction_panel")
	var section: Variant = faction_panel.get("feudal_section") if faction_panel != null else null
	if section is FeudalSection:
		button = (section as FeudalSection).tree_button
		(section as FeudalSection).tree_requested.connect(func(faction: String) -> void: open_for(faction))
	_setup_tutorial()


func available() -> bool:
	return map != null and map.sim != null and map.sim.has_method("get_feudal_tree")


func player() -> String:
	return str(map.get("player_faction"))


# --- Panneau ---------------------------------------------------------------------------------


func _build_panel() -> void:
	panel = PanelContainer.new()
	panel.name = "FeudalTreePanel"
	panel.theme = load("res://scenes/ui/parchment_theme.tres")
	panel.add_theme_stylebox_override("panel", HudStyle.panel_box(10))
	# Largeur donnée par la zone `SIDE_PANEL` : aucun enfant ne doit l'élargir (lignes coupées).
	panel.custom_minimum_size = Vector2(0, 0)
	panel.hide()
	var scroll := ScrollContainer.new()
	scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	panel.add_child(scroll)
	_box = VBoxContainer.new()
	_box.add_theme_constant_override("separation", 6)
	_box.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	scroll.add_child(_box)
	var header := HBoxContainer.new()
	_box.add_child(header)
	var titles := VBoxContainer.new()
	titles.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	header.add_child(titles)
	titles.add_child(HudStyle.label("Arbre féodal", UiType.size(UiType.HEADING), HudStyle.RUBRIC))
	_title = HudStyle.label("", UiType.size(UiType.CAPTION), HudStyle.INK_SOFT)
	_title.name = "Realm"
	_title.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	_title.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	titles.add_child(_title)
	var close := Button.new()
	close.text = "×"
	RichTooltip.attach_plain(close, "close_escape")
	close.pressed.connect(panel.hide)
	header.add_child(close)
	_position_box = _section("Position")
	_calls_box = _section("Calls")
	tree = Tree.new()
	tree.name = "FeudalTree"
	tree.columns = 2
	tree.select_mode = Tree.SELECT_ROW
	tree.custom_minimum_size = Vector2(0, TREE_HEIGHT)
	tree.set_column_expand(1, false)
	tree.set_column_custom_minimum_width(1, 110)
	tree.set_column_clip_content(0, true)
	tree.item_selected.connect(func() -> void:
		var item := tree.get_selected()
		if item != null:
			select(str(item.get_metadata(0))))
	_box.add_child(tree)
	_selection_box = _section("Selection")
	_recent_box = _section("Recent")


func _section(node_name: String) -> VBoxContainer:
	var box := VBoxContainer.new()
	box.name = node_name
	box.add_theme_constant_override("separation", 3)
	_box.add_child(box)
	return box


func toggle() -> void:
	if panel.visible:
		panel.hide()
	else:
		open_for(player())


## Ouvre l'arbre sur `faction` (le joueur par défaut) : racine = son souverain, chemin déplié.
func open_for(faction: String = "") -> void:
	if not available():
		map.ui.show_toast("Féodalité indisponible avec cette simulation.", true)
		return
	focus = faction if faction != "" else player()
	selected = focus
	fill()
	panel.show()
	panel.reset_size()


## Reconstruit tout le panneau depuis le cœur.
func fill() -> void:
	var sheet: Dictionary = map.sim.call("get_feudal_sheet", focus)
	var root := str(sheet.get("sovereign", focus)) if not sheet.is_empty() else focus
	var realm: Dictionary = map.sim.call("get_feudal_sheet", root)
	_title.text = "%s — vu depuis %s" % [str(realm.get("primary_name", realm.get("name", root))), str(sheet.get("name", focus))]
	_fill_position()
	_fill_calls()
	_fill_tree(root)
	_fill_selection()
	_fill_recent()


func _clear(box: Container) -> void:
	for child in box.get_children():
		box.remove_child(child)
		child.queue_free()


func _label(box: Container, text: String, color: Color, node_name: String = "", size: String = UiType.CAPTION) -> Label:
	var label := HudStyle.label(text, UiType.size(size), color)
	if node_name != "":
		label.name = node_name
	label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	label.custom_minimum_size.x = 1.0  # retour à la ligne dans la largeur du panneau
	box.add_child(label)
	return label


## Liste déroulante qui n'impose pas la largeur de son plus long choix.
static func _compact(choice: OptionButton) -> void:
	choice.fit_to_longest_item = false
	choice.clip_text = true
	choice.custom_minimum_size.x = 150.0


func _button(box: Container, text: String, tip: String, node_name: String, action: Callable) -> Button:
	var b := RichButton.new()
	b.name = node_name
	b.text = text
	b.tooltip_text = tip
	b.pressed.connect(action)
	box.add_child(b)
	return b


## Position du joueur : suzerain, loyauté, obligations ; révolte et hommage.
func _fill_position() -> void:
	_clear(_position_box)
	var me := player()
	var sheet: Dictionary = map.sim.call("get_feudal_sheet", me)
	var duties: Dictionary = map.sim.call("get_feudal_obligations", me)
	_label(_position_box, "Votre position", HudStyle.RUBRIC, "", UiType.BODY)
	if sheet.is_empty():
		_label(_position_box, "Vous ne tenez aucun titre.", HudStyle.INK_FADED)
		return
	for entry in FeudalSection.lines(sheet, duties):
		_label(_position_box, str(entry[0]), entry[1], str(entry[2]))
	var actions := HFlowContainer.new()
	actions.name = "Actions"
	_position_box.add_child(actions)
	if str(sheet.get("liege", "")) != "":
		var revolt_button := _button(actions, "Se révolter", "Rompre avec %s : guerre d'indépendance, et un cas de félonie contre vous." % str(sheet.get("liege_name", "")), "RevoltButton", revolt)
		revolt_button.disabled = str(sheet.get("status", "")) == "in_revolt"
	var lords: Array = duties.get("homage_candidates", [])
	if not lords.is_empty():
		_homage_choice = OptionButton.new()
		_homage_choice.name = "HomageChoice"
		_compact(_homage_choice)
		for lord in lords:
			_homage_choice.add_item(str(lord.get("name", "")))
			_homage_choice.set_item_metadata(_homage_choice.item_count - 1, str(lord.get("id", "")))
		actions.add_child(_homage_choice)
		_button(actions, "Prêter hommage", "Se placer sous la protection du seigneur choisi (quitter son suzerain est une félonie).", "HomageButton",
			func() -> void: switch_allegiance(str(_homage_choice.get_item_metadata(_homage_choice.selected))))


## Appels féodaux en attente (protection d'un vassal attaqué, guerre privée à arbitrer).
func _fill_calls() -> void:
	_clear(_calls_box)
	var calls: Array = map.sim.call("get_feudal_offers") if map.sim.has_method("get_feudal_offers") else []
	_calls_box.visible = not calls.is_empty()
	if calls.is_empty():
		return
	_label(_calls_box, "Appels féodaux", HudStyle.RUBRIC, "", UiType.BODY)
	for call in calls:
		var offer := int(call.get("id", -1))
		_label(_calls_box, "%s (%s)" % [str(call.get("text", "")), FrText.count(int(call.get("expires_in", 0)), "tour")], HudStyle.INK, "Call%d" % offer)
		var row := HFlowContainer.new()
		row.name = "CallActions%d" % offer
		_calls_box.add_child(row)
		if str(call.get("kind", "")) == "protection":
			_button(row, "Intervenir", "Entrer en guerre contre %s aux côtés de votre vassal." % str(call.get("aggressor_name", "")), "Intervene",
				func() -> void: answer_call(offer, true))
			_button(row, "Se dérober", "Perte de prestige et de loyauté de vos vassaux ; le vassal peut changer d'allégeance.", "Shirk",
				func() -> void: answer_call(offer, false))
		elif str(call.get("kind", "")) == "summons":  # ADR 0146 : sommation de paix du suzerain
			_button(row, "Obéir", "Paix blanche et trêve avec %s." % str(call.get("target_name", "")), "Obey",
				func() -> void: answer_call(offer, true, "Vous faites la paix."))
			_button(row, "Passer outre", "La guerre continue ; votre loyauté envers votre suzerain baisse.", "Defy",
				func() -> void: answer_call(offer, false, "Vous passez outre."))
		else:
			_button(row, "Imposer la paix", "Paix blanche et trêve entre vos deux vassaux.", "ImposePeace",
				func() -> void: arbitrate(offer, "impose_peace", ""))
			for side in [["attacker", "attacker_name"], ["target", "target_name"]]:
				var side_id := str(call.get(side[0], ""))
				_button(row, "Soutenir %s" % str(call.get(side[1], "")), "Prendre son parti : guerre contre l'autre vassal.", "Side_%s" % side_id,
					func() -> void: arbitrate(offer, "take_side", side_id))
			_button(row, "Laisser faire", "Ils vident leur querelle entre eux.", "LetBe",
				func() -> void: arbitrate(offer, "let_be", ""))


func _fill_tree(root: String) -> void:
	tree.clear()
	_items.clear()
	var data: Dictionary = map.sim.call("get_feudal_tree", root)
	if data.is_empty():
		return
	var chain := {}
	var focus_sheet: Dictionary = map.sim.call("get_feudal_sheet", focus)
	for link in focus_sheet.get("liege_chain", []):
		chain[str(link.get("id", ""))] = true
	_add_node(null, data, chain, 0)
	var item: TreeItem = _items.get(selected, _items.get(focus))
	if item != null:
		item.select(0)
		tree.scroll_to_item(item)


func _add_node(parent: TreeItem, node: Dictionary, chain: Dictionary, depth: int) -> void:
	var item := tree.create_item(parent)
	var id := str(node.get("faction", ""))
	_items[id] = item
	item.set_metadata(0, id)
	var titles := PackedStringArray()
	for title in node.get("titles", []):
		titles.append(str(title.get("name", "")))
	var mine := bool(node.get("player", false))
	item.set_text(0, "%s%s" % [str(node.get("name", id)), " (vous)" if mine else ""])
	var arms := PortraitLoader.heraldry_texture(id)
	if arms != null:
		item.set_icon(0, arms)
		item.set_icon_max_width(0, 18)
	var status := str(node.get("status", ""))
	var loyalty := int(node.get("loyalty", -1))
	var tip := ", ".join(titles)
	if loyalty >= 0:
		tip += "\nLoyauté envers son suzerain : %d (%s)" % [loyalty, str(node.get("status_label", ""))]
	item.set_tooltip_text(0, tip)
	if status != "":
		item.set_text(1, "%s %s" % [STATUS_GLYPHS.get(status, ""), str(node.get("status_label", ""))])
		item.set_custom_color(1, STATUS_COLORS.get(status, HudStyle.INK))
		item.set_tooltip_text(1, "Loyauté %d" % loyalty if loyalty >= 0 else "")
	if mine or id == focus:
		item.set_custom_color(0, HudStyle.RUBRIC)
	var vassals: Array = node.get("vassals", [])
	for vassal in vassals:
		_add_node(item, vassal, chain, depth + 1)
	# Déplié sur la position mise en avant (ses suzerains et elle-même), replié ailleurs.
	item.collapsed = not vassals.is_empty() and not (depth == 0 or chain.has(id) or id == focus)


## Ligne de l'arbre de `faction` (tests).
func tree_item(faction: String) -> TreeItem:
	return _items.get(faction)


func select(faction: String) -> void:
	selected = faction
	_fill_selection()


## Fiche et actions sur la faction sélectionnée : commise, concession de titre.
func _fill_selection() -> void:
	_clear(_selection_box)
	if selected == "":
		return
	var sheet: Dictionary = map.sim.call("get_feudal_sheet", selected)
	if sheet.is_empty():
		return
	_label(_selection_box, str(sheet.get("name", selected)), HudStyle.RUBRIC, "SelectedName", UiType.BODY)
	var liege := str(sheet.get("liege_name", ""))
	var line := str(sheet.get("primary_name", ""))
	if liege != "":
		line += " — vassal de %s, loyauté %d (%s)" % [liege, int(sheet.get("loyalty", 0)), str(sheet.get("status_label", ""))]
	else:
		line += " — souverain"
	_label(_selection_box, line, HudStyle.INK_SOFT, "SelectedLine")
	if str(sheet.get("liege", "")) != player():
		return
	var duties: Dictionary = map.sim.call("get_feudal_obligations", player())
	var actions := HFlowContainer.new()
	actions.name = "SelectionActions"
	_selection_box.add_child(actions)
	var felon := false
	for case in duties.get("felons", []):
		felon = felon or str(case.get("vassal", "")) == selected
	var tip := "Guerre contre ce seul vassal félon : ses fiefs vous reviennent si vous gagnez." if felon \
		else "Aucun cas de félonie ouvert contre ce vassal."
	var target := selected
	var commise := _button(actions, "Prononcer la commise", tip, "CommiseButton", func() -> void: declare_commise(target))
	commise.disabled = not felon
	var titles: Array = duties.get("grantable_titles", [])
	if titles.is_empty():
		return
	_grant_choice = OptionButton.new()
	_grant_choice.name = "GrantChoice"
	_compact(_grant_choice)
	for title in titles:
		_grant_choice.add_item(str(title.get("name", "")))
		_grant_choice.set_item_metadata(_grant_choice.item_count - 1, str(title.get("id", "")))
	actions.add_child(_grant_choice)
	_button(actions, "Concéder", "Donner ce titre et ses terres au vassal (+ loyauté).", "GrantButton",
		func() -> void: grant_title(str(_grant_choice.get_item_metadata(_grant_choice.selected)), target))


func _fill_recent() -> void:
	_clear(_recent_box)
	_recent_box.visible = not recent.is_empty()
	if recent.is_empty():
		return
	_label(_recent_box, "Événements féodaux", HudStyle.RUBRIC, "", UiType.BODY)
	for index in recent.size():
		var entry: Dictionary = recent[index]
		_label(_recent_box, "%s — %s" % [str(entry.get("date", "")), str(entry.get("text", ""))], HudStyle.INK_SOFT, "Recent%d" % index)


# --- Actions du joueur (ordres transmis au cœur) ---------------------------------------------


func _after(result: Dictionary, success: String) -> bool:
	var ok := bool(result.get("ok", false))
	if ok:
		map.ui.show_toast(success)
	else:
		map.ui.show_toast(str(result.get("error", "Ordre refusé")), true)
	map.refresh_all()  # repeint aussi ce panneau (`refresh`)
	return ok


func declare_commise(vassal: String) -> bool:
	return _after(map.sim.call("feudal_declare_commise", vassal), "Commise prononcée : la guerre est déclarée au félon.")


func grant_title(title: String, grantee: String) -> bool:
	return _after(map.sim.call("feudal_grant_title", title, grantee), "Titre concédé.")


func revolt() -> bool:
	return _after(map.sim.call("feudal_revolt"), "Vous vous révoltez contre votre suzerain.")


func switch_allegiance(lord: String) -> bool:
	return _after(map.sim.call("feudal_switch_allegiance", lord), "Hommage prêté.")


func answer_call(offer: int, accept: bool, done := "") -> bool:
	if done == "":
		done = "Vous intervenez." if accept else "Vous vous dérobez."
	return _after(map.sim.call("answer_offer", offer, accept), done)


func arbitrate(offer: int, verdict: String, side: String) -> bool:
	return _after(map.sim.call("feudal_arbitrate", offer, verdict, side), "Arbitrage rendu.")


# --- Fin de tour -----------------------------------------------------------------------------


static func is_feudal_event(event: Dictionary) -> bool:
	if FEUDAL_KINDS.has(str(event.get("kind", ""))):
		return true
	var text := str(event.get("text_fr", "")).to_lower()
	for word in FEUDAL_WORDS:
		if text.contains(word):
			return true
	return false


## Événements féodaux du tour : notés (liste du panneau), signalés s'ils touchent le joueur.
func after_end_turn(events: Array) -> void:
	if not available():
		return
	var date := str(map.sim.call("get_date_label")) if map.sim.has_method("get_date_label") else ""
	var me := player()
	var facade := get_node_or_null("/root/SimFacade")
	var my_name := str(facade.call("faction_short_name", me)) if facade != null else me
	var mine := 0
	var fresh: Array = []
	for event in events:
		if not (event is Dictionary) or not is_feudal_event(event):
			continue
		var text := str(event.get("text_fr", ""))
		fresh.append({"text": text, "date": date})
		if str(event.get("faction", "")) == me or text.contains(my_name):
			mine += 1
	recent = (fresh + recent).slice(0, RECENT_MAX)
	var calls: Array = map.sim.call("get_feudal_offers") if map.sim.has_method("get_feudal_offers") else []
	if not calls.is_empty():
		map.ui.show_toast("Appel féodal : %s" % str(calls[0].get("text", "")))
	elif mine > 0:
		map.ui.show_toast("%s vous concernant : voir l'arbre féodal." % FrText.count(mine, "événement féodal", "événements féodaux"))
	if panel.visible:
		fill()
	maybe_start_tutorial()  # tutoriel général terminé depuis : le guide féodal prend la suite


func refresh() -> void:
	if panel != null and panel.visible and available():
		fill()


# --- Guide de trois étapes ------------------------------------------------------------------


func _setup_tutorial() -> void:
	persist_tutorial = not capture_mode()
	tutorial = (load(TUTORIAL_SCENE) as PackedScene).instantiate()
	tutorial.name = "FeudalTutorial"
	tutorial.hide()
	PanelStack.set_tier(tutorial, PanelStack.Tier.TUTORIAL)
	map.ui.add_child(tutorial)
	tutorial.continue_pressed.connect(advance_tutorial)
	tutorial.skip_step_pressed.connect(advance_tutorial)
	tutorial.skip_all_pressed.connect(finish_tutorial)
	tutorial.later_pressed.connect(func() -> void:
		_tutorial_postponed = true
		tutorial_index = -1
		tutorial.hide())
	tutorial.step_chosen.connect(func(index: int) -> void: _enter_tutorial(index))


## Captures et vues de flux (mêmes arguments que `TutorialController.capture_mode`) : le guide ne
## démarre pas et ne note rien.
static func capture_mode() -> bool:
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--screenshot=") or arg.begins_with("--flow-stage=") or arg.begins_with("--loading-shot="):
			return true
	return false


func _setting(key: String, fallback: Variant) -> Variant:
	var settings := get_node_or_null("/root/Settings")
	return settings.call("get_value", key) if settings != null else fallback


## Démarre le guide féodal au premier lancement, une fois le tutoriel général terminé ou passé.
func maybe_start_tutorial() -> void:
	if not available() or not persist_tutorial or _tutorial_postponed or tutorial_index >= 0 \
			or bool(_setting("feudal_tutorial/done", false)):
		return
	var general: Node = map.get("tutorial")
	if general != null and bool(general.get("active")):
		return
	start_tutorial()


func start_tutorial() -> void:
	var titles := PackedStringArray()
	for step in TUTORIAL_STEPS:
		titles.append(str(step["title"]))
	tutorial.set_steps(titles)
	_enter_tutorial(0)


func _enter_tutorial(index: int) -> void:
	tutorial_index = index
	tutorial.show_step(TUTORIAL_STEPS[index], index, TUTORIAL_STEPS.size())


func tutorial_active() -> bool:
	return tutorial_index >= 0


func advance_tutorial() -> void:
	if tutorial_index < 0:
		return
	if tutorial_index + 1 >= TUTORIAL_STEPS.size():
		finish_tutorial()
	else:
		_enter_tutorial(tutorial_index + 1)


func finish_tutorial() -> void:
	tutorial_index = -1
	tutorial.hide()
	tutorial.set_target({})
	var settings := get_node_or_null("/root/Settings")
	if settings != null and persist_tutorial:
		settings.call("set_value", "feudal_tutorial/done", true)


## Objectif de l'étape courante rempli ?
func tutorial_objective_met() -> bool:
	if tutorial_index < 0:
		return false
	match str(TUTORIAL_STEPS[tutorial_index]["id"]):
		"feudal_tree":
			return panel.visible
		"feudal_filter":
			var modes: Node = map.get("map_modes")
			return modes != null and str(modes.get("mode")) == "feudal"
	return false


func _process(delta: float) -> void:
	if tutorial_index < 0 or tutorial == null:
		return
	var target := str(TUTORIAL_STEPS[tutorial_index].get("target", ""))
	var control: Control = null
	if target == "feudal_button":
		# Le bouton du panneau de faction s'il est ouvert, sinon l'écu qui ouvre ce panneau.
		control = button if button != null and button.is_visible_in_tree() else map.ui.get("faction_swatch")
	if target == "filters_button":
		var modes: Node = map.get("map_modes")
		control = modes.get("button") if modes != null else null
	tutorial.set_target({"rect": control.get_global_rect()} if control != null and control.is_visible_in_tree() else {})
	_tutorial_timer -= delta
	if _tutorial_timer > 0.0:
		return
	_tutorial_timer = 0.25
	if tutorial_objective_met():
		tutorial.mark_done()
		advance_tutorial()
