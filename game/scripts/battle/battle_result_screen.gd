class_name BattleResultScreen
extends Control

## Écran de fin de bataille mis en scène (lot B2 / T2) : voile sur le champ, grand parchemin avec
## les écus des deux camps, verdict (« Victoire décisive », « Défaite… ») tiré du rapport de
## pertes, bilan par camp, tableau des pertes par régiment, mentions notables (régiments anéantis,
## général tombé ou capturé, « pas de quartier », rançons à attendre) et retour à la campagne.
## Aucune règle : tout vient de `BattleSim.get_outcome()` et de `get_units()` ; le verdict n'est
## qu'un libellé d'après les proportions de pertes (seuils d'affichage).

signal return_pressed

const INK := Color(0.22, 0.14, 0.07)
const MUTED := Color(0.42, 0.33, 0.22)
const RED := Color(0.55, 0.1, 0.08)
const GREEN := Color(0.16, 0.4, 0.16)
const GOLD := Color(0.85, 0.65, 0.12)
const PANEL_SIZE := Vector2(900, 620)
const ROW_FONT := 13
## Proportions de pertes (pertes / engagés) qui qualifient le verdict.
const DECISIVE_ENEMY := 0.5
const DECISIVE_OWN := 0.25

var title_label: Label
var subtitle_label: Label
var return_button: Button
var mentions_box: VBoxContainer
var _tables: Dictionary = {}  # side -> GridContainer
var _row_count: int = 0


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


func _ready() -> void:
	name = "BattleResult"
	set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	mouse_filter = Control.MOUSE_FILTER_STOP
	var veil := ColorRect.new()
	veil.color = Color(0.08, 0.05, 0.02, 0.55)
	veil.set_anchors_preset(Control.PRESET_FULL_RECT)
	veil.mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_child(veil)


func _label(text: String, size: int = 14, color: Color = INK) -> Label:
	var label := Label.new()
	label.text = text
	label.add_theme_font_size_override("font_size", size)
	label.add_theme_color_override("font_color", color)
	label.mouse_filter = Control.MOUSE_FILTER_IGNORE
	return label


## Construit l'écran. `sides` : {side: {name, faction, color}} ; `player_side` ; `units` : dernier
## `get_units()` ; `outcome` : `get_outcome()` ; `battle_title` : « Bataille de … ».
func show_result(battle_title: String, player_side: String, sides: Dictionary, units: Array, outcome: Dictionary) -> void:
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
	var panel := PanelContainer.new()
	panel.name = "Scroll"
	panel.anchor_left = 0.5
	panel.anchor_right = 0.5
	panel.anchor_top = 0.5
	panel.anchor_bottom = 0.5
	panel.offset_left = -PANEL_SIZE.x * 0.5
	panel.offset_right = PANEL_SIZE.x * 0.5
	panel.offset_top = -PANEL_SIZE.y * 0.5
	panel.offset_bottom = PANEL_SIZE.y * 0.5
	add_child(panel)
	var box := VBoxContainer.new()
	box.add_theme_constant_override("separation", 8)
	panel.add_child(box)
	# En-tête : écu du joueur, verdict, écu de l'adversaire.
	var head := HBoxContainer.new()
	head.add_theme_constant_override("separation", 16)
	box.add_child(head)
	head.add_child(_shield(sides[player_side], winner == player_side))
	var center := VBoxContainer.new()
	center.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	center.alignment = BoxContainer.ALIGNMENT_CENTER
	head.add_child(center)
	title_label = _label(verdict(won, float(totals[player_side]["ratio"]), float(totals[enemy_side]["ratio"])), 34, GREEN if won else RED)
	title_label.name = "Verdict"
	title_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	center.add_child(title_label)
	var duration := int(float(outcome.get("duration", 0.0)))
	subtitle_label = _label("%s · %d min %02d s · vainqueur : %s" % [battle_title, duration / 60, duration % 60, str(sides[winner]["name"])], 15, MUTED)
	subtitle_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	center.add_child(subtitle_label)
	center.add_child(_balance_row(sides, player_side, enemy_side, totals))
	head.add_child(_shield(sides[enemy_side], winner == enemy_side))
	box.add_child(HSeparator.new())
	# Tableau des pertes par régiment, un camp par colonne.
	var columns := HBoxContainer.new()
	columns.add_theme_constant_override("separation", 18)
	columns.size_flags_vertical = Control.SIZE_EXPAND_FILL
	box.add_child(columns)
	for side in [player_side, enemy_side]:
		columns.add_child(_side_table(side, sides[side], units, totals[side]))
	box.add_child(HSeparator.new())
	mentions_box = VBoxContainer.new()
	mentions_box.name = "Mentions"
	mentions_box.add_theme_constant_override("separation", 1)
	box.add_child(mentions_box)
	for line in mentions(player_side, sides, units, outcome):
		var label := _label("• " + line, 14)
		label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		mentions_box.add_child(label)
	var footer := HBoxContainer.new()
	footer.alignment = BoxContainer.ALIGNMENT_CENTER
	box.add_child(footer)
	return_button = Button.new()
	return_button.name = "Return"
	return_button.text = "Retour à la campagne"
	return_button.custom_minimum_size = Vector2(260, 36)
	return_button.pressed.connect(func() -> void: return_pressed.emit())
	footer.add_child(return_button)
	visible = true


## Écu de la faction (blason peint ; à défaut, carré de sa couleur), nom dessous, liseré d'or au
## vainqueur.
func _shield(side_info: Dictionary, is_winner: bool) -> Control:
	var box := VBoxContainer.new()
	box.custom_minimum_size = Vector2(120, 0)
	box.alignment = BoxContainer.ALIGNMENT_CENTER
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
	rect.custom_minimum_size = Vector2(84, 96)
	rect.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
	rect.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_CENTERED
	rect.texture = PortraitLoader.heraldry_texture(str(side_info.get("faction", "")))
	frame.add_child(rect)
	var label := _label(str(side_info.get("name", "")), 15)
	label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	box.add_child(label)
	return box


## Bilan chiffré : engagés et pertes des deux camps, barre des pertes.
func _balance_row(sides: Dictionary, player_side: String, enemy_side: String, totals: Dictionary) -> Control:
	var grid := GridContainer.new()
	grid.columns = 3
	grid.size_flags_horizontal = Control.SIZE_SHRINK_CENTER
	grid.add_theme_constant_override("h_separation", 22)
	for text in ["", "Engagés", "Pertes"]:
		grid.add_child(_label(text, 13, MUTED))
	for side in [player_side, enemy_side]:
		grid.add_child(_label(str(sides[side]["name"]), 15))
		grid.add_child(_label(RichTooltip.thousands(int(totals[side]["engaged"])), 15))
		var losses := _label("%s (%d %%)" % [RichTooltip.thousands(int(totals[side]["losses"])), int(round(float(totals[side]["ratio"]) * 100.0))], 15, RED)
		grid.add_child(losses)
	return grid


## Colonne d'un camp : en-tête, puis une ligne par régiment (icône, nom, engagés, pertes, sort).
func _side_table(side: String, side_info: Dictionary, units: Array, totals: Dictionary) -> Control:
	var column := VBoxContainer.new()
	column.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	column.add_theme_constant_override("separation", 4)
	var head := HBoxContainer.new()
	var swatch := ColorRect.new()
	swatch.color = side_info.get("color", Color.GRAY)
	swatch.custom_minimum_size = Vector2(12, 12)
	swatch.size_flags_vertical = Control.SIZE_SHRINK_CENTER
	head.add_child(swatch)
	head.add_child(_label("Pertes — %s" % str(side_info.get("name", "")), 16))
	column.add_child(head)
	var scroll := ScrollContainer.new()
	scroll.size_flags_vertical = Control.SIZE_EXPAND_FILL
	scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	column.add_child(scroll)
	var grid := GridContainer.new()
	grid.name = "Losses_%s" % side
	grid.columns = 5
	grid.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	grid.add_theme_constant_override("h_separation", 8)
	grid.add_theme_constant_override("v_separation", 1)
	scroll.add_child(grid)
	_tables[side] = grid
	for text in ["", "Régiment", "Engagés", "Pertes", "Sort"]:
		grid.add_child(_label(text, 12, MUTED))
	var icons := get_node_or_null("/root/IconLibrary")
	for unit in units:
		if str(unit["side"]) != side or bool(unit.get("synthetic", false)):
			continue
		var initial := int(unit["initial_soldiers"])
		var lost := initial - int(unit["soldiers"])
		if icons != null:
			grid.add_child(icons.call("make_rect", str(unit.get("type", "")), 16.0, "unit"))
		else:
			grid.add_child(Control.new())
		var name_label := _label(str(unit["name"]) + (" ★" if bool(unit["is_general"]) else ""), ROW_FONT)
		name_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		name_label.clip_text = true
		name_label.text_overrun_behavior = TextServer.OVERRUN_TRIM_ELLIPSIS
		name_label.custom_minimum_size = Vector2(150, 0)
		grid.add_child(name_label)
		grid.add_child(_label(str(initial), ROW_FONT))
		grid.add_child(_label(str(lost), ROW_FONT, RED if lost > initial / 2 else INK))
		var fate := unit_fate(unit)
		grid.add_child(_label(fate, ROW_FONT, RED if fate == "anéanti" or fate == "en déroute" else MUTED))
		_row_count += 1
	return column


## Sort d'un régiment en fin de bataille (libellé court).
static func unit_fate(unit: Dictionary) -> String:
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
		var destroyed: Array[String] = []
		for unit in units:
			if str(unit["side"]) == side and not bool(unit.get("synthetic", false)) and int(unit["soldiers"]) <= 0:
				destroyed.append(str(unit["name"]))
		if not destroyed.is_empty():
			lines.append("%s : %s anéanti%s (%s)." % [side_name, "un régiment" if destroyed.size() == 1 else "%d régiments" % destroyed.size(), "" if destroyed.size() == 1 else "s", ", ".join(destroyed)])
		if bool(result.get("general_killed", false)):
			lines.append("Le chef de l'ost %s est tombé sur le champ." % BattleScene.de(side_name))
		elif bool(result.get("general_captured", false)):
			var other_name := str(sides[enemy_side if side == player_side else player_side]["name"])
			lines.append("Le chef de l'ost %s est aux mains de l'ost %s : rançon à attendre." % [BattleScene.de(side_name), BattleScene.de(other_name)])
		if bool(result.get("no_quarter", false)):
			lines.append("%s a déployé l'étendard du « pas de quartier » : aucun prisonnier, aucune rançon." % side_name)
		if bool(result.get("routed", false)):
			lines.append("L'ost %s a été mis en déroute." % BattleScene.de(side_name))
	if lines.is_empty():
		lines.append("Aucun fait d'armes notable : les deux osts se sont séparés en bon ordre.")
	return lines


func row_count() -> int:
	return _row_count
