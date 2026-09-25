class_name PreBattleDialog
extends Control

## Écran d'avant-bataille (lot UB1, reprend M7 § 4 et M8 § 2) : bannière
## illustrée, rapport de forces (barre d'équilibre et chances **estimées par le cœur**,
## `CampaignSim.get_battle_forecast`), sceaux et portraits des généraux, régiments des deux
## camps en cartes (`RosterCard`), renforts alliés, terrain, saison, météo prévue et site, puis
## « Combattre » / « Résolution automatique » / « Retraite » (ou « Maintenir le siège » pour un
## assaut). Le camp du joueur est toujours à gauche.
## La météo et le site sont lus sur un `BattleSim` construit avec la même graine que la bataille
## qui sera livrée (B6). Aucune règle ici.

signal fight_requested(index: int, seed: int)
signal auto_requested(index: int)
signal withdraw_requested(index: int)

const INK := BattleUiKit.INK
const TERRAIN_FR := {
	"plains": "plaines", "hills": "collines", "mountains": "montagnes", "forest": "forêt",
	"marsh": "marais", "heath": "lande", "bocage": "bocage",
}
const SEASON_FR := {"spring": "printemps", "summer": "été", "autumn": "automne", "winter": "hiver"}
const BANNER_FIELD := "res://assets/events/evt_crecy.jpg"
const BANNER_SIEGE := "res://assets/events/evt_sluys.jpg"
const PANEL_MAX := Vector2(1180, 820)
const CARD_COLUMNS := 8
const CARD_SIZE := Vector2(64, 94)

var battle: Dictionary = {}
var setup: Dictionary = {}
var forecast: Dictionary = {}
var weather_label_text: String = ""
var site_label_text: String = ""
var player_side: String = "attacker"
var title_label: Label
var subtitle_label: Label
var body_label: Label  # conditions : terrain, saison, météo prévue, site (lu par le smoke)
var modifiers_label: Label
var verdict_label: Label
var chance_label: Label
var balance_bar: Control
var fight_button: Button
var auto_button: Button
var withdraw_button: Button
var panel: PanelContainer
var banner: Control
var _banner_texture: Texture2D
var _columns: Array[VBoxContainer] = []
var _share := 0.5
var _colors: Array[Color] = [Color(0.2, 0.3, 0.75), Color(0.75, 0.15, 0.12)]


func _ready() -> void:
	set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	mouse_filter = Control.MOUSE_FILTER_STOP
	theme = load("res://scenes/ui/parchment_theme.tres")
	var dim := ColorRect.new()
	dim.color = Color(0.05, 0.03, 0.02, 0.62)
	dim.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	dim.mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_child(dim)
	panel = PanelContainer.new()
	panel.name = "PreBattlePanel"
	panel.add_theme_stylebox_override("panel", BattleUiKit.parchment_box(0))
	add_child(panel)
	var box := VBoxContainer.new()
	box.add_theme_constant_override("separation", 8)
	panel.add_child(box)
	box.add_child(_build_banner())
	var inner := MarginContainer.new()
	for side in ["left", "right"]:
		inner.add_theme_constant_override("margin_" + side, 20)
	inner.add_theme_constant_override("margin_bottom", 16)
	inner.size_flags_vertical = Control.SIZE_EXPAND_FILL
	box.add_child(inner)
	var content := VBoxContainer.new()
	content.add_theme_constant_override("separation", 8)
	inner.add_child(content)
	content.add_child(_build_balance())
	content.add_child(BattleUiKit.rule())
	var sides := HBoxContainer.new()
	sides.add_theme_constant_override("separation", 24)
	sides.size_flags_vertical = Control.SIZE_EXPAND_FILL
	content.add_child(sides)
	for i in 2:
		var column := VBoxContainer.new()
		column.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		column.add_theme_constant_override("separation", 6)
		sides.add_child(column)
		_columns.append(column)
		if i == 0:
			var divider := VSeparator.new()
			sides.add_child(divider)
	content.add_child(BattleUiKit.rule())
	body_label = BattleUiKit.label("", 16)
	body_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	content.add_child(body_label)
	modifiers_label = BattleUiKit.label("", 14, BattleUiKit.INK_SOFT)
	modifiers_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	modifiers_label.text_overrun_behavior = TextServer.OVERRUN_TRIM_ELLIPSIS
	modifiers_label.clip_text = true
	content.add_child(modifiers_label)
	content.add_child(_build_buttons())
	get_viewport().size_changed.connect(_layout)
	_layout()
	visible = false


func _build_banner() -> Control:
	banner = Control.new()
	banner.name = "Banner"
	banner.custom_minimum_size = Vector2(0, 132)
	banner.clip_contents = true
	banner.draw.connect(_draw_banner)
	var margin := MarginContainer.new()
	margin.set_anchors_preset(Control.PRESET_FULL_RECT)
	margin.add_theme_constant_override("margin_left", 28)
	margin.add_theme_constant_override("margin_bottom", 14)
	banner.add_child(margin)
	var texts := VBoxContainer.new()
	texts.alignment = BoxContainer.ALIGNMENT_END
	texts.add_theme_constant_override("separation", 0)
	margin.add_child(texts)
	title_label = BattleUiKit.label("", 38, Color(0.99, 0.95, 0.84), true)
	title_label.add_theme_color_override("font_outline_color", Color(0.1, 0.05, 0.02))
	title_label.add_theme_constant_override("outline_size", 6)
	texts.add_child(title_label)
	subtitle_label = BattleUiKit.label("", 17, Color(0.96, 0.88, 0.68))
	subtitle_label.add_theme_font_override("font", BattleUiKit.title_italic_font())
	subtitle_label.add_theme_color_override("font_outline_color", Color(0.1, 0.05, 0.02))
	subtitle_label.add_theme_constant_override("outline_size", 4)
	texts.add_child(subtitle_label)
	return banner


func _draw_banner() -> void:
	var rect := Rect2(Vector2.ZERO, banner.size)
	if _banner_texture != null:
		BattleUiKit.draw_cover(banner, _banner_texture, rect, Color(1, 1, 1), 0.45)
	else:
		banner.draw_rect(rect, Color(0.35, 0.22, 0.12))
	# Voile sombre vers le bas pour lire le titre, filet doré.
	banner.draw_polygon(PackedVector2Array([Vector2(0, 0), Vector2(rect.size.x, 0), rect.size, Vector2(0, rect.size.y)]),
		PackedColorArray([Color(0, 0, 0, 0.05), Color(0, 0, 0, 0.05), Color(0.08, 0.04, 0.02, 0.8), Color(0.08, 0.04, 0.02, 0.8)]))
	banner.draw_line(Vector2(0, rect.size.y - 2), Vector2(rect.size.x, rect.size.y - 2), BattleUiKit.GOLD, 3.0)


func _build_balance() -> Control:
	var box := VBoxContainer.new()
	box.add_theme_constant_override("separation", 6)
	verdict_label = BattleUiKit.label("", 24, INK, true)
	verdict_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	box.add_child(verdict_label)
	var row := HBoxContainer.new()
	row.alignment = BoxContainer.ALIGNMENT_CENTER
	box.add_child(row)
	balance_bar = Control.new()
	balance_bar.name = "BalanceBar"
	balance_bar.custom_minimum_size = Vector2(620, 18)
	balance_bar.draw.connect(func() -> void: BattleUiKit.draw_balance(balance_bar, _share, _colors[0], _colors[1]))
	row.add_child(balance_bar)
	chance_label = BattleUiKit.label("", 14, BattleUiKit.INK_SOFT)
	chance_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	box.add_child(chance_label)
	return box


func _build_buttons() -> Control:
	var row := HBoxContainer.new()
	row.add_theme_constant_override("separation", 18)
	withdraw_button = _button("Retraite", 18)
	withdraw_button.name = "Withdraw"
	withdraw_button.pressed.connect(func() -> void:
		visible = false
		withdraw_requested.emit(int(battle.get("index", 0))))
	row.add_child(withdraw_button)
	var spacer := Control.new()
	spacer.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	row.add_child(spacer)
	auto_button = _button("Résolution automatique", 18)
	auto_button.name = "AutoResolve"
	auto_button.tooltip_text = "La bataille est tranchée sans être jouée, selon le rapport de forces et la fortune des armes."
	auto_button.pressed.connect(func() -> void:
		visible = false
		auto_requested.emit(int(battle.get("index", 0))))
	row.add_child(auto_button)
	fight_button = _button("Combattre", 22)
	fight_button.name = "Fight"
	fight_button.custom_minimum_size = Vector2(230, 48)
	var style := BattleUiKit.parchment_box(8, Color(0.55, 0.11, 0.08), Color(0.36, 0.06, 0.04), 2)
	style.shadow_size = 4
	fight_button.add_theme_stylebox_override("normal", style)
	var hover := style.duplicate() as StyleBoxFlat
	hover.bg_color = Color(0.68, 0.18, 0.12)
	fight_button.add_theme_stylebox_override("hover", hover)
	fight_button.add_theme_color_override("font_color", Color(0.99, 0.94, 0.82))
	fight_button.add_theme_color_override("font_hover_color", Color(1, 1, 0.92))
	fight_button.pressed.connect(func() -> void:
		visible = false
		fight_requested.emit(int(battle.get("index", 0)), int(battle.get("seed", 1))))
	row.add_child(fight_button)
	return row


func _button(text: String, font_size: int) -> Button:
	var button := Button.new()
	button.text = text
	button.focus_mode = Control.FOCUS_NONE
	button.custom_minimum_size = Vector2(0, 44)
	BattleUiKit.button_font(button, font_size)
	return button


func _layout() -> void:
	if panel == null:
		return
	var view := get_viewport_rect().size
	var width := minf(PANEL_MAX.x, view.x - 32.0)
	panel.custom_minimum_size = Vector2(width, 0)
	var height := minf(panel.get_combined_minimum_size().y, minf(PANEL_MAX.y, view.y - 32.0))
	panel.size = Vector2(width, height)
	panel.position = (view - panel.size) * 0.5


## Remplit l'écran pour `p_battle` (entrée de `get_pending_battles`).
func show_battle(sim: Object, p_battle: Dictionary) -> void:
	battle = p_battle
	var index := int(battle.get("index", 0))
	setup = sim.call("get_battle_setup", index)
	forecast = sim.call("get_battle_forecast", index) if sim.has_method("get_battle_forecast") else {}
	var siege := bool(battle.get("siege", false))
	player_side = str(battle.get("player_side", ""))
	if player_side == "":
		player_side = "attacker"
	var enemy_side := "defender" if player_side == "attacker" else "attacker"
	var place := str(battle.get("settlement_name", "")) if siege else str(battle.get("province_name", ""))
	title_label.text = ("Assaut de %s" if siege else "Bataille en vue : %s") % place
	var season := str(SEASON_FR.get(str(setup.get("season", "")), ""))
	var sub := "%s contre %s" % [str(battle.get("attacker_name", "")), str(battle.get("defender_name", ""))]
	if siege:
		var breach := int(battle.get("breach", 0))
		sub += " · murailles de niveau %d, brèche %d %%%s" % [int(battle.get("fortification", 0)), breach, " (ouverte)" if breach >= 50 else ""]
	elif str(battle.get("province_name", "")) != "":
		sub += " · %s" % str(battle.get("province_name", ""))
	if season != "":
		sub += " · %s" % season
	subtitle_label.text = sub
	# AR1 : enluminure du contexte (siège ou bataille rangée), repli sur l'ancienne miniature.
	_banner_texture = ArtPlates.texture(ArtPlates.random_loading_screen("siege" if siege else "battle"))
	if _banner_texture == null:
		_banner_texture = PortraitLoader.load_texture(BANNER_SIEGE if siege else BANNER_FIELD)
	banner.queue_redraw()
	# Couleurs des camps.
	for i in 2:
		var side := player_side if i == 0 else enemy_side
		var side_setup: Dictionary = setup.get(side, {})
		var fallback := Color(0.2, 0.3, 0.75) if i == 0 else Color(0.75, 0.15, 0.12)
		_colors[i] = BattleUiKit.faction_color(str(side_setup.get("faction", "")), fallback)
	if _colors[0].is_equal_approx(_colors[1]):
		_colors[1] = _colors[1].darkened(0.4)
	_fill_balance(siege)
	for i in 2:
		_fill_column(_columns[i], player_side if i == 0 else enemy_side, i)
	_fill_conditions(siege)
	fight_button.text = "Donner l'assaut" if siege else "Combattre"
	fight_button.disabled = setup.is_empty()
	withdraw_button.text = "Maintenir le siège" if siege else "Retraite"
	var can_withdraw := bool(forecast.get("can_withdraw", false))
	withdraw_button.disabled = not can_withdraw
	if siege:
		withdraw_button.tooltip_text = "Remettre l'assaut : le siège continue et affame la garnison."
	elif can_withdraw:
		withdraw_button.tooltip_text = "Refuser la bataille : l'ost se replie et perd du moral."
	else:
		withdraw_button.tooltip_text = "Impossible : vous êtes attaqué, il faut tenir ou laisser trancher la fortune."
	_layout()
	if not visible:
		UiSounds.play("alert")  # UB1 / U13 : bataille en vue
	visible = true
	_layout.call_deferred()


## Barre d'équilibre et verdict, vus du joueur (gauche).
func _fill_balance(siege: bool) -> void:
	if forecast.is_empty():
		var a := float(battle.get("attacker_strength", 1))
		var d := float(battle.get("defender_strength", 1))
		var share_attacker := a / maxf(a + d, 1.0)
		_share = share_attacker if player_side == "attacker" else 1.0 - share_attacker
		verdict_label.text = "Rapport de forces"
		verdict_label.add_theme_color_override("font_color", INK)
		chance_label.text = "Estimation indisponible : effectifs seuls."
	else:
		var share := float(forecast.get("attacker_share", 0.5))
		var chance := float(forecast.get("attacker_win_chance", 0.5))
		if player_side != "attacker":
			share = 1.0 - share
			chance = 1.0 - chance
		_share = share
		verdict_label.text = BattleUiKit.verdict(chance)
		verdict_label.add_theme_color_override("font_color", BattleUiKit.verdict_color(chance))
		chance_label.text = "Chances de victoire estimées : %d %% · puissance %s contre %s%s" % [
			roundi(chance * 100.0),
			BattleUiKit.thousands(roundi(float(forecast.get("%s_power" % player_side, 0.0)))),
			BattleUiKit.thousands(roundi(float(forecast.get("%s_power" % ("defender" if player_side == "attacker" else "attacker"), 0.0)))),
			" (assaut)" if siege else "",
		]
	balance_bar.tooltip_text = "Estimation du cœur de la simulation (mêmes règles que la résolution automatique, sans le hasard) : effectifs, qualité, moral, ravitaillement, général, terrain."
	balance_bar.queue_redraw()


func _fill_column(column: VBoxContainer, side: String, slot: int) -> void:
	for child in column.get_children():
		column.remove_child(child)
		child.queue_free()
	var side_setup: Dictionary = setup.get(side, {})
	var faction := str(side_setup.get("faction", ""))
	var units: Array = side_setup.get("units", [])
	var siege := bool(battle.get("siege", false))
	# En-tête : blason, nom, rôle.
	var header := HBoxContainer.new()
	header.add_theme_constant_override("separation", 10)
	header.alignment = BoxContainer.ALIGNMENT_BEGIN if slot == 0 else BoxContainer.ALIGNMENT_END
	column.add_child(header)
	var arms := TextureRect.new()
	arms.texture = PortraitLoader.heraldry_texture(faction)
	arms.custom_minimum_size = Vector2(40, 46)
	arms.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
	arms.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_CENTERED
	var name_box := VBoxContainer.new()
	name_box.add_theme_constant_override("separation", -2)
	var name_text := str(battle.get("%s_name" % side, side_setup.get("faction_name", "")))
	var name_label := BattleUiKit.label(name_text + ("  (vous)" if slot == 0 and str(battle.get("player_side", "")) != "" else ""), 24, INK, true)
	name_box.add_child(name_label)
	var role := "assaillant" if side == "attacker" else "défenseur"
	if siege:
		role = "assiégeants" if side == "attacker" else "garnison"
	var soldiers := 0
	for unit in units:
		soldiers += int(unit.get("soldiers", 0))
	var strength_text := "%s · %s hommes en %d régiments" % [role.capitalize(), BattleUiKit.thousands(soldiers), units.size()]
	name_box.add_child(BattleUiKit.label(strength_text, 15, BattleUiKit.INK_SOFT))
	if slot == 0:
		header.add_child(arms)
		header.add_child(name_box)
	else:
		name_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_RIGHT
		(name_box.get_child(1) as Label).horizontal_alignment = HORIZONTAL_ALIGNMENT_RIGHT
		header.add_child(name_box)
		header.add_child(arms)
	# Général.
	column.add_child(_general_row(side_setup.get("general", null), faction, slot))
	# Renforts.
	var reinforcements: Array = forecast.get("%s_reinforcements" % side, [])
	if not reinforcements.is_empty():
		var parts := PackedStringArray()
		for entry in reinforcements:
			var text := "%s (%s h., %d rég.)" % [str(entry.get("faction_name", "")), BattleUiKit.thousands(int(entry.get("soldiers", 0))), int(entry.get("regiments", 0))]
			if str(entry.get("general", "")) != "":
				text += " sous %s" % str(entry.get("general", ""))
			parts.append(text)
		var reinf := BattleUiKit.label("Renforts : " + ", ".join(parts), 14, BattleUiKit.GOOD, false, true)
		reinf.clip_text = true
		reinf.text_overrun_behavior = TextServer.OVERRUN_TRIM_ELLIPSIS
		reinf.tooltip_text = reinf.text
		reinf.mouse_filter = Control.MOUSE_FILTER_PASS
		column.add_child(reinf)
	else:
		column.add_child(BattleUiKit.label("Aucun renfort à portée.", 14, BattleUiKit.INK_FADED))
	# Régiments en cartes.
	var grid := GridContainer.new()
	grid.columns = CARD_COLUMNS
	grid.add_theme_constant_override("h_separation", 4)
	grid.add_theme_constant_override("v_separation", 4)
	var general_index := -1
	var general: Variant = side_setup.get("general", null)
	if general is Dictionary:
		general_index = int(general.get("unit_index", -1))
	for i in units.size():
		var card := RosterCard.new().setup(units[i], _colors[slot], faction, i == general_index)
		card.custom_minimum_size = CARD_SIZE
		grid.add_child(card)
	var scroll := ScrollContainer.new()
	scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	scroll.size_flags_vertical = Control.SIZE_EXPAND_FILL
	var rows := clampi(ceili(units.size() / float(CARD_COLUMNS)), 1, 3)
	scroll.custom_minimum_size = Vector2(0, rows * (CARD_SIZE.y + 4) + 2)
	scroll.add_child(grid)
	column.add_child(scroll)
	var composition := BattleUiKit.label(_composition(units), 14, BattleUiKit.INK_SOFT)
	composition.clip_text = true
	composition.text_overrun_behavior = TextServer.OVERRUN_TRIM_ELLIPSIS
	composition.tooltip_text = composition.text
	composition.mouse_filter = Control.MOUSE_FILTER_PASS
	column.add_child(composition)


## Ligne du général : portrait en médaillon (sinon blason), nom, commandement en étoiles.
func _general_row(general: Variant, faction: String, slot: int) -> Control:
	var row := HBoxContainer.new()
	row.add_theme_constant_override("separation", 10)
	row.alignment = BoxContainer.ALIGNMENT_BEGIN if slot == 0 else BoxContainer.ALIGNMENT_END
	var medallion := Control.new()
	medallion.custom_minimum_size = Vector2(76, 76)
	var character := ""
	var command := 0
	var name_text := "Sans chef"
	if general is Dictionary:
		character = str(general.get("character", ""))
		command = int(general.get("command", 0))
		name_text = str(general.get("name", ""))
	var portrait := PortraitLoader.portrait_texture(character)
	var arms := PortraitLoader.heraldry_texture(faction)
	var color := _colors[slot]
	medallion.draw.connect(func() -> void:
		var c := medallion.size * 0.5
		var r := minf(c.x, c.y) - 2.0
		medallion.draw_circle(c, r + 2.0, BattleUiKit.GOLD)
		medallion.draw_circle(c, r, color.darkened(0.3))
		if portrait != null:
			HudStyle.draw_texture_disc(medallion, portrait, c, r - 1.0)
		elif arms != null:
			HudStyle.draw_texture_fit(medallion, arms, c, r * 1.3)
		medallion.draw_arc(c, r, 0, TAU, 48, BattleUiKit.INK, 1.5))
	var texts := VBoxContainer.new()
	texts.alignment = BoxContainer.ALIGNMENT_CENTER
	var name_label := BattleUiKit.label(name_text, 19, INK, false, true)
	texts.add_child(name_label)
	var stars := "★".repeat(clampi(command, 0, 10)) + "☆".repeat(clampi(10 - command, 0, 10)) if general is Dictionary else "L'ost combat sans général : moral fragile."
	var stars_label := BattleUiKit.label(stars, 14, BattleUiKit.GOLD if general is Dictionary else BattleUiKit.RUBRIC)
	stars_label.tooltip_text = "Commandement %d / 10" % command
	stars_label.mouse_filter = Control.MOUSE_FILTER_PASS
	texts.add_child(stars_label)
	if slot == 0:
		row.add_child(medallion)
		row.add_child(texts)
	else:
		name_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_RIGHT
		stars_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_RIGHT
		row.add_child(texts)
		row.add_child(medallion)
	return row


func _fill_conditions(siege: bool) -> void:
	var terrain := str(setup.get("terrain", "plains"))
	var terrain_text: String = TERRAIN_FR.get(terrain, terrain)
	if bool(setup.get("river", false)):
		terrain_text += ", rivière et gués"
	var preview := _preview(setup, int(battle.get("seed", 1)))
	weather_label_text = "inconnue"
	site_label_text = ""
	if preview != null:
		weather_label_text = str(preview.call("get_weather").get("label", "inconnue")).to_lower()
		site_label_text = str(preview.call("get_site_label"))
	var parts := PackedStringArray([
		"Terrain : %s" % terrain_text,
		"Saison : %s" % str(SEASON_FR.get(str(setup.get("season", "")), "")),
		"Météo prévue : %s" % weather_label_text,
	])
	var text := "   ·   ".join(parts)
	if site_label_text != "":
		text += "\nSite : %s" % site_label_text
	body_label.text = text
	var modifiers: Array = forecast.get("modifiers", [])
	var mods := PackedStringArray()
	for entry in modifiers:
		mods.append(str(entry))
	if siege and mods.is_empty():
		mods.append("Brèche ouverte ou tours de siège : les murailles ne gênent plus l'assaut")
	modifiers_label.text = " · ".join(mods)
	modifiers_label.visible = not mods.is_empty()


## « 3 × Chevaliers, 2 × Archers… » depuis les régiments du setup.
func _composition(units: Array) -> String:
	var counts := {}
	var order: Array[String] = []
	for unit in units:
		var name := str(unit.get("name", "?"))
		if not counts.has(name):
			counts[name] = 0
			order.append(name)
		counts[name] += 1
	var parts: Array[String] = []
	for name in order:
		parts.append("%d × %s" % [counts[name], name])
	return ", ".join(parts)


## Aperçu de la bataille à livrer (même graine : même météo, même site), ou `null`.
func _preview(p_setup: Dictionary, seed: int) -> Object:
	if p_setup.is_empty() or not ClassDB.class_exists("BattleSim"):
		return null
	var preview: Object = ClassDB.instantiate("BattleSim")
	if not preview.call("setup", p_setup, seed):
		return null
	return preview
