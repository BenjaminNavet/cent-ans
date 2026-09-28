class_name MapModeController
extends Node

## Lot MF1 : filtres de la carte de campagne, façon Total War. Un seul mode de teinte actif à la
## fois (politique, diplomatie, religion, mécontentement, richesse, population, loyauté des
## vassaux, ravitaillement, revendications) ; les routes commerciales restent un calque à part,
## combinable. Bouton « Filtres » (touche F) en tête de la rangée de la minicarte, qui ouvre le
## menu ; légende permanente en bas de l'écran ; valeur du mode dans l'infobulle de survol.
##
## Les valeurs viennent du cœur (`get_map_lens`, `get_province_relations`,
## `get_province_religion`) : ce fichier ne fait que les traduire en couleurs.

const POLITICAL := "political"
## Ordre du menu : [id, libellé, action de l'InputMap (ou ""), infobulle].
const MODES := [
	[POLITICAL, "Politique", "", "Couleurs des royaumes"],
	["diplomacy", "Diplomatie", "map_mode_diplomacy", "Vert allié, bleu accord, jaune neutre, orange tension, rouge guerre, gris vassal"],
	["religion", "Religion", "map_mode_religion", "Obédience d'Avignon ou de Rome, foyers d'hérésie"],
	["unrest", "Mécontentement", "map_toggle_unrest", "Mécontentement moyen de la population (ordre public)"],
	["wealth", "Richesse", "", "Impôt de base par saison : population, aisance, dévastation"],
	["population", "Population", "", "Nombre d'habitants de chaque province"],
	["loyalty", "Loyauté des vassaux", "", "Loyauté de chaque vassal envers son suzerain"],
	["supply", "Ravitaillement", "", "Ravitaillement gagné ou perdu par saison par une armée sans général"],
	["claims", "Revendications", "", "Provinces que vous revendiquez, ou que d'autres vous disputent"],
	# FE6 : fond du royaume, hachures du tenant direct, écu parti des doubles allégeances.
	["feudal", "Féodalité", "", "Royaumes, grands vassaux (hachures) et doubles allégeances (écu parti)"],
]

const RELATION_COLORS := {
	"self": Color(0.85, 0.75, 0.30), "war": Color(0.72, 0.12, 0.10), "truce": Color(0.85, 0.70, 0.20),
	"peace": Color(0.55, 0.55, 0.52), "alliance": Color(0.20, 0.40, 0.78), "vassal": Color(0.50, 0.25, 0.65),
	"suzerain": Color(0.50, 0.25, 0.65),
}
const RELIGION_AVIGNON := Color(0.25, 0.35, 0.70)
const RELIGION_ROME := Color(0.80, 0.65, 0.20)
const RELIGION_OTHER := Color(0.45, 0.45, 0.45)
const HERESY := Color(0.20, 0.65, 0.25)
## Rampes séquentielles (faible → fort), lisibles sur le parchemin.
const GOOD := Color(0.20, 0.55, 0.20)
const BAD := Color(0.75, 0.15, 0.10)
const WEALTH_LOW := Color(0.93, 0.88, 0.70)
const WEALTH_HIGH := Color(0.62, 0.42, 0.05)
const POPULATION_LOW := Color(0.90, 0.88, 0.80)
const POPULATION_HIGH := Color(0.30, 0.18, 0.45)
const LOYALTY_LOW := Color(0.75, 0.15, 0.10)
const LOYALTY_MID := Color(0.88, 0.62, 0.15)
const LOYALTY_HIGH := Color(0.20, 0.40, 0.78)
const NEUTRAL := Color(0.62, 0.60, 0.56)
const SUPPLY_GAIN := Color(0.22, 0.55, 0.25)
const SUPPLY_LOSS := Color(0.85, 0.55, 0.15)
const SUPPLY_STARVE := Color(0.60, 0.08, 0.06)
const CLAIM_COLORS := {
	"ours": Color(0.85, 0.68, 0.20), "against_us": Color(0.72, 0.12, 0.10), "contested": Color(0.50, 0.25, 0.65),
}

## Teinte des provinces à courte distance hors du mode politique (le rendu politique normal la
## garde discrète pour laisser voir le terrain).
const MODE_TINT_NEAR := 0.62
const MODE_TINT_FAR := 0.8
const MODE_SATURATION := 0.9
const MARKER_FONT := "res://assets/third_party/fonts/noto/NotoSansSymbols2-Subset.ttf"
const MARKER_FONT_MAIN := "res://assets/third_party/fonts/noto/NotoSansSymbols-Subset.ttf"
const MARKED_RELATIONS := ["war", "alliance", "vassal", "suzerain", "truce"]

signal mode_changed(mode: String)

var map: Node = null  # CampaignMap
var mode: String = POLITICAL
var button: Button
var menu: PopupPanel
var _menu_buttons: Dictionary = {}  # mode → Button
var _trade_check: CheckBox
var _legend: PanelContainer
var _markers: Node3D
var _tint_saved: Dictionary = {}
## Valeurs du dernier rafraîchissement, par id de province (infobulle de survol).
var _lens: Dictionary = {}
var _relations: Dictionary = {}
var _religions: Dictionary = {}
## Lot DZ : faction dont le mode Diplomatie montre les relations ("" : le joueur). Un clic sur une
## province la choisit (son contrôleur) ; un clic sur nos terres ou hors carte revient au joueur.
var focus_faction: String = ""
## FE6 : filtre « Féodalité » (couleurs, hachures, écus partis).
var feudal_lens := FeudalMapLens.new()


func setup(campaign_map: Node) -> void:
	map = campaign_map
	button = Button.new()
	button.name = "MapFiltersButton"
	button.text = "Filtres ▾"
	RichTooltip.attach_plain(button, "map_mode_filters")
	button.focus_mode = Control.FOCUS_NONE
	button.pressed.connect(toggle_menu)
	_decorate_ink(button, "map_filters", 14)  # DA5
	_build_menu()
	var minimap: Node = map.ui.get("minimap") if map.ui != null else null
	if minimap != null and minimap.has_method("add_layer_button"):
		minimap.add_layer_button(button)
	elif map.ui != null:
		map.ui.court_button.get_parent().add_child(button)
	# Le bouton « Commerce » de la rangée cède la place à la case du menu des filtres.
	var trade_button: Button = map.ui.get("trade_button") if map.ui != null else null
	if trade_button != null:
		trade_button.hide()
	if map.ui != null and map.ui.has_method("add_keycap"):
		map.ui.add_keycap(button, "map_filters_menu")


## DA5 : icône d'action à l'encre sur un bouton (or au survol) ; rien si l'icône manque.
static func _decorate_ink(target: Button, icon_id: String, size: int) -> void:
	var library := HudStyle.icon_library()
	if library == null or not bool(library.call("has_icon", icon_id)):
		return
	library.call("decorate_button", target, icon_id, size)


func _build_menu() -> void:
	menu = PopupPanel.new()
	menu.name = "MapFiltersMenu"
	menu.theme = load("res://scenes/ui/parchment_theme.tres")
	menu.add_theme_stylebox_override("panel", HudStyle.panel_box(8))
	var box := VBoxContainer.new()
	box.add_theme_constant_override("separation", 2)
	menu.add_child(box)
	box.add_child(HudStyle.label("Filtres de la carte", HudStyle.FONT_TITLE, HudStyle.RUBRIC))
	var group := ButtonGroup.new()
	for entry in MODES:
		var id: String = entry[0]
		var item := Button.new()
		item.name = "Mode_%s" % id
		item.toggle_mode = true
		item.button_group = group
		item.flat = true
		item.add_theme_color_override("font_pressed_color", HudStyle.RUBRIC)
		item.add_theme_color_override("font_hover_pressed_color", HudStyle.RUBRIC)
		item.alignment = HORIZONTAL_ALIGNMENT_LEFT
		item.focus_mode = Control.FOCUS_NONE
		var key := ShortcutSheet.first_key(str(entry[2])) if str(entry[2]) != "" else ""
		item.text = "%s   (%s)" % [entry[1], key] if key != "" else str(entry[1])
		item.tooltip_text = entry[3]
		_decorate_ink(item, "lens_" + id, 18)  # DA5
		item.pressed.connect(func() -> void:
			set_mode(id)
			menu.hide())
		box.add_child(item)
		_menu_buttons[id] = item
	box.add_child(HSeparator.new())
	_trade_check = CheckBox.new()
	_trade_check.name = "TradeLayer"
	var trade_key := ShortcutSheet.first_key("map_toggle_trade")
	_trade_check.text = "Routes commerciales   (%s)" % trade_key if trade_key != "" else "Routes commerciales"
	RichTooltip.attach_plain(_trade_check, "map_mode_trade_overlay")
	_trade_check.focus_mode = Control.FOCUS_NONE
	_trade_check.toggled.connect(func(_on: bool) -> void: map.call("_toggle_trade_layer"))
	box.add_child(_trade_check)
	_sync_menu()


func toggle_menu() -> void:
	if menu.visible:
		menu.hide()
		return
	if menu.get_parent() == null:
		map.ui.add_child(menu)
	_sync_menu()
	menu.reset_size()
	var anchor: Rect2 = button.get_global_rect()
	var menu_size := Vector2i(menu.get_contents_minimum_size())
	var viewport: Vector2 = button.get_viewport_rect().size
	# Sous le bouton, ou au-dessus s'il n'y a pas la place.
	var pos := Vector2(anchor.end.x - menu_size.x, anchor.end.y + 4)
	if pos.y + menu_size.y > viewport.y:
		pos.y = anchor.position.y - menu_size.y - 4
	pos.x = clampf(pos.x, 4.0, maxf(4.0, viewport.x - menu_size.x - 4))
	menu.popup(Rect2i(Vector2i(pos), menu_size))


func _sync_menu() -> void:
	for id in _menu_buttons:
		var item: Button = _menu_buttons[id]
		item.set_pressed_no_signal(id == mode)
		item.text = ("▸ " if id == mode else "   ") + item.text.trim_prefix("▸ ").trim_prefix("   ")
	if _trade_check != null and map != null:
		_trade_check.set_pressed_no_signal(bool(map.get("trade_mode")))


func handle_input(event: InputEvent) -> bool:
	if not (event is InputEventKey) or not event.pressed or event.echo:
		return false
	if event.is_action_pressed("map_filters_menu"):
		toggle_menu()
		return true
	for entry in MODES:
		var action: String = entry[2]
		if action != "" and event.is_action_pressed(action):
			toggle_mode(entry[0])
			return true
	return false


## Même touche deux fois = retour à la carte politique.
func toggle_mode(target: String) -> void:
	set_mode(POLITICAL if mode == target else target)


func set_mode(target: String) -> void:
	if not _available(target):
		map.ui.show_toast("Filtre indisponible avec cette simulation.", true)
		_sync_menu()
		return
	var previous := mode
	mode = target
	_sync_menu()
	_clear_relation_markers()
	if mode != "diplomacy":
		focus_faction = ""
		_set_border_override({}, "")
	_clear_feudal()
	_set_tint_boost(mode != POLITICAL)
	_show_legend()
	if mode == POLITICAL:
		_lens.clear()
		map.refresh_all()
		if previous != POLITICAL:
			map.ui.show_toast("Carte politique.")
	else:
		map.ui.hide_province()
		refresh()
	mode_changed.emit(mode)


func active() -> bool:
	return mode != POLITICAL


func _available(target: String) -> bool:
	if target == POLITICAL:
		return true
	if map == null or map.sim == null:
		return false
	match target:
		"diplomacy":
			return map.sim.has_method("get_province_relations")
		"religion":
			return map.sim.has_method("get_province_religion")
		"feudal":
			return FeudalMapLens.available(map.sim)
		_:
			return map.sim.has_method("get_map_lens")


## Après `refresh_all` (couleurs politiques déjà posées) : repeint dans le mode courant.
func refresh() -> void:
	if mode == POLITICAL or not _available(mode):
		return
	var ids := _province_ids()
	var colors := PackedColorArray()
	match mode:
		"diplomacy":
			colors = _relation_colors(ids)
		"religion":
			colors = _religion_colors(ids)
		"feudal":
			colors = _feudal_colors(ids)
		_:
			colors = _lens_colors(ids)
	map.terrain.set_province_colors(colors)
	if map.get("minimap_ctl") != null:
		map.minimap_ctl.set_province_colors(colors)


func _province_ids() -> PackedStringArray:
	var ids := PackedStringArray()
	for index in range(1, map.map_data.province_count + 1):
		ids.append(str(map.map_data.get_province(index).get("id", "")))
	return ids


# ----- Féodalité (FE6) --------------------------------------------------------------------------

func _feudal_colors(ids: PackedStringArray) -> PackedColorArray:
	var result := feudal_lens.read(map.sim, ids)
	var terrain: Node = map.get("terrain")
	if terrain != null and terrain.has_method("set_province_hatch"):
		terrain.call("set_province_hatch", result["hatch"])
	feudal_lens.place_shields(map, result["doubles"])
	return result["colors"]


func _clear_feudal() -> void:
	feudal_lens.clear_shields()
	feudal_lens.cells.clear()
	var terrain: Node = map.get("terrain") if map != null else null
	if terrain != null and terrain.has_method("set_province_hatch"):
		terrain.call("set_province_hatch", PackedColorArray())


# ----- Diplomatie et religion (M5, DP1) ---------------------------------------------------------

func _relation_colors(ids: PackedStringArray) -> PackedColorArray:
	# Lot DP2 : positions diplomatiques (allié, accord, neutre, tension, guerre, vassal).
	if DiplomaticStances.available(map.sim):
		var stances := DiplomaticStances.stances(map.sim, ids, focus_faction)
		var stance_colors := PackedColorArray()
		var symbols := PackedStringArray()
		_relations.clear()
		for index in ids.size():
			var key := stances[index] if index < stances.size() else ""
			_relations[ids[index]] = key
			stance_colors.append(DiplomaticStances.color_of(key))
			symbols.append(str(DiplomaticStances.SYMBOL_KEYS.get(key, "")))
		_place_relation_markers(ids, symbols)
		_apply_stance_borders()
		return stance_colors
	var relations: PackedStringArray = map.sim.call("get_province_relations", ids)
	var colors := PackedColorArray()
	_relations.clear()
	for index in ids.size():
		var relation := relations[index] if index < relations.size() else ""
		_relations[ids[index]] = relation
		var color: Color = RELATION_COLORS.get(relation, Color(0, 0, 0, 0))
		color.a = 0.0 if relation == "" else 1.0
		colors.append(color)
	_place_relation_markers(ids, relations)
	return colors


## Lot DZ : faction observée par le mode Diplomatie (le joueur par défaut).
func viewer() -> String:
	if focus_faction != "":
		return focus_faction
	return str(map.get("player_faction")) if map != null else ""


## Lot DZ : frontières de royaume aux couleurs de position envers la faction observée (rouge :
## ses ennemis), halo respirant autour d'elle.
func _apply_stance_borders() -> void:
	var stances := DiplomaticStances.faction_stances(map.sim, viewer())
	var colors := {}
	for faction in stances:
		var key := str(stances[faction])
		if DiplomaticStances.COLORS.has(key):
			colors[str(faction)] = DiplomaticStances.COLORS[key]
	_set_border_override(colors, viewer())


func _set_border_override(colors: Dictionary, highlight: String) -> void:
	var borders: Object = map.get("faction_borders") if map != null else null
	if borders != null and borders.has_method("set_color_override"):
		borders.call("set_color_override", colors, highlight if not colors.is_empty() else "")


## Lot DZ : clic sur une province en mode Diplomatie — son contrôleur devient la faction
## observée (nos terres, rebelles ou hors carte : retour au joueur). Vrai si la vue a changé.
func focus_on_province(province_id: String) -> bool:
	if mode != "diplomacy" or map == null or map.sim == null:
		return false
	var controller := ""
	if province_id != "":
		var state: Dictionary = map.sim.call("get_province_state", province_id)
		controller = str(state.get("controller", state.get("owner", "")))
	return set_focus_faction(controller)


func set_focus_faction(faction: String) -> bool:
	var player := str(map.get("player_faction")) if map != null else ""
	if faction == player or faction == "fac_rebels":
		faction = ""
	if faction == focus_faction:
		return false
	focus_faction = faction
	refresh()
	_show_legend()
	if map.ui != null:
		if faction == "":
			map.ui.show_toast("Carte diplomatique : vos relations.")
		else:
			map.ui.show_toast("Relations de %s : rouge ses ennemis, vert ses alliés. Cliquez vos terres pour revenir." % SimFacade.faction_short_name(faction))
	return true


func _religion_colors(ids: PackedStringArray) -> PackedColorArray:
	var colors := PackedColorArray()
	_religions.clear()
	for id in ids:
		var info: Dictionary = map.sim.call("get_province_religion", id) if id != "" else {}
		if info.is_empty():
			colors.append(Color(0, 0, 0, 0))
			continue
		var state: Dictionary = map.sim.call("get_province_state", id)
		var controller := str(state.get("controller", ""))
		var faith: Dictionary = map.sim.call("get_religion_state", controller) if controller != "" and map.sim.has_method("get_religion_state") else {}
		var religion := str(faith.get("religion", info.get("religion", "")))
		var base := RELIGION_AVIGNON  # Église (Avignon pendant le Schisme)
		if religion == "rel_catholic_rome":
			base = RELIGION_ROME
		elif religion != "rel_catholic":
			base = RELIGION_OTHER
		var heresy := float(info.get("heresy", 0)) / 100.0
		_religions[id] = info
		colors.append(base.lerp(HERESY, clampf(heresy * 1.5, 0.0, 1.0)))
	return colors


## Lot U12 : en mode daltonien, un symbole par province sur la carte diplomatique (⚔ guerre,
## ⚭ alliance, ⚜ vassal ou suzerain, ⌛ trêve), lisible sans distinguer les couleurs.
func _place_relation_markers(ids: PackedStringArray, relations: PackedStringArray) -> void:
	_clear_relation_markers()
	if not Accessibility.colorblind() or not (map is Node3D):
		return
	_markers = Node3D.new()
	_markers.name = "RelationMarkers"
	map.add_child(_markers)
	var font := FontVariation.new()
	font.base_font = load(MARKER_FONT_MAIN)
	font.fallbacks = [load(MARKER_FONT)]
	for index in mini(ids.size(), relations.size()):
		var relation := relations[index]
		if not MARKED_RELATIONS.has(relation):
			continue
		var centroid: Vector2 = map.map_data.centroid_of_id(ids[index])
		var label := Label3D.new()
		label.text = Accessibility.relation_symbol(relation)
		label.font = font
		label.font_size = 72
		label.outline_size = 14
		label.outline_modulate = Color(0.97, 0.93, 0.82)
		label.modulate = (RELATION_COLORS.get(relation, Color.BLACK) as Color).darkened(0.35)
		label.billboard = BaseMaterial3D.BILLBOARD_ENABLED
		label.no_depth_test = true
		label.fixed_size = true
		label.pixel_size = 0.0009
		label.position = Vector3(centroid.x, map.map_data.surface_world_at(centroid.x, centroid.y) + 0.5, centroid.y)
		_markers.add_child(label)


func _clear_relation_markers() -> void:
	if _markers != null and is_instance_valid(_markers):
		_markers.queue_free()
	_markers = null


# ----- Filtres calculés par le cœur (`get_map_lens`) -------------------------------------------

func _lens_colors(ids: PackedStringArray) -> PackedColorArray:
	var rows: Array = map.sim.call("get_map_lens", ids)
	_lens.clear()
	for index in mini(ids.size(), rows.size()):
		if not (rows[index] as Dictionary).is_empty():
			_lens[ids[index]] = rows[index]
	var ranks := {}
	if mode == "wealth" or mode == "population":
		ranks = _percentiles("income" if mode == "wealth" else "population")
	var colors := PackedColorArray()
	for id in ids:
		var row: Dictionary = _lens.get(id, {})
		if row.is_empty():
			colors.append(Color(0, 0, 0, 0))
			continue
		colors.append(_lens_color(row, float(ranks.get(id, 0.0))))
	return colors


## Rang de chaque province (0 = la plus faible, 1 = la plus forte) pour `key` : une échelle par
## rang, et non par valeur, sans quoi Paris écrase toutes les autres teintes.
func _percentiles(key: String) -> Dictionary:
	var ids := _lens.keys()
	ids.sort_custom(func(a: String, b: String) -> bool: return float(_lens[a].get(key, 0)) < float(_lens[b].get(key, 0)))
	var ranks := {}
	var last := maxf(1.0, ids.size() - 1.0)
	for index in ids.size():
		ranks[ids[index]] = index / last
	return ranks


## `rank` : rang de la province (richesse, population), ignoré pour les autres modes.
func _lens_color(row: Dictionary, rank: float) -> Color:
	match mode:
		"unrest":
			return GOOD.lerp(BAD, clampf(float(row.get("unrest", 0.0)) / 100.0, 0.0, 1.0))
		"wealth":
			return WEALTH_LOW.lerp(WEALTH_HIGH, rank)
		"population":
			return POPULATION_LOW.lerp(POPULATION_HIGH, rank)
		"loyalty":
			var loyalty := int(row.get("vassal_loyalty", -1))
			if loyalty < 0:
				return NEUTRAL
			var ratio := clampf(loyalty / 100.0, 0.0, 1.0)
			return LOYALTY_LOW.lerp(LOYALTY_MID, ratio * 2.0) if ratio < 0.5 else LOYALTY_MID.lerp(LOYALTY_HIGH, ratio * 2.0 - 1.0)
		"supply":
			var change := int(row.get("supply_change", 0))
			if change >= 0:
				return SUPPLY_GAIN
			# SV4 : rouge sombre à la perte d'hiver hors de nos terres (`supply_loss_winter`, cœur).
			var worst := RuleValues.value("supply_loss_winter", 0.0)
			return SUPPLY_LOSS.lerp(SUPPLY_STARVE, clampf(-change / worst, 0.0, 1.0) if worst > 0.0 else 1.0)
		"claims":
			return CLAIM_COLORS.get(str(row.get("claim", "")), NEUTRAL)
	return Color(0, 0, 0, 0)


## Texte ajouté au nom de la province survolée dans le mode courant ("" en mode politique).
func hover_text(province_id: String) -> String:
	match mode:
		"diplomacy":
			if focus_faction != "":  # DZ
				return _focus_hover_text(str(_relations.get(province_id, "")))
			return {"self": "vos terres", "war": "en guerre", "truce": "trêve", "peace": "neutre",
				"alliance": "allié", "vassal": "vassal", "suzerain": "suzerain",
				# DP2 : positions diplomatiques
				"ally": "allié", "agreement": "en paix, avec un accord", "neutral": "neutre",
				"tension": "tensions (hostilité, embargo ou intrusion)"}.get(str(_relations.get(province_id, "")), "")
		"religion":
			var info: Dictionary = _religions.get(province_id, {})
			if info.is_empty():
				return ""
			var text := str(info.get("religion_name", ""))
			if int(info.get("heresy", 0)) > 0:
				text += ", hérésie %d %%" % int(info.get("heresy", 0))
			return text
		"feudal":
			return feudal_lens.hover_text(province_id)
	var row: Dictionary = _lens.get(province_id, {})
	if row.is_empty():
		return ""
	match mode:
		"unrest":
			return "mécontentement %d %%" % roundi(float(row.get("unrest", 0.0)))
		"wealth":
			return "%d livres par saison (impôt de base)" % roundi(float(row.get("income", 0.0)))
		"population":
			return "%s habitants" % HudStyle.thousands(int(row.get("population", 0)))
		"loyalty":
			var loyalty := int(row.get("vassal_loyalty", -1))
			if loyalty < 0:
				return "pas un vassal"
			return "loyauté %d envers %s" % [loyalty, SimFacade.faction_short_name(str(row.get("suzerain", "")))]
		"supply":
			var change := int(row.get("supply_change", 0))
			return "ravitaillement +%d par saison" % change if change >= 0 else "attrition : ravitaillement %d par saison" % change
		"claims":
			return {"ours": "vous la revendiquez", "against_us": "revendiquée contre vous",
				"contested": "revendiquée par vous et par un autre"}.get(str(row.get("claim", "")), "")
	return ""


## DZ : valeur de survol vue de la faction observée.
func _focus_hover_text(key: String) -> String:
	var faction_name := SimFacade.faction_short_name(focus_faction)
	var text: String = {"self": "terres de %s", "war": "en guerre contre %s", "ally": "allié de %s",
		"agreement": "en paix avec %s, avec un accord", "neutral": "neutre envers %s",
		"tension": "tensions avec %s", "vassal": "vassal ou suzerain de %s"}.get(key, "")
	return text.replace("%s", faction_name)


# ----- Teinte renforcée et légende (DP1, audit A3 C11/U14) --------------------------------------

func _set_tint_boost(on: bool) -> void:
	var terrain: Node = map.get("terrain") if map != null else null
	var material: ShaderMaterial = terrain.get("material") if terrain != null else null
	if material == null:
		return
	var params := {"faction_alpha_near": MODE_TINT_NEAR, "faction_alpha_far": MODE_TINT_FAR, "faction_saturation": MODE_SATURATION}
	if on:
		if _tint_saved.is_empty():
			for key in params:
				_tint_saved[key] = material.get_shader_parameter(key)
		for key in params:
			material.set_shader_parameter(key, params[key])
	elif not _tint_saved.is_empty():
		for key in _tint_saved:
			material.set_shader_parameter(key, _tint_saved[key])
		_tint_saved.clear()


## Entrées de légende du mode : [[couleur ou relation, libellé]] (catégories) ou
## {"from": Color, "to": Color, "low": texte, "high": texte} (rampe).
func _legend_entries() -> Variant:
	match mode:
		"diplomacy":
			if map != null and DiplomaticStances.available(map.sim):  # DP2
				var stance_entries: Array = []
				for key in DiplomaticStances.ORDER:
					var label: String = DiplomaticStances.LABELS[key]
					if key == "self" and focus_faction != "":  # DZ
						label = SimFacade.faction_short_name(focus_faction)
					stance_entries.append([key, label])
				return stance_entries
			return [["self", "Nous"], ["war", "Guerre"], ["truce", "Trêve"], ["alliance", "Alliés"], ["vassal", "Vassaux"], ["peace", "Neutres"]]
		"religion":
			return [[RELIGION_AVIGNON, "Obédience d'Avignon"], [RELIGION_ROME, "Obédience de Rome"], [HERESY, "Hérésie"], [RELIGION_OTHER, "Autre foi"]]
		"unrest":
			return {"from": GOOD, "to": BAD, "low": "Calme", "high": "Révolte proche"}
		"wealth":
			return {"from": WEALTH_LOW, "to": WEALTH_HIGH, "low": "Les plus pauvres", "high": "Les plus riches"}
		"population":
			return {"from": POPULATION_LOW, "to": POPULATION_HIGH, "low": "Les moins peuplées", "high": "Les plus peuplées"}
		"loyalty":
			return [[LOYALTY_HIGH, "Vassal fidèle"], [LOYALTY_MID, "Hésitant"], [LOYALTY_LOW, "Prêt à se révolter"], [NEUTRAL, "Pas un vassal"]]
		"supply":
			return [[SUPPLY_GAIN, "Ravitaillée (terres amies)"], [SUPPLY_LOSS, "Attrition"], [SUPPLY_STARVE, "Attrition d'hiver"]]
		"claims":
			return [[CLAIM_COLORS["ours"], "Nos revendications"], [CLAIM_COLORS["against_us"], "Revendiquées contre nous"], [CLAIM_COLORS["contested"], "Disputées"], [NEUTRAL, "Aucune"]]
		"feudal":
			return FeudalMapLens.legend_entries()
	return []


func _mode_title() -> String:
	for entry in MODES:
		if entry[0] == mode:
			var key := ShortcutSheet.first_key(str(entry[2])) if str(entry[2]) != "" else ""
			var seen := " — relations de %s" % SimFacade.faction_short_name(focus_faction) if mode == "diplomacy" and focus_faction != "" else ""  # DZ
			return "Carte : %s%s%s" % [str(entry[1]).to_lower(), seen, " (%s)" % key if key != "" else ""]
	return ""


func _show_legend() -> void:
	if _legend != null and is_instance_valid(_legend):
		# Détachée tout de suite : la nouvelle légende garde le nom « MapModeLegend ».
		_legend.get_parent().remove_child(_legend)
		_legend.queue_free()
	_legend = null
	if mode == POLITICAL or map == null or map.ui == null:
		return
	_legend = PanelContainer.new()
	_legend.name = "MapModeLegend"
	_legend.theme = load("res://scenes/ui/parchment_theme.tres")
	_legend.add_theme_stylebox_override("panel", HudStyle.panel_box(8))
	_legend.mouse_filter = Control.MOUSE_FILTER_IGNORE
	var box := VBoxContainer.new()
	box.add_theme_constant_override("separation", 2)
	_legend.add_child(box)
	box.add_child(HudStyle.label(_mode_title(), HudStyle.FONT_TITLE, HudStyle.RUBRIC))
	var row := HBoxContainer.new()
	row.add_theme_constant_override("separation", 12)
	var entries: Variant = _legend_entries()
	if entries is Dictionary:
		row.add_child(HudStyle.label(entries["low"], HudStyle.FONT_BODY, HudStyle.INK))
		var ramp := TextureRect.new()
		var gradient := Gradient.new()
		gradient.set_color(0, entries["from"])
		gradient.set_color(1, entries["to"])
		var texture := GradientTexture2D.new()
		texture.gradient = gradient
		texture.width = 160
		texture.height = 14
		ramp.texture = texture
		ramp.size_flags_vertical = Control.SIZE_SHRINK_CENTER
		row.add_child(ramp)
		row.add_child(HudStyle.label(entries["high"], HudStyle.FONT_BODY, HudStyle.INK))
	else:
		for entry in entries:
			var chip := HBoxContainer.new()
			chip.add_theme_constant_override("separation", 4)
			var swatch := ColorRect.new()
			swatch.custom_minimum_size = Vector2(16, 16)
			swatch.size_flags_vertical = Control.SIZE_SHRINK_CENTER
			swatch.color = DiplomaticStances.COLORS.get(entry[0], RELATION_COLORS.get(entry[0], Color.GRAY)) if entry[0] is String else entry[0]
			chip.add_child(swatch)
			var symbol_key: String = str(DiplomaticStances.SYMBOL_KEYS.get(entry[0], entry[0])) if entry[0] is String else ""
			var symbol := Accessibility.relation_symbol(symbol_key) if mode == "diplomacy" and entry[0] is String and Accessibility.colorblind() else ""
			chip.add_child(HudStyle.label(("%s %s" % [symbol, entry[1]]).strip_edges(), HudStyle.FONT_BODY, HudStyle.INK))
			row.add_child(chip)
	box.add_child(row)
	box.add_child(HudStyle.label("Survolez une province pour sa valeur. Les terres voilées sont hors de vue de vos armées, places et agents.", HudStyle.FONT_SMALL, HudStyle.INK_FADED))
	if mode == "diplomacy" and DiplomaticStances.available(map.sim):  # DZ
		box.add_child(HudStyle.label("Cliquez une province pour voir les relations de son seigneur ; vos terres pour revenir aux vôtres.", HudStyle.FONT_SMALL, HudStyle.INK_FADED))
	map.ui.add_child(_legend)
	_legend.set_anchors_and_offsets_preset(Control.PRESET_CENTER_BOTTOM, Control.PRESET_MODE_MINSIZE, 18)
	_legend.grow_horizontal = Control.GROW_DIRECTION_BOTH
	_legend.grow_vertical = Control.GROW_DIRECTION_BEGIN


## Tests et captures : légende affichée du mode de carte courant (null sinon).
func legend() -> PanelContainer:
	return _legend if _legend != null and is_instance_valid(_legend) else null
