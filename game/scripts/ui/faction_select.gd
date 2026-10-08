class_name FactionSelect
extends Control

## Lot MM1 — choix de faction, posé sur le décor 3D du menu : date de départ,
## trois grandes cartes opaques (miniature de la faction, écu, nom, accroche, souverain avec son
## portrait, défi propre à la faction), fiche détaillée de la faction choisie (introduction,
## forces, faiblesses, objectifs historiques), niveau de difficulté de la campagne (DF1 : quatre
## niveaux lus dans la sim, `get_difficulty_levels`), options avancées (graine) et barre
## d'actions (« Retour », « Commencer — <faction> »). Textes : `data/ui/front_end.json` (`factions`,
## `start_dates`) ; noms, blasons et objectifs : données des factions via `SimFacade`.
## Double-clic sur une carte : commencer.
## FE6 : deux onglets — « Départs recommandés » (cartes, `recommended_factions`) et « Toutes les
## factions » (carte de 1337 cliquable, `FactionMapPicker`, filtres par royaume et par rang).
## JR6 : entre les deux, « Défis singuliers » (`special_starts`) : grandes cartes des factions à
## mécanique propre (les Croisés), pour qu'elles ne se perdent pas dans la liste.

signal back_requested
signal start_requested(faction_id: String, seed_value: int, start_date: String)

const CARD_SIZE := Vector2(236, 360)
## A6-L11 (U2) : tailles de texte de l'écran (pixels de base ; × 0,9 à 720p donne au moins 16 px
## de corps) et largeur de la colonne de droite (fiche + difficulté).
const BODY_PX := 18
const CAPTION_PX := 16
const HEADING_PX := 22
const TITLE_PX := 28
const SIDE_WIDTH := 420.0
const ART_HEIGHT := 150.0
## Q2 : taille d'écran sous laquelle l'écran est réduit d'un bloc (trois cartes, fiche, boutons).
const FIT_SIZE := Vector2(1280.0, 720.0)

var selected_faction: String = "fac_france"
var selected_start: String = ""
## DF1 : niveau de difficulté de la campagne (`easy`, `normal`, `hard`, `very_hard`).
var selected_difficulty: String = "normal"
var start_button: Button
var back_button: Button
var seed_edit: LineEdit

var _cards: Dictionary = {}  # faction_id → PanelContainer
var _card_styles: Dictionary = {}  # faction_id → [normal, selected]
var _detail_intro: RichTextLabel
var _detail_strengths: VBoxContainer
var _detail_weaknesses: VBoxContainer
var _detail_objectives: Label
var _detail_title: Label
var _advanced_box: Control
var _content: Control = null
var _difficulty_buttons: Dictionary = {}  # id → Button
var _difficulty_levels: Dictionary = {}  # id → {label, description, effects…}
var _difficulty_description: Label = null
## FE6 : onglets (cartes / carte des factions), carte et filtres.
var start_tabs: TabContainer = null
var map_picker: FactionMapPicker = null
## JR6 : page de l'onglet « Toutes les factions » (son rang dépend des onglets présents).
var _map_page: Control = null
## JR6 : page de l'onglet « Défis singuliers », `null` si `special_starts` est vide.
var special_page: Control = null
var kingdom_filter: OptionButton = null
var rank_filter: OptionButton = null
var _faction_list: VBoxContainer = null
var _faction_buttons: Dictionary = {}  # faction_id → Button
## Q7 (tests) : nombre d'ajustements `_fit` effectués.
var fit_count := 0
## Q7 : agrandissements permis après un ajustement complet (fenêtre, faction) avant réduction seule.
const FIT_GROW_BUDGET := 3
var _fit_grow_budget := FIT_GROW_BUDGET
var _fit_queued := false
const RANK_FILTERS := [["", "Tous les rangs"], ["kingdom", "Royaumes"], ["duchy", "Duchés"], ["county", "Comtés"]]


func _ready() -> void:
	set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	mouse_filter = Control.MOUSE_FILTER_STOP
	_build()
	# Q2 : en 1280×720 le bas de l'écran (bouton « Commencer ») sortait de la fenêtre.
	resized.connect(_fit)
	# Onglets, fiche plus longue : la taille minimale du contenu peut dépasser FIT_SIZE.
	# Q7 : une fois par image au plus (en différé, la boucle se rejouait dans le même vidage de
	# la file de messages jusqu'à la saturer et faire planter le jeu en vue 1280×720).
	_content.minimum_size_changed.connect(_queue_content_fit)
	_fit.call_deferred()
	var entries := FrontEndData.start_dates()
	if not entries.is_empty():
		selected_start = str((entries[0] as Dictionary).get("id", ""))
	select(selected_faction if _cards.has(selected_faction) else _first_faction())


## Q2 : réduit l'écran d'un bloc quand la fenêtre est plus petite que sa taille minimale
## (1280×720 : cartes, fiche et boutons restent tous visibles et cliquables).
func _fit(shrink_only := false) -> void:
	fit_count += 1
	if not shrink_only:
		_fit_grow_budget = FIT_GROW_BUDGET
	if _content == null:
		return
	var view := size
	var needed := _content.get_combined_minimum_size()
	var fit_size := Vector2(maxf(FIT_SIZE.x, needed.x), maxf(FIT_SIZE.y, needed.y))
	var factor := clampf(minf(view.x / fit_size.x, view.y / fit_size.y), 0.5, 1.0)
	# Q7 : le texte replié rend la hauteur minimale dépendante de la largeur, donc du facteur ;
	# après quelques agrandissements, l'ajustement ne fait plus que réduire (sinon il oscille).
	if shrink_only:
		if factor > _content.scale.x:
			if _fit_grow_budget <= 0:
				return
			_fit_grow_budget -= 1
		elif is_equal_approx(factor, _content.scale.x):
			return
	# Ancres au-delà de 1 : la mise en page donne la taille (vue / facteur), l'échelle la réduit.
	_content.scale = Vector2.ONE * factor
	_content.anchor_left = 0.0
	_content.anchor_top = 0.0
	_content.anchor_right = 1.0 / factor
	_content.anchor_bottom = 1.0 / factor
	for side in [SIDE_LEFT, SIDE_TOP, SIDE_RIGHT, SIDE_BOTTOM]:
		_content.set_offset(side, 0.0)


func _queue_content_fit() -> void:
	if _fit_queued or not is_inside_tree():
		return
	_fit_queued = true
	get_tree().process_frame.connect(func() -> void:
		_fit_queued = false
		_fit(true), CONNECT_ONE_SHOT)


func _facade() -> Node:
	return get_node_or_null("/root/SimFacade")


func _first_faction() -> String:
	return str(_cards.keys()[0]) if not _cards.is_empty() else ""


func card_count() -> int:
	return _cards.size()


# --- Construction ------------------------------------------------------------------------------


func _build() -> void:
	# Voile : le décor reste visible mais ne gêne pas la lecture.
	var veil := ColorRect.new()
	veil.color = Color(0.02, 0.015, 0.01, 0.42)
	veil.mouse_filter = Control.MOUSE_FILTER_IGNORE
	veil.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	add_child(veil)

	var margin := MarginContainer.new()
	margin.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	_content = margin
	for side in ["left", "right"]:
		margin.add_theme_constant_override("margin_" + side, 24)
	margin.add_theme_constant_override("margin_top", 26)
	margin.add_theme_constant_override("margin_bottom", 22)
	add_child(margin)
	var column := UiBuild.vbox(14, margin)

	# En-tête : titre et date de départ.
	var header := UiBuild.hbox(24, column)
	var heading := FrontEndStyle.label("Choisissez votre couronne", TITLE_PX, Color(0.97, 0.92, 0.80), FrontEndStyle.title_font(), 8)
	heading.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	header.add_child(heading)
	header.add_child(_build_start_dates())

	# A6-L11 (U2) : à gauche les onglets (cartes, défis, carte), à droite la fiche et la difficulté,
	# pour occuper toute la largeur au lieu d'empiler trois étages centrés.
	var body_row := UiBuild.hbox(18)
	body_row.name = "ScreenBody"
	body_row.size_flags_vertical = Control.SIZE_EXPAND_FILL
	column.add_child(body_row)
	start_tabs = TabContainer.new()
	start_tabs.name = "StartTabs"
	start_tabs.custom_minimum_size = Vector2(0, CARD_SIZE.y + 40)
	start_tabs.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	start_tabs.size_flags_vertical = Control.SIZE_EXPAND_FILL
	body_row.add_child(start_tabs)
	var cards := UiBuild.hbox(14)
	cards.name = "Départs recommandés"
	cards.alignment = BoxContainer.ALIGNMENT_CENTER
	start_tabs.add_child(cards)
	for faction_id in FrontEndData.recommended():
		cards.add_child(_build_card(FrontEndData.faction(faction_id)))
	if _cards.is_empty():
		# Données d'accueil manquantes : cartes minimales des trois couronnes.
		for faction_id in ["fac_france", "fac_england", "fac_burgundy"]:
			cards.add_child(_build_card({"id": faction_id}))
	special_page = _build_special_tab()
	if special_page != null:
		start_tabs.add_child(special_page)
	_map_page = _build_map_tab()
	start_tabs.add_child(_map_page)

	var side := UiBuild.vbox(10)
	side.name = "SideColumn"
	side.custom_minimum_size = Vector2(SIDE_WIDTH, 0)
	body_row.add_child(side)
	var detail := _build_detail()
	detail.size_flags_vertical = Control.SIZE_EXPAND_FILL
	side.add_child(detail)
	side.add_child(_build_difficulty_selector())
	column.add_child(_build_actions())


## JR6 : onglet « Défis singuliers » : texte d'accroche puis les grandes cartes des factions à
## mécanique propre, centrées comme les départs recommandés.
func _build_special_tab() -> Control:
	var entry := FrontEndData.special_starts()
	if entry.is_empty():
		return null
	var page := UiBuild.vbox(8)
	page.name = str(entry["title"])
	var text := str(entry.get("text", ""))
	if text != "":
		var caption := FrontEndStyle.label(text, BODY_PX, Color(0.97, 0.92, 0.80))
		caption.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
		caption.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		caption.custom_minimum_size = Vector2(320, 0)  # A6-L11 : évite un repli à une colonne de mots
		page.add_child(caption)
	var cards := UiBuild.hbox(14)
	cards.name = "SpecialCards"
	cards.alignment = BoxContainer.ALIGNMENT_CENTER
	page.add_child(cards)
	for faction_id in entry["factions"]:
		cards.add_child(_build_card(FrontEndData.faction(str(faction_id))))
	return page


## JR6 : rang de l'onglet « Toutes les factions ».
func map_tab_index() -> int:
	return _map_page.get_index() if _map_page != null else 0


## FE6 : onglet « Toutes les factions » : filtres (royaume, rang), carte cliquable de 1337 à ses
## proportions et, à côté, la liste des factions filtrées groupées par royaume.
func _build_map_tab() -> Control:
	var page := UiBuild.vbox(6)
	page.name = "Toutes les factions"
	var filters := UiBuild.hbox(10, page)
	map_picker = FactionMapPicker.new()
	map_picker.faction_chosen.connect(select)
	filters.add_child(FrontEndStyle.label("Royaume", BODY_PX, FrontEndStyle.INK))
	kingdom_filter = OptionButton.new()
	kingdom_filter.name = "KingdomFilter"
	filters.add_child(kingdom_filter)
	filters.add_child(FrontEndStyle.label("Rang", BODY_PX, FrontEndStyle.INK))
	rank_filter = OptionButton.new()
	rank_filter.name = "RankFilter"
	for entry in RANK_FILTERS:
		rank_filter.add_item(str(entry[1]))
		rank_filter.set_item_metadata(rank_filter.item_count - 1, str(entry[0]))
	filters.add_child(rank_filter)
	var hint := FrontEndStyle.label("Survolez une terre pour la fiche de son seigneur, cliquez pour le choisir.", CAPTION_PX, Color(0.40, 0.32, 0.22))
	hint.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	hint.horizontal_alignment = HORIZONTAL_ALIGNMENT_RIGHT
	hint.text_overrun_behavior = TextServer.OVERRUN_TRIM_ELLIPSIS
	filters.add_child(hint)
	var body := UiBuild.hbox(10)
	body.name = "MapBody"
	body.size_flags_vertical = Control.SIZE_EXPAND_FILL
	page.add_child(body)
	map_picker.size_flags_vertical = Control.SIZE_EXPAND_FILL
	body.add_child(map_picker)
	var list_scroll := ScrollContainer.new()
	list_scroll.name = "FactionListScroll"
	list_scroll.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	list_scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	body.add_child(list_scroll)
	_faction_list = UiBuild.vbox(4)
	_faction_list.name = "FactionList"
	_faction_list.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	list_scroll.add_child(_faction_list)
	# La carte prend la largeur de ses proportions à la hauteur disponible ; la liste, le reste.
	body.resized.connect(func() -> void: _fit_map_width(body))
	map_picker.load_data()
	kingdom_filter.add_item("Tous les royaumes")
	kingdom_filter.set_item_metadata(0, "")
	for kingdom in map_picker.kingdoms():
		kingdom_filter.add_item(str(kingdom[1]))
		kingdom_filter.set_item_metadata(kingdom_filter.item_count - 1, str(kingdom[0]))
	kingdom_filter.item_selected.connect(func(_i: int) -> void: _apply_map_filters())
	rank_filter.item_selected.connect(func(_i: int) -> void: _apply_map_filters())
	_rebuild_faction_list()
	return page


func _fit_map_width(body: Control) -> void:
	var aspect := map_picker.aspect_ratio()
	var width := clampf(body.size.y * aspect, 280.0, body.size.x * 0.62)
	if absf(map_picker.custom_minimum_size.x - width) > 1.0:
		map_picker.custom_minimum_size = Vector2(width, 0)


## Liste des factions qui passent les filtres, groupées par royaume ; survol = surbrillance sur
## la carte, clic = choix (comme sur la carte).
const LANDLESS_HEADER := "Sans terre"


func _rebuild_faction_list() -> void:
	if _faction_list == null:
		return
	for child in _faction_list.get_children():
		child.queue_free()
	_faction_buttons.clear()
	var groups := {}
	var order: Array = []
	for id in map_picker.visible_factions():
		var sheet: Dictionary = map_picker.sheets[id]
		var kingdom_name := str(sheet.get("kingdom_name", ""))
		if not groups.has(kingdom_name):
			groups[kingdom_name] = []
			order.append(kingdom_name)
		(groups[kingdom_name] as Array).append(id)
	# JR6 : les factions sans terre (Croisés) en tête, puis les grands royaumes (le plus de
	# factions) d'abord, puis par nom.
	order.sort_custom(func(a: String, b: String) -> bool:
		if (a == "") != (b == ""):
			return a == ""
		var count_a := (groups[a] as Array).size()
		var count_b := (groups[b] as Array).size()
		return count_a > count_b if count_a != count_b else a < b)
	for kingdom_name in order:
		# JR3 : les factions sans terre (croisés) n'ont pas de royaume.
		var header := FrontEndStyle.label(str(kingdom_name) if str(kingdom_name) != "" else LANDLESS_HEADER, BODY_PX, FrontEndStyle.GULES, FrontEndStyle.title_font())
		_faction_list.add_child(header)
		var flow := HFlowContainer.new()
		flow.add_theme_constant_override("h_separation", 6)
		flow.add_theme_constant_override("v_separation", 4)
		_faction_list.add_child(flow)
		for id in groups[kingdom_name]:
			var button := UiBuild.button(str((map_picker.sheets[id] as Dictionary).get("name", id)), func() -> void: select(id))
			button.name = "Faction_%s" % id
			button.toggle_mode = true
			button.button_pressed = id == selected_faction
			button.focus_mode = Control.FOCUS_NONE
			button.add_theme_font_size_override("font_size", CAPTION_PX)
			button.mouse_entered.connect(func() -> void: map_picker.highlight(id))
			button.mouse_exited.connect(func() -> void: map_picker.highlight(""))
			flow.add_child(button)
			_faction_buttons[id] = button
	if order.is_empty():
		_faction_list.add_child(FrontEndStyle.label("Aucune faction ne correspond à ces filtres.", CAPTION_PX, FrontEndStyle.INK))


## FE6 (captures) : onglet de la carte, `faction_id` choisi et sa fiche de survol affichée.
func stage_map(faction_id: String) -> void:
	if start_tabs == null or map_picker == null:
		return
	start_tabs.current_tab = map_tab_index()
	select(faction_id)
	await get_tree().process_frame
	await get_tree().process_frame
	map_picker._hover(faction_id, map_picker.faction_center(faction_id))


func _apply_map_filters() -> void:
	map_picker.set_filters(str(kingdom_filter.get_item_metadata(kingdom_filter.selected)), str(rank_filter.get_item_metadata(rank_filter.selected)))
	_rebuild_faction_list()


func _build_start_dates() -> Control:
	var row := UiBuild.hbox(10)
	row.size_flags_vertical = Control.SIZE_SHRINK_CENTER
	var group := ButtonGroup.new()
	var first := true
	for entry in FrontEndData.start_dates():
		var info: Dictionary = entry
		var button := Button.new()
		button.toggle_mode = true
		button.button_group = group
		button.button_pressed = first
		first = false
		button.text = "%s %d — %s" % [str(info.get("season", "")).capitalize(), int(info.get("year", 1337)), str(info.get("title", ""))]
		button.tooltip_text = str(info.get("text", ""))
		FrontEndStyle.style_action_button(button, false, HEADING_PX)
		var id := str(info.get("id", ""))
		button.pressed.connect(func() -> void: selected_start = id)
		row.add_child(button)
	return row


func _build_card(entry: Dictionary) -> Control:
	var faction_id := str(entry.get("id", ""))
	var facade := _facade()
	var info: Dictionary = facade.call("faction_info", faction_id) if facade != null else {}
	var color: Color = info.get("color", Color(0.4, 0.4, 0.5))
	var panel := PanelContainer.new()
	panel.custom_minimum_size = CARD_SIZE
	panel.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	panel.mouse_filter = Control.MOUSE_FILTER_STOP
	panel.mouse_default_cursor_shape = Control.CURSOR_POINTING_HAND
	var normal := FrontEndStyle.vellum_panel(FrontEndStyle.GOLD_DARK)
	normal.set_content_margin_all(0)
	var chosen := normal.duplicate() as StyleBoxFlat
	chosen.border_color = FrontEndStyle.GOLD
	chosen.set_border_width_all(4)
	chosen.shadow_color = Color(FrontEndStyle.GOLD, 0.45)
	chosen.shadow_size = 18
	panel.add_theme_stylebox_override("panel", normal)
	_card_styles[faction_id] = [normal, chosen]
	var box := UiBuild.vbox(0)
	box.mouse_filter = Control.MOUSE_FILTER_IGNORE
	panel.add_child(box)

	# Miniature de la faction, bandeau aux couleurs, écu en surimpression.
	var art_holder := Control.new()
	art_holder.custom_minimum_size = Vector2(0, ART_HEIGHT)
	art_holder.clip_contents = true
	art_holder.mouse_filter = Control.MOUSE_FILTER_IGNORE
	box.add_child(art_holder)
	var art_path := str(entry.get("illustration", "res://assets/illustrations/%s.jpg" % faction_id))
	var art_texture := PortraitLoader.load_texture(art_path)
	if art_texture != null:
		var art := TextureRect.new()
		art.texture = art_texture
		art.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
		art.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_COVERED
		art.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
		art.mouse_filter = Control.MOUSE_FILTER_IGNORE
		art_holder.add_child(art)
	else:
		# JR6 : sans miniature (Croisés), l'écu de la faction en grand sur fond d'encre.
		var ground := ColorRect.new()
		ground.color = FrontEndStyle.INK
		ground.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
		ground.mouse_filter = Control.MOUSE_FILTER_IGNORE
		art_holder.add_child(ground)
		var arms_texture := PortraitLoader.heraldry_texture(faction_id)
		if arms_texture != null:
			var arms := TextureRect.new()
			arms.name = "ArtShield"
			arms.texture = arms_texture
			arms.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
			arms.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_CENTERED
			arms.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
			arms.offset_top = 18.0
			arms.offset_bottom = -18.0
			arms.mouse_filter = Control.MOUSE_FILTER_IGNORE
			art_holder.add_child(arms)
	var shade := ColorRect.new()
	shade.color = Color(0, 0, 0, 0.0)
	shade.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	shade.mouse_filter = Control.MOUSE_FILTER_IGNORE
	art_holder.add_child(shade)
	var band := ColorRect.new()
	band.color = color
	band.custom_minimum_size = Vector2(0, 6)
	band.mouse_filter = Control.MOUSE_FILTER_IGNORE
	box.add_child(band)
	var shield_texture := PortraitLoader.heraldry_texture(faction_id)

	var body := MarginContainer.new()
	body.mouse_filter = Control.MOUSE_FILTER_IGNORE
	for side in ["left", "right", "top", "bottom"]:
		body.add_theme_constant_override("margin_" + side, 12)
	box.add_child(body)
	var text_box := UiBuild.vbox(4)
	text_box.mouse_filter = Control.MOUSE_FILTER_IGNORE
	body.add_child(text_box)
	var title_row := UiBuild.hbox(10)
	title_row.mouse_filter = Control.MOUSE_FILTER_IGNORE
	text_box.add_child(title_row)
	if shield_texture != null:
		var shield := TextureRect.new()
		shield.texture = shield_texture
		shield.custom_minimum_size = Vector2(46, 54)
		shield.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
		shield.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_CENTERED
		shield.mouse_filter = Control.MOUSE_FILTER_IGNORE
		title_row.add_child(shield)
	var names := UiBuild.vbox(-2)
	names.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	names.mouse_filter = Control.MOUSE_FILTER_IGNORE
	title_row.add_child(names)
	var name_text := str(info.get("name", faction_id)) if not info.is_empty() else faction_id
	var name_label := FrontEndStyle.label(name_text, HEADING_PX, FrontEndStyle.INK, FrontEndStyle.title_font())
	name_label.mouse_filter = Control.MOUSE_FILTER_IGNORE
	name_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART  # A6-L11 : la carte reste étroite
	names.add_child(name_label)
	var tagline := FrontEndStyle.label(str(entry.get("tagline", "")), BODY_PX, FrontEndStyle.GULES, FrontEndStyle.body_italic())
	tagline.mouse_filter = Control.MOUSE_FILTER_IGNORE
	tagline.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	names.add_child(tagline)

	text_box.add_child(_ruler_row(str(info.get("ruler", ""))))
	text_box.add_child(_difficulty_row(int(entry.get("difficulty", 1))))

	panel.gui_input.connect(func(event: InputEvent) -> void:
		if event is InputEventMouseButton and event.pressed and event.button_index == MOUSE_BUTTON_LEFT:
			select(faction_id)
			if event.double_click:
				_on_start())
	panel.mouse_entered.connect(func() -> void:
		if faction_id != selected_faction:
			shade.color = Color(1, 0.9, 0.6, 0.08))
	panel.mouse_exited.connect(func() -> void: shade.color = Color(0, 0, 0, 0.0))
	_cards[faction_id] = panel
	return panel


func _ruler_row(ruler_id: String) -> Control:
	var row := UiBuild.hbox(10)
	row.mouse_filter = Control.MOUSE_FILTER_IGNORE
	if ruler_id == "":
		return row
	var portrait := PortraitLoader.portrait_texture(ruler_id)
	if portrait != null:
		var frame := PanelContainer.new()
		var style := StyleBoxFlat.new()
		style.bg_color = FrontEndStyle.GOLD_DARK
		style.set_content_margin_all(2)
		frame.add_theme_stylebox_override("panel", style)
		frame.mouse_filter = Control.MOUSE_FILTER_IGNORE
		var picture := TextureRect.new()
		picture.texture = portrait
		picture.custom_minimum_size = Vector2(64, 64)
		picture.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
		picture.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_COVERED
		picture.mouse_filter = Control.MOUSE_FILTER_IGNORE
		frame.add_child(picture)
		row.add_child(frame)
	var character := _character(ruler_id)
	var texts := VBoxContainer.new()
	texts.alignment = BoxContainer.ALIGNMENT_CENTER
	texts.mouse_filter = Control.MOUSE_FILTER_IGNORE
	texts.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	row.add_child(texts)
	var caption := FrontEndStyle.label("Souverain", CAPTION_PX, FrontEndStyle.FADED_INK, FrontEndStyle.body_italic())
	texts.add_child(caption)
	var name := str(character.get("name", ruler_id.trim_prefix("chr_").capitalize()))
	var ruler_label := FrontEndStyle.label(name, HEADING_PX, FrontEndStyle.INK, FrontEndStyle.title_font())
	ruler_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	texts.add_child(ruler_label)
	var titles: Array = Array(character.get("titles", PackedStringArray()))
	if not titles.is_empty():
		var title_label := FrontEndStyle.label(str(titles[0]), CAPTION_PX, FrontEndStyle.FADED_INK)
		texts.add_child(title_label)
	for child in texts.get_children():
		(child as Control).mouse_filter = Control.MOUSE_FILTER_IGNORE
	return row


func _character(character_id: String) -> Dictionary:
	var facade := _facade()
	if facade == null:
		return {}
	var store: Variant = facade.get("store")
	if store is Object and (store as Object).has_method("get_character"):
		return (store as Object).call("get_character", character_id)
	return {}


## Défi propre à la faction (indicatif : situation historique de départ), à ne pas confondre avec
## le niveau de difficulté de la campagne choisi sous la fiche.
func _difficulty_row(level: int) -> Control:
	var row := UiBuild.hbox(6)
	row.mouse_filter = Control.MOUSE_FILTER_PASS
	RichTooltip.attach_plain(row, "faction_challenge_indicative")
	row.add_child(FrontEndStyle.label("Défi :", CAPTION_PX, FrontEndStyle.FADED_INK, FrontEndStyle.body_italic()))
	var pips := UiBuild.hbox(4)
	pips.size_flags_vertical = Control.SIZE_SHRINK_CENTER
	for i in 4:
		var pip := ColorRect.new()
		pip.custom_minimum_size = Vector2(12, 12)
		pip.color = FrontEndStyle.GULES if i <= level else Color(0.7, 0.64, 0.52)
		pip.rotation = PI / 4.0
		pip.pivot_offset = Vector2(6, 6)
		pip.mouse_filter = Control.MOUSE_FILTER_IGNORE
		pips.add_child(pip)
	row.add_child(pips)
	row.add_child(FrontEndStyle.label(FrontEndData.difficulty_label(level), BODY_PX, FrontEndStyle.GULES, FrontEndStyle.title_font()))
	for child in row.get_children():
		(child as Control).mouse_filter = Control.MOUSE_FILTER_IGNORE
	return row


func _build_detail() -> Control:
	var panel := PanelContainer.new()
	var style := FrontEndStyle.vellum_panel(FrontEndStyle.GOLD_DARK)
	style.set_content_margin_all(16)
	panel.add_theme_stylebox_override("panel", style)
	var scroll := ScrollContainer.new()
	scroll.name = "DetailScroll"
	scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	scroll.custom_minimum_size = Vector2(0, 160)
	panel.add_child(scroll)
	var left := UiBuild.vbox(8)
	left.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	scroll.add_child(left)
	_detail_title = FrontEndStyle.label("", TITLE_PX, FrontEndStyle.INK, FrontEndStyle.title_font())
	left.add_child(_detail_title)
	_detail_intro = RichTextLabel.new()
	_detail_intro.fit_content = true
	_detail_intro.scroll_active = false
	_detail_intro.bbcode_enabled = true
	_detail_intro.add_theme_font_override("normal_font", FrontEndStyle.body_font())
	_detail_intro.add_theme_font_size_override("normal_font_size", BODY_PX)
	_detail_intro.add_theme_color_override("default_color", FrontEndStyle.INK)
	left.add_child(_detail_intro)
	# BP1 : mots du Codex cliquables (bulles imbriquées) dans la description de la faction.
	var bubbles := get_node_or_null("/root/CodexBubbles")
	if bubbles != null:
		bubbles.call("attach", _detail_intro)
	_detail_objectives = FrontEndStyle.label("", BODY_PX, FrontEndStyle.GULES, FrontEndStyle.body_italic())
	_detail_objectives.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	left.add_child(_detail_objectives)
	_detail_strengths = _list_column(left, "Forces", Color(0.18, 0.36, 0.14))
	_detail_weaknesses = _list_column(left, "Faiblesses", FrontEndStyle.GULES)
	return panel


func _list_column(parent: Control, heading: String, color: Color) -> VBoxContainer:
	var column := UiBuild.vbox(6)
	column.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	parent.add_child(column)
	column.add_child(FrontEndStyle.label(heading, HEADING_PX, color, FrontEndStyle.title_font()))
	var items := UiBuild.vbox(6, column)
	items.set_meta("color", color)
	return items


# --- Difficulté de la campagne (DF1) ---------------------------------------------------------------


func _build_difficulty_selector() -> Control:
	var bar := PanelContainer.new()
	bar.add_theme_stylebox_override("panel", FrontEndStyle.night_panel(0.72))
	var column := UiBuild.vbox(6, bar)
	var heading := FrontEndStyle.label("Difficulté de la campagne", HEADING_PX, Color(0.97, 0.92, 0.80), FrontEndStyle.title_font(), 4)
	RichTooltip.attach_plain(heading, "difficulty_fixed_effects")
	heading.mouse_filter = Control.MOUSE_FILTER_PASS
	column.add_child(heading)
	var row := HFlowContainer.new()
	row.add_theme_constant_override("h_separation", 8)
	row.add_theme_constant_override("v_separation", 6)
	column.add_child(row)
	var facade := _facade()
	var levels: Array = facade.call("difficulty_levels") if facade != null and facade.has_method("difficulty_levels") else []
	var pending := str(facade.get("pending_difficulty")) if facade != null else ""
	var default_id := ""
	var group := ButtonGroup.new()
	for entry in levels:
		var level: Dictionary = entry
		var id := str(level.get("id", ""))
		if id == "":
			continue
		_difficulty_levels[id] = level
		if bool(level.get("default", false)):
			default_id = id
		var button := RichButton.new()
		button.toggle_mode = true
		button.button_group = group
		button.text = str(level.get("label", id))
		button.tooltip_text = _difficulty_tooltip(level)
		button.pressed.connect(func() -> void: select_difficulty(id))
		row.add_child(button)
		_difficulty_buttons[id] = button
	_difficulty_description = FrontEndStyle.label("", BODY_PX, Color(0.93, 0.88, 0.76), FrontEndStyle.body_italic())
	_difficulty_description.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	_difficulty_description.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	column.add_child(_difficulty_description)
	bar.visible = not _difficulty_buttons.is_empty()
	if _difficulty_levels.has(pending):
		select_difficulty(pending)
	elif _difficulty_levels.has(default_id):
		select_difficulty(default_id)
	elif not _difficulty_levels.is_empty():
		select_difficulty(str(_difficulty_levels.keys()[0]))
	return bar


## Infobulle parchemin : description et effets chiffrés du niveau.
func _difficulty_tooltip(level: Dictionary) -> String:
	var text := "[b]%s[/b]\n%s" % [str(level.get("label", "")), str(level.get("description", ""))]
	var effects: Array = Array(level.get("effects", PackedStringArray()))
	if not effects.is_empty():
		text += "\n"
		for line in effects:
			text += "\n• %s" % str(line)
	return text


func select_difficulty(id: String) -> void:
	if not _difficulty_buttons.has(id):
		return
	selected_difficulty = id
	for key in _difficulty_buttons:
		var button: Button = _difficulty_buttons[key]
		button.set_pressed_no_signal(key == id)
		FrontEndStyle.style_action_button(button, key == id, HEADING_PX)
	if _difficulty_description != null:
		_difficulty_description.text = str((_difficulty_levels[id] as Dictionary).get("description", ""))


func difficulty_count() -> int:
	return _difficulty_buttons.size()


func _build_actions() -> Control:
	var bar := PanelContainer.new()
	bar.add_theme_stylebox_override("panel", FrontEndStyle.night_panel(0.72))
	var row := UiBuild.hbox(16, bar)
	back_button = UiBuild.button("Retour")
	FrontEndStyle.style_action_button(back_button, false, HEADING_PX)
	back_button.pressed.connect(func() -> void: back_requested.emit())
	row.add_child(back_button)

	var advanced := UiBuild.button("Options avancées")
	advanced.toggle_mode = true
	FrontEndStyle.style_action_button(advanced, false, HEADING_PX)
	row.add_child(advanced)
	var seed_row := UiBuild.hbox(8)
	seed_row.visible = false
	_advanced_box = seed_row
	var seed_label := FrontEndStyle.label("Graine aléatoire", BODY_PX, Color(0.93, 0.88, 0.76), FrontEndStyle.body_italic())
	RichTooltip.attach_plain(seed_label, "seed_same_draw")
	seed_label.mouse_filter = Control.MOUSE_FILTER_PASS
	seed_row.add_child(seed_label)
	seed_edit = LineEdit.new()
	seed_edit.custom_minimum_size = Vector2(130, 0)
	var facade := _facade()
	seed_edit.text = str(facade.get("pending_seed")) if facade != null else "1337"
	seed_row.add_child(seed_edit)
	row.add_child(seed_row)
	advanced.toggled.connect(func(on: bool) -> void: seed_row.visible = on)

	var spacer := Control.new()
	spacer.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	row.add_child(spacer)
	start_button = UiBuild.button("Commencer")
	FrontEndStyle.style_action_button(start_button, true, TITLE_PX)
	start_button.pressed.connect(_on_start)
	row.add_child(start_button)
	return bar


# --- Sélection ---------------------------------------------------------------------------------


func select(faction_id: String) -> void:
	if not _cards.has(faction_id) and FrontEndData.faction(faction_id).is_empty():
		return
	selected_faction = faction_id
	_fit_grow_budget = FIT_GROW_BUDGET  # Q7 : nouvelle fiche, l'écran peut de nouveau s'agrandir
	for id in _cards:
		var panel: PanelContainer = _cards[id]
		var styles: Array = _card_styles[id]
		panel.add_theme_stylebox_override("panel", styles[1] if id == faction_id else styles[0])
		panel.modulate = Color.WHITE if id == faction_id else Color(0.82, 0.8, 0.76)
	if map_picker != null:
		map_picker.select(faction_id)
	for id in _faction_buttons:
		(_faction_buttons[id] as Button).set_pressed_no_signal(id == faction_id)
	var facade := _facade()
	var short_name := str(facade.call("faction_short_name", faction_id)) if facade != null else faction_id
	start_button.text = "Commencer — %s" % short_name
	_fill_detail(faction_id)


func _fill_detail(faction_id: String) -> void:
	var entry := FrontEndData.faction(faction_id)
	var facade := _facade()
	var info: Dictionary = facade.call("faction_info", faction_id) if facade != null else {}
	_detail_title.text = str(info.get("name", faction_id))
	_detail_intro.text = CodexText.format(str(entry.get("intro", info.get("description", ""))), true)
	var summary := str(info.get("victory_summary", ""))
	_detail_objectives.text = "Objectifs historiques (avant %d) : %s" % [int(info.get("victory_end_year", 1453)), summary] if summary != "" else ""
	_detail_objectives.visible = summary != ""
	_fill_list(_detail_strengths, entry.get("strengths", []), "✦")
	_fill_list(_detail_weaknesses, entry.get("weaknesses", []), "✧")


func _fill_list(items: VBoxContainer, lines: Array, bullet: String) -> void:
	for child in items.get_children():
		child.queue_free()
	var color: Color = items.get_meta("color", FrontEndStyle.INK)
	for line in lines:
		var row := UiBuild.hbox(8)
		var mark := FrontEndStyle.label(bullet, BODY_PX, color)
		mark.size_flags_vertical = Control.SIZE_SHRINK_BEGIN
		row.add_child(mark)
		var text := FrontEndStyle.label(str(line), BODY_PX, FrontEndStyle.INK)
		text.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		text.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		row.add_child(text)
		items.add_child(row)


func seed_value() -> int:
	return int(seed_edit.text) if seed_edit.text.is_valid_int() else seed_edit.text.hash()


func _on_start() -> void:
	start_requested.emit(selected_faction, seed_value(), selected_start)


func _unhandled_input(event: InputEvent) -> void:
	if not visible:
		return
	if event.is_action_pressed("ui_cancel"):
		get_viewport().set_input_as_handled()
		back_requested.emit()
	elif event is InputEventKey and event.pressed and not event.echo and (event.keycode == KEY_LEFT or event.keycode == KEY_RIGHT):
		var ids: Array = _cards.keys()
		var index := ids.find(selected_faction)
		index = clampi(index + (1 if event.keycode == KEY_RIGHT else -1), 0, ids.size() - 1)
		select(str(ids[index]))
		get_viewport().set_input_as_handled()
