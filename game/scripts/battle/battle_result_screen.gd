class_name BattleResultScreen
extends Control

## Écran de fin de bataille (lot B2 / T2, refait par UB1) : grande bannière
## illustrée « Victoire », « Défaite » ou « Victoire à la Pyrrhus » avec le verdict nuancé, bilan
## chiffré, puis pour chaque camp les régiments en cartes (`RosterCard` : pertes, ennemis abattus,
## héros) et un tableau par régiment (engagés, pertes, tués, sort) ; ensuite le héros de la
## bataille, les captifs et rançons, l'expérience gagnée, le butin et les faits notables ; retour
## à la campagne.
## Aucune règle : tout vient de `BattleSim.get_outcome()`, de `get_units()` (dont `kills`) et, pour
## les suites de campagne, de `BattleAftermath` (lecture du cœur avant / après `resolve_battle`).
## Le verdict n'est qu'un libellé d'après les proportions de pertes (seuils d'affichage).

signal return_pressed

const INK := BattleUiKit.INK
const MUTED := Color(0.42, 0.33, 0.22)
const RED := Color(0.55, 0.1, 0.08)
const GREEN := Color(0.16, 0.4, 0.16)
const GOLD := Color(0.85, 0.65, 0.12)
const PANEL_MAX := Vector2(1180, 860)
const ROW_FONT := 13
## Proportions de pertes (pertes / engagés) qui qualifient le verdict.
const DECISIVE_ENEMY := 0.5
const DECISIVE_OWN := 0.25
## Victoire à la Pyrrhus : vainqueur ayant perdu au moins cette part, et plus que le vaincu.
const PYRRHIC_OWN := 0.3
const BANNERS := {
	"victory": "res://assets/events/evt_crecy.jpg",
	"pyrrhic": "res://assets/events/evt_poitiers.jpg",
	"defeat": "res://assets/events/evt_azincourt.jpg",
}
const CARD_SIZE := Vector2(54, 80)

var title_label: Label
var subtitle_label: Label
var return_button: Button
var mentions_box: VBoxContainer
var aftermath_box: HBoxContainer
var hero_label: Label
var panel: PanelContainer
var banner: Control
var _banner_texture: Texture2D
var _banner_color := GREEN
var _tables: Dictionary = {}  # side -> GridContainer
var _row_count: int = 0
var _sides: Dictionary = {}


## Libellé du verdict pour le camp du joueur d'après les proportions de pertes des deux camps.
static func verdict(won: bool, own_ratio: float, enemy_ratio: float) -> String:
	if won:
		if enemy_ratio >= DECISIVE_ENEMY and own_ratio <= DECISIVE_OWN:
			return "Victoire décisive"
		if own_ratio > enemy_ratio:
			return "Victoire chèrement acquise"
		return "Victoire"
	if own_ratio >= DECISIVE_ENEMY and enemy_ratio <= DECISIVE_OWN:
		return "Défaite écrasante"
	if enemy_ratio > own_ratio:
		return "Défaite honorable"
	return "Défaite"


## Grand titre de la bannière : « Victoire », « Victoire à la Pyrrhus » ou « Défaite ».
static func banner_title(won: bool, own_ratio: float, enemy_ratio: float) -> String:
	if not won:
		return "Défaite"
	if own_ratio >= PYRRHIC_OWN and own_ratio > enemy_ratio:
		return "Victoire à la Pyrrhus"
	return "Victoire"


func _ready() -> void:
	name = "BattleResult"
	set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	mouse_filter = Control.MOUSE_FILTER_STOP
	var veil := ColorRect.new()
	veil.color = Color(0.08, 0.05, 0.02, 0.6)
	veil.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	veil.mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_child(veil)
	get_viewport().size_changed.connect(_layout)


func _label(text: String, size: int = 14, color: Color = INK, bold: bool = false) -> Label:
	var label := BattleUiKit.label(text, size, color, false, bold)
	label.mouse_filter = Control.MOUSE_FILTER_IGNORE
	return label


func _layout() -> void:
	if panel == null:
		return
	var view := get_viewport_rect().size
	var size := Vector2(minf(PANEL_MAX.x, view.x - 24.0), minf(PANEL_MAX.y, view.y - 24.0))
	panel.position = (view - size) * 0.5
	panel.size = size


## Construit l'écran. `sides` : {side: {name, faction, color}} ; `player_side` ; `units` : dernier
## `get_units()` ; `outcome` : `get_outcome()` ; `battle_title` : « Bataille de … » ;
## `aftermath` : `BattleAftermath.diff` (vide hors campagne).
func show_result(battle_title: String, player_side: String, sides: Dictionary, units: Array, outcome: Dictionary, aftermath: Dictionary = {}) -> void:
	_sides = sides
	var enemy_side := "defender" if player_side == "attacker" else "attacker"
	var winner := str(outcome.get("winner", "defender"))
	var totals := {}
	for side in [player_side, enemy_side]:
		var engaged := 0
		for unit in units:
			if str(unit["side"]) == side and not bool(unit.get("synthetic", false)):
				engaged += int(unit["initial_soldiers"])
		var losses := int((outcome.get(side, {}) as Dictionary).get("total_losses", 0))
		totals[side] = {"engaged": engaged, "losses": losses, "ratio": float(losses) / maxf(float(engaged), 1.0)}
	var won := winner == player_side
	var own_ratio := float(totals[player_side]["ratio"])
	var enemy_ratio := float(totals[enemy_side]["ratio"])
	var big_title := banner_title(won, own_ratio, enemy_ratio)
	var banner_key := "defeat" if not won else ("pyrrhic" if big_title != "Victoire" else "victory")
	_banner_texture = PortraitLoader.load_texture(BANNERS[banner_key])
	_banner_color = {"victory": Color(0.98, 0.86, 0.45), "pyrrhic": Color(0.95, 0.78, 0.55), "defeat": Color(0.95, 0.55, 0.45)}[banner_key]
	panel = PanelContainer.new()
	panel.name = "Scroll"
	panel.add_theme_stylebox_override("panel", BattleUiKit.parchment_box(0))
	add_child(panel)
	var box := VBoxContainer.new()
	box.add_theme_constant_override("separation", 6)
	panel.add_child(box)
	box.add_child(_build_banner(big_title, battle_title, outcome, sides, winner, player_side, enemy_side, won, own_ratio, enemy_ratio))
	var scroll := ScrollContainer.new()
	scroll.size_flags_vertical = Control.SIZE_EXPAND_FILL
	scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	box.add_child(scroll)
	var margin := MarginContainer.new()
	margin.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	for side_name in ["left", "right"]:
		margin.add_theme_constant_override("margin_" + side_name, 20)
	scroll.add_child(margin)
	var content := VBoxContainer.new()
	content.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	content.add_theme_constant_override("separation", 8)
	margin.add_child(content)
	content.add_child(_balance_row(sides, player_side, enemy_side, totals))
	content.add_child(BattleUiKit.rule())
	# Faits notables, juste sous le bilan (Q2 : en 720p ils tombaient sous le bouton de retour).
	content.add_child(_label("Faits notables", 17, INK, true))
	mentions_box = VBoxContainer.new()
	mentions_box.name = "Mentions"
	mentions_box.add_theme_constant_override("separation", 1)
	content.add_child(mentions_box)
	for line in mentions(player_side, sides, units, outcome):
		var label := _label("• " + line, 14)
		label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		label.custom_minimum_size = Vector2(1000, 0)
		mentions_box.add_child(label)
	content.add_child(BattleUiKit.rule())
	# Un camp par colonne : cartes des régiments puis tableau.
	var hero := _hero(units, player_side)
	var columns := HBoxContainer.new()
	columns.add_theme_constant_override("separation", 18)
	content.add_child(columns)
	for side in [player_side, enemy_side]:
		columns.add_child(_side_column(side, sides[side], units, totals[side], hero if side == player_side else -1))
		if side == player_side:
			columns.add_child(VSeparator.new())
	content.add_child(BattleUiKit.rule())
	# Suites : héros, captifs et rançons, expérience, butin.
	aftermath_box = HBoxContainer.new()
	aftermath_box.name = "Aftermath"
	aftermath_box.add_theme_constant_override("separation", 10)
	content.add_child(aftermath_box)
	_fill_aftermath(units, player_side, enemy_side, hero, outcome, aftermath)
	var footer := HBoxContainer.new()
	footer.alignment = BoxContainer.ALIGNMENT_CENTER
	var footer_margin := MarginContainer.new()
	footer_margin.add_theme_constant_override("margin_bottom", 14)
	footer_margin.add_child(footer)
	box.add_child(footer_margin)
	return_button = Button.new()
	return_button.name = "Return"
	return_button.text = "Retour à la campagne"
	return_button.custom_minimum_size = Vector2(300, 44)
	BattleUiKit.button_font(return_button, 19)
	return_button.pressed.connect(func() -> void: return_pressed.emit())
	footer.add_child(return_button)
	_layout()
	visible = true


func _build_banner(big_title: String, battle_title: String, outcome: Dictionary, sides: Dictionary, winner: String, player_side: String, enemy_side: String, won: bool, own_ratio: float, enemy_ratio: float) -> Control:
	banner = Control.new()
	banner.name = "Banner"
	banner.custom_minimum_size = Vector2(0, 168)
	banner.clip_contents = true
	banner.draw.connect(_draw_banner)
	var row := HBoxContainer.new()
	row.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	row.add_theme_constant_override("separation", 12)
	banner.add_child(row)
	row.add_child(_shield(sides[player_side], winner == player_side))
	var center := VBoxContainer.new()
	center.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	center.alignment = BoxContainer.ALIGNMENT_CENTER
	center.add_theme_constant_override("separation", -4)
	row.add_child(center)
	title_label = BattleUiKit.label(big_title, 60, _banner_color, true)
	title_label.name = "Verdict"
	title_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	title_label.add_theme_color_override("font_outline_color", Color(0.1, 0.04, 0.02))
	title_label.add_theme_constant_override("outline_size", 10)
	center.add_child(title_label)
	var duration := int(float(outcome.get("duration", 0.0)))
	var nuance := verdict(won, own_ratio, enemy_ratio)
	nuance = "" if nuance == big_title else nuance + " · "
	subtitle_label = BattleUiKit.label("%s%s · %d min %02d s · vainqueur : %s" % [nuance, battle_title, duration / 60, duration % 60, str(sides[winner]["name"])], 17, Color(0.97, 0.9, 0.74))
	subtitle_label.add_theme_font_override("font", BattleUiKit.title_italic_font())
	subtitle_label.add_theme_color_override("font_outline_color", Color(0.1, 0.04, 0.02))
	subtitle_label.add_theme_constant_override("outline_size", 5)
	subtitle_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	center.add_child(subtitle_label)
	row.add_child(_shield(sides[enemy_side], winner == enemy_side))
	return banner


func _draw_banner() -> void:
	var rect := Rect2(Vector2.ZERO, banner.size)
	if _banner_texture != null:
		BattleUiKit.draw_cover(banner, _banner_texture, rect, Color(0.85, 0.82, 0.78), 0.4)
	else:
		banner.draw_rect(rect, Color(0.3, 0.2, 0.1))
	banner.draw_rect(rect, Color(0.06, 0.03, 0.01, 0.45))
	banner.draw_line(Vector2(0, rect.size.y - 2), Vector2(rect.size.x, rect.size.y - 2), BattleUiKit.GOLD, 3.0)


## Écu de la faction (blason peint ; à défaut, carré de sa couleur), nom dessous, liseré d'or au
## vainqueur.
func _shield(side_info: Dictionary, is_winner: bool) -> Control:
	var margin := MarginContainer.new()
	margin.add_theme_constant_override("margin_left", 22)
	margin.add_theme_constant_override("margin_right", 22)
	var box := VBoxContainer.new()
	box.custom_minimum_size = Vector2(110, 0)
	box.alignment = BoxContainer.ALIGNMENT_CENTER
	margin.add_child(box)
	var frame := PanelContainer.new()
	var style := StyleBoxFlat.new()
	style.bg_color = side_info.get("color", Color.GRAY)
	style.border_color = GOLD if is_winner else INK
	style.set_border_width_all(4 if is_winner else 1)
	style.set_corner_radius_all(4)
	style.set_content_margin_all(4)
	frame.add_theme_stylebox_override("panel", style)
	frame.size_flags_horizontal = Control.SIZE_SHRINK_CENTER
	box.add_child(frame)
	var rect := TextureRect.new()
	rect.custom_minimum_size = Vector2(76, 88)
	rect.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
	rect.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_CENTERED
	rect.texture = PortraitLoader.heraldry_texture(str(side_info.get("faction", "")))
	frame.add_child(rect)
	var label := BattleUiKit.label(str(side_info.get("name", "")), 17, Color(0.98, 0.93, 0.8), false, true)
	label.add_theme_color_override("font_outline_color", Color(0.1, 0.04, 0.02))
	label.add_theme_constant_override("outline_size", 4)
	label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	box.add_child(label)
	return margin


## Bilan chiffré : engagés et pertes des deux camps, barre des pertes.
func _balance_row(sides: Dictionary, player_side: String, enemy_side: String, totals: Dictionary) -> Control:
	var grid := GridContainer.new()
	grid.columns = 4
	grid.size_flags_horizontal = Control.SIZE_SHRINK_CENTER
	grid.add_theme_constant_override("h_separation", 28)
	for text in ["", "Engagés", "Pertes", "Survivants"]:
		grid.add_child(_label(text, 13, MUTED))
	for side in [player_side, enemy_side]:
		grid.add_child(_label(str(sides[side]["name"]), 16, INK, true))
		grid.add_child(_label(BattleUiKit.thousands(int(totals[side]["engaged"])), 16))
		grid.add_child(_label("%s (%d %%)" % [BattleUiKit.thousands(int(totals[side]["losses"])), int(round(float(totals[side]["ratio"]) * 100.0))], 16, RED))
		grid.add_child(_label(BattleUiKit.thousands(int(totals[side]["engaged"]) - int(totals[side]["losses"])), 16, GREEN))
	return grid


## Colonne d'un camp : en-tête, cartes des régiments (pertes, tués, héros) puis tableau.
func _side_column(side: String, side_info: Dictionary, units: Array, totals: Dictionary, hero_id: int) -> Control:
	var column := VBoxContainer.new()
	column.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	column.add_theme_constant_override("separation", 4)
	var head := HBoxContainer.new()
	var swatch := ColorRect.new()
	swatch.color = side_info.get("color", Color.GRAY)
	swatch.custom_minimum_size = Vector2(12, 12)
	swatch.size_flags_vertical = Control.SIZE_SHRINK_CENTER
	head.add_child(swatch)
	head.add_child(_label("Régiments — %s" % str(side_info.get("name", "")), 17, INK, true))
	column.add_child(head)
	var names := BattleHud.distinct_names(units, side)
	var cards := HFlowContainer.new()
	cards.add_theme_constant_override("h_separation", 3)
	cards.add_theme_constant_override("v_separation", 3)
	column.add_child(cards)
	var grid := GridContainer.new()
	grid.name = "Losses_%s" % side
	grid.columns = 6
	grid.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	grid.add_theme_constant_override("h_separation", 8)
	grid.add_theme_constant_override("v_separation", 1)
	column.add_child(grid)
	_tables[side] = grid
	for text in ["", "Régiment", "Engagés", "Pertes", "Tués", "Sort"]:
		grid.add_child(_label(text, 12, MUTED))
	var icons := get_node_or_null("/root/IconLibrary")
	var color: Color = side_info.get("color", Color.GRAY)
	for unit in units:
		if str(unit["side"]) != side or bool(unit.get("synthetic", false)):
			continue
		var id := int(unit["id"])
		var initial := int(unit["initial_soldiers"])
		var lost := initial - int(unit["soldiers"])
		var kills := int(unit.get("kills", 0))
		var fate := unit_fate(unit)
		var name_text := str(names.get(id, unit["name"]))
		var card_data := {"unit_type": str(unit.get("type", "")), "name": name_text, "soldiers": int(unit["soldiers"]),
			"max_soldiers": int(unit.get("max_soldiers", initial)), "engaged": initial}
		var card := RosterCard.new().setup(card_data, color, str(side_info.get("faction", "")), bool(unit["is_general"]))
		card.custom_minimum_size = CARD_SIZE
		card.set_losses(lost, kills, fate, id == hero_id)
		cards.add_child(card)
		if icons != null:
			grid.add_child(icons.call("make_rect", str(unit.get("type", "")), 16.0, "unit"))
		else:
			grid.add_child(Control.new())
		var name_label := _label(name_text + (" ★" if bool(unit["is_general"]) else ""), ROW_FONT, INK, id == hero_id)
		name_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		name_label.clip_text = true
		name_label.text_overrun_behavior = TextServer.OVERRUN_TRIM_ELLIPSIS
		name_label.custom_minimum_size = Vector2(140, 0)
		grid.add_child(name_label)
		grid.add_child(_label(str(initial), ROW_FONT))
		grid.add_child(_label(str(lost), ROW_FONT, RED if lost > initial / 2 else INK))
		grid.add_child(_label(str(kills), ROW_FONT, GREEN if kills > 0 else MUTED))
		grid.add_child(_label(fate, ROW_FONT, RED if fate == "anéanti" or fate == "en déroute" else MUTED))
		_row_count += 1
	return column


## Régiment du joueur qui a abattu le plus d'ennemis (-1 si aucun).
static func _hero(units: Array, side: String) -> int:
	var best := -1
	var best_kills := 0
	for unit in units:
		if str(unit["side"]) == side and not bool(unit.get("synthetic", false)) and int(unit.get("kills", 0)) > best_kills:
			best_kills = int(unit.get("kills", 0))
			best = int(unit["id"])
	return best


func _aftermath_card(title: String, lines: Array, accent: Color = INK) -> PanelContainer:
	var card := PanelContainer.new()
	card.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	card.add_theme_stylebox_override("panel", BattleUiKit.parchment_box(10, BattleUiKit.PARCHMENT_LIGHT, BattleUiKit.PARCHMENT_DARK, 1))
	var box := VBoxContainer.new()
	box.add_theme_constant_override("separation", 2)
	card.add_child(box)
	box.add_child(BattleUiKit.label(title, 18, accent, true))
	for line in lines:
		var label := _label(str(line), 14)
		label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		label.custom_minimum_size = Vector2(220, 0)
		box.add_child(label)
	return card


func _fill_aftermath(units: Array, player_side: String, enemy_side: String, hero_id: int, outcome: Dictionary, aftermath: Dictionary) -> void:
	# Héros de la bataille : le régiment du joueur le plus meurtrier, et le plus redoutable adversaire.
	var hero_lines: Array = []
	var names := BattleHud.distinct_names(units, player_side)
	for unit in units:
		if int(unit["id"]) == hero_id:
			hero_lines.append("%s : %d ennemis abattus, %d survivants." % [str(names.get(hero_id, unit["name"])), int(unit.get("kills", 0)), int(unit["soldiers"])])
	if hero_lines.is_empty():
		hero_lines.append("Aucun régiment ne s'est distingué.")
	var foe := _hero(units, enemy_side)
	if foe >= 0:
		var foe_names := BattleHud.distinct_names(units, enemy_side)
		for unit in units:
			if int(unit["id"]) == foe:
				hero_lines.append("Le plus redoutable adversaire : %s (%d des nôtres)." % [str(foe_names.get(foe, unit["name"])), int(unit.get("kills", 0))])
	var hero_card := _aftermath_card("Héros de la bataille", hero_lines, BattleUiKit.GOLD.darkened(0.3))
	hero_label = hero_card.get_child(0).get_child(1) as Label
	aftermath_box.add_child(hero_card)
	# Captifs et rançons.
	var captive_lines: Array = []
	var in_campaign := not aftermath.is_empty()
	for captive in aftermath.get("captives", []):
		captive_lines.append("%s%s — rançon %s ₶" % [str(captive["name"]), " (%s)" % captive["rank"] if str(captive.get("rank", "")) != "" else "", BattleUiKit.thousands(int(captive["ransom"]))])
	for captive in aftermath.get("lost", []):
		captive_lines.append("Des nôtres pris : %s — rançon exigée %s ₶" % [str(captive["name"]), BattleUiKit.thousands(int(captive["ransom"]))])
	if captive_lines.is_empty():
		var enemy_result: Dictionary = outcome.get(enemy_side, {})
		if bool((outcome.get(outcome.get("winner", ""), {}) as Dictionary).get("no_quarter", false)):
			captive_lines.append("Pas de quartier : aucun prisonnier.")
		elif bool(enemy_result.get("general_captured", false)):
			captive_lines.append("Le chef ennemi est pris : rançon à fixer à la cour.")
		else:
			captive_lines.append("Aucun captif de rang." if in_campaign else "Suites connues au retour en campagne.")
	aftermath_box.add_child(_aftermath_card("Captifs et rançons", captive_lines, RED))
	# Expérience.
	var xp_lines: Array = []
	if in_campaign:
		var general_name := str(aftermath.get("general_name", ""))
		if general_name != "":
			var fate := str(aftermath.get("general_fate", ""))
			if fate != "":
				xp_lines.append("%s : %s." % [general_name, fate])
			else:
				var line := "%s : +%d d'expérience" % [general_name, int(aftermath.get("general_xp", 0))]
				if int(aftermath.get("skill_points", 0)) > 0:
					line += ", +%d point de compétence" % int(aftermath["skill_points"])
				xp_lines.append(line + ".")
		var veterans := int(aftermath.get("veterans", 0))
		xp_lines.append("Aucun régiment aguerri." if veterans == 0 else ("%d régiment%s aguerri%s (chevron gagné)." % [veterans, "s" if veterans > 1 else "", "s" if veterans > 1 else ""]))
	else:
		xp_lines.append("Suites connues au retour en campagne.")
	aftermath_box.add_child(_aftermath_card("Expérience", xp_lines, GREEN))
	# Butin : les rançons attendues (une bataille rangée ne rapporte pas d'autre butin).
	var loot_lines: Array = []
	var total := int(aftermath.get("ransom_total", 0))
	if total > 0:
		loot_lines.append("%s ₶ de rançons à percevoir." % BattleUiKit.thousands(total))
	else:
		loot_lines.append("Aucun butin : pas de rançon à percevoir.")
	loot_lines.append("Le pillage vient des chevauchées et des villes prises.")
	aftermath_box.add_child(_aftermath_card("Butin", loot_lines, BattleUiKit.GOLD.darkened(0.35)))


## Q2 : libellés du sort d'un régiment calculé par le cœur (`Unit::fate`, clé `fate`).
const FATE_LABELS := {
	"destroyed": "anéanti",
	"routed": "en déroute",
	"withdrawn": "s'est retiré",
	"reserve": "en réserve",
	"held": "tient le champ",
}


## Sort d'un régiment en fin de bataille (libellé court) : celui du cœur (`fate`), sinon
## déduit de l'état (anciens dictionnaires, doubles de test).
static func unit_fate(unit: Dictionary) -> String:
	if FATE_LABELS.has(str(unit.get("fate", ""))):
		return str(FATE_LABELS[str(unit["fate"])])
	if int(unit["soldiers"]) <= 0:
		return "anéanti"
	if bool(unit.get("left_field", false)):
		return "hors du champ" if str(unit["state"]) != "routing" else "en déroute"
	if str(unit["state"]) == "routing":
		return "en déroute"
	if bool(unit.get("reserve", false)):
		return "en réserve"
	return "tient le champ"


## Mentions notables : régiments anéantis, généraux tombés ou capturés, pas de quartier, rançons,
## déroute générale.
static func mentions(player_side: String, sides: Dictionary, units: Array, outcome: Dictionary) -> Array[String]:
	var lines: Array[String] = []
	var enemy_side := "defender" if player_side == "attacker" else "attacker"
	for side in [player_side, enemy_side]:
		var result: Dictionary = outcome.get(side, {})
		var side_name := str(sides[side]["name"])
		var names := BattleHud.distinct_names(units, side)
		var destroyed: Array[String] = []
		for unit in units:
			if str(unit["side"]) == side and not bool(unit.get("synthetic", false)) and int(unit["soldiers"]) <= 0:
				destroyed.append(str(names.get(int(unit["id"]), unit["name"])))
		if not destroyed.is_empty():
			lines.append("%s : %s anéanti%s (%s)." % [side_name, "un régiment" if destroyed.size() == 1 else "%d régiments" % destroyed.size(), "" if destroyed.size() == 1 else "s", ", ".join(destroyed)])
		if bool(result.get("general_killed", false)):
			lines.append("Le chef de l'ost %s est tombé sur le champ." % BattleScene.de(side_name))
		elif bool(result.get("general_captured", false)):
			var other_name := str(sides[enemy_side if side == player_side else player_side]["name"])
			lines.append("Le chef de l'ost %s est aux mains de l'ost %s : rançon à attendre." % [BattleScene.de(side_name), BattleScene.de(other_name)])
		if bool(result.get("no_quarter", false)):
			lines.append("%s a déployé l'étendard du « pas de quartier » : aucun prisonnier, aucune rançon." % side_name)
		if bool(result.get("withdrew", false)):  # Q2 : retraite en bon ordre, pas une déroute
			lines.append("L'ost %s a sonné la retraite et quitté le champ en bon ordre." % BattleScene.de(side_name))
		elif bool(result.get("routed", false)):
			lines.append("L'ost %s a été mis en déroute." % BattleScene.de(side_name))
	if lines.is_empty():
		lines.append("Aucun fait d'armes notable : les deux osts se sont séparés en bon ordre.")
	return lines


func row_count() -> int:
	return _row_count
