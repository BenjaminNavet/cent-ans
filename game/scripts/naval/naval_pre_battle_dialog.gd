class_name NavalPreBattleDialog
extends PreBattleDialog

## Écran d'avant-bataille navale (lot NV1), même présentation qu'UB1 (`PreBattleDialog`) :
## « Flotte ennemie en vue », chances estimées par le cœur (`win_chance` de
## `get_pending_naval_battles`, cinq résolutions automatiques), navires et équipages des deux
## flottes, vent prévu (aperçu `NavalBattleSim` à la même graine), puis « Résolution
## automatique » / « Retraite » (la flotte interceptée rentre au port). « Combattre » (scène 3D)
## est masqué depuis le 25/09 à la demande du joueur (`PLAYABLE_3D`).

const BANNER_SEA := "res://assets/events/evt_sluys.jpg"
## Bataille navale jouée en 3D proposée au joueur ; désactivée (résolution automatique seule).
const PLAYABLE_3D := false
const COMPASS := ["nord", "nord-est", "est", "sud-est", "sud", "sud-ouest", "ouest", "nord-ouest"]


func show_naval(sim: Object, p_battle: Dictionary) -> void:
	battle = p_battle
	var index := int(battle.get("index", 0))
	setup = sim.call("get_naval_battle_setup", index)
	player_side = str(battle.get("player_side", "attacker"))
	if player_side == "" or player_side == "<null>":
		player_side = "attacker"
	var enemy_side := "defender" if player_side == "attacker" else "attacker"
	# Les eaux de la traversée (« le pas de Calais ») plutôt que la mer entière (NV2).
	var waters := str(setup.get("place_name", ""))
	title_label.text = "Flotte ennemie en vue : %s" % (waters if waters != "" else str(battle.get("sea_name", "en mer")))
	subtitle_label.text = "%s barre la traversée de %s · %s" % [str(battle.get("interceptor_name", "")), str(battle.get("faction_name", "")), str(SEASON_FR.get(str(setup.get("season", "")), ""))]
	_banner_texture = ArtPlates.texture(ArtPlates.random_loading_screen("naval"))  # AR1
	if _banner_texture == null:
		_banner_texture = PortraitLoader.load_texture(BANNER_SEA)
	banner.queue_redraw()
	for i in 2:
		var side := player_side if i == 0 else enemy_side
		var fallback := Color(0.2, 0.3, 0.75) if i == 0 else Color(0.75, 0.15, 0.12)
		_colors[i] = BattleUiKit.faction_color(str((setup.get(side, {}) as Dictionary).get("faction", "")), fallback)
	if _colors[0].is_equal_approx(_colors[1]):
		_colors[1] = _colors[1].darkened(0.4)
	# Chances : part des résolutions automatiques gagnées par le joueur.
	var chance := float(battle.get("win_chance", 0.5))
	var own_men := float(battle.get("army_men", 1)) if player_side == "defender" else float(battle.get("interceptor_men", 1))
	var foe_men := float(battle.get("interceptor_men", 1)) if player_side == "defender" else float(battle.get("army_men", 1))
	_share = own_men / maxf(own_men + foe_men, 1.0)
	verdict_label.text = BattleUiKit.verdict(chance)
	verdict_label.add_theme_color_override("font_color", BattleUiKit.verdict_color(chance))
	chance_label.text = "Chances de victoire estimées : %d %% · %d navires contre %d" % [roundi(chance * 100.0), _ships_of(player_side), _ships_of(enemy_side)]
	balance_bar.tooltip_text = "Estimation du cœur : cinq combats résolus automatiquement avec les mêmes flottes (navires, hauteur de bord, archers aux châteaux, hommes d'armes pour l'abordage)."
	balance_bar.queue_redraw()
	for i in 2:
		_fill_fleet(_columns[i], player_side if i == 0 else enemy_side)
	body_label.text = _conditions(int(battle.get("seed", 1)))
	modifiers_label.text = "Les archers tirent des châteaux ; le plus haut bord domine l'abordage ; qui tient le vent choisit l'heure du combat."
	modifiers_label.visible = true
	fight_button.text = "Combattre"
	fight_button.disabled = setup.is_empty()
	# Choix du joueur (25/09) : les batailles navales se règlent par la résolution automatique ;
	# la scène 3D reste accessible en debug (`--naval-scenario`).
	fight_button.visible = PLAYABLE_3D
	withdraw_button.text = "Rentrer au port" if player_side == "defender" else "Refuser le combat"
	withdraw_button.disabled = false
	withdraw_button.tooltip_text = "La flotte interceptée regagne son port sans combattre : l'armée ne traverse pas ce tour." if player_side == "defender" else "L'escadre ne s'engage pas ; la flotte ennemie, menacée, regagne son port."
	_layout()
	if not visible:
		UiSounds.play("alert")
	visible = true
	_layout.call_deferred()


func _ships_of(side: String) -> int:
	return ((setup.get(side, {}) as Dictionary).get("ships", []) as Array).size()


## Colonne d'une flotte : faction, amiral, navires par classe, hommes embarqués par régiment.
func _fill_fleet(column: VBoxContainer, side: String) -> void:
	for child in column.get_children():
		column.remove_child(child)
		child.queue_free()
	var side_setup: Dictionary = setup.get(side, {})
	var faction := str(side_setup.get("faction", ""))
	var head := HBoxContainer.new()
	head.add_theme_constant_override("separation", 8)
	column.add_child(head)
	var arms := TextureRect.new()
	arms.texture = PortraitLoader.heraldry_texture(faction)
	arms.custom_minimum_size = Vector2(40, 48)
	arms.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
	arms.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_CENTERED
	head.add_child(arms)
	var names := VBoxContainer.new()
	head.add_child(names)
	names.add_child(BattleUiKit.label(str(side_setup.get("faction_name", faction)), 22, BattleUiKit.INK, true))
	var admiral := str(side_setup.get("admiral", ""))
	if admiral != "" and admiral != "<null>":
		names.add_child(BattleUiKit.label("Amiral : %s" % admiral, 15, BattleUiKit.INK_SOFT))
	var ships: Array = side_setup.get("ships", [])
	var classes := {}
	var order: Array[String] = []
	var soldiers := 0
	for ship in ships:
		var ship_class: Dictionary = ship.get("class", {})
		var class_name_text := str((ship_class.get("name", {}) as Dictionary).get("display", ship_class.get("id", "?")))
		if not classes.has(class_name_text):
			classes[class_name_text] = 0
			order.append(class_name_text)
		classes[class_name_text] += 1
		for crew in ship.get("crew", []):
			soldiers += int(crew.get("men", 0))
	var parts: Array[String] = []
	for name in order:
		parts.append("%d %s%s" % [classes[name], name.to_lower(), "s" if int(classes[name]) > 1 else ""])
	var fleet := BattleUiKit.label("%d navires : %s" % [ships.size(), ", ".join(parts)], 17, BattleUiKit.INK, false, true)
	fleet.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	column.add_child(fleet)
	column.add_child(BattleUiKit.label("%s hommes embarqués" % BattleUiKit.thousands(soldiers), 16))
	var units: Array = side_setup.get("units", [])
	var composition := _composition(units)
	if composition != "":
		var label := BattleUiKit.label(composition, 14, BattleUiKit.INK_SOFT)
		label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		column.add_child(label)
	var list := VBoxContainer.new()
	list.add_theme_constant_override("separation", 0)
	column.add_child(list)
	for i in mini(ships.size(), 8):
		var ship: Dictionary = ships[i]
		var men := 0
		for crew in ship.get("crew", []):
			men += int(crew.get("men", 0))
		var ship_class: Dictionary = ship.get("class", {})
		var line := "%s%s — %s, %d hommes" % ["⚑ " if bool(ship.get("flagship", false)) else "", str(ship.get("name", "")), str((ship_class.get("name", {}) as Dictionary).get("display", "")), men]
		list.add_child(BattleUiKit.label(line, 14, BattleUiKit.INK))
	if ships.size() > 8:
		list.add_child(BattleUiKit.label("… et %d autres" % (ships.size() - 8), 14, BattleUiKit.INK_FADED))


## Saison et vent prévus (aperçu de la bataille à la même graine).
func _conditions(seed: int) -> String:
	var parts := PackedStringArray(["Saison : %s" % str(SEASON_FR.get(str(setup.get("season", "")), ""))])
	if not setup.is_empty() and ClassDB.class_exists("NavalBattleSim"):
		var preview: Object = ClassDB.instantiate("NavalBattleSim")
		if preview.call("setup", setup, seed):
			var wind: Dictionary = preview.call("get_wind")
			var from := float(wind.get("to", 0.0)) + PI
			var bearing := fposmod(rad_to_deg(atan2(cos(from), -sin(from))), 360.0)
			var strength := float(wind.get("strength", 0.5))
			var force := "faible" if strength < 0.3 else ("frais" if strength < 0.65 else "fort")
			parts.append("Vent %s du %s" % [force, COMPASS[int(round(bearing / 45.0)) % 8]])
			var gauge := str(wind.get("gauge", ""))
			if gauge == player_side:
				parts.append("vous avez l'avantage du vent")
			elif gauge != "":
				parts.append("l'ennemi a l'avantage du vent")
	if bool(setup.get("rain", false)):
		parts.append("pluie : cordes d'arc mouillées")
	return "   ·   ".join(parts)
