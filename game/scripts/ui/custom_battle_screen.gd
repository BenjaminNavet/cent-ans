class_name CustomBattleScreen
extends PanelContainer

## NT2 — « Bataille personnalisée » du menu principal : deux camps, chacun une faction jouable,
## un budget de points et les unités achetées dans le roster de la faction (coût = coût de
## recrutement) ; champ (terrain, saison, météo, heure), option siège (le camp 2 défend une
## place) et camp mené par le joueur. Les règles (roster, budget, plafond d'unités, construction
## de la bataille) sont au cœur (`BattleSim.custom_*`, `sim_battle::custom`) ; cet écran ne fait
## que composer le dictionnaire de configuration et afficher le rapport de `validate_custom`.
## « Lancer la bataille » passe la configuration à `BattleScene.custom_config` puis change de
## scène, comme les batailles de démonstration. La dernière composition est gardée dans les
## réglages (`custom_battle/last`). Échap ou « Fermer » : signal `closed`.

signal closed
signal battle_started(config: Dictionary)

const BATTLE_SCENE := "res://scenes/battle/battle.tscn"
const SETTINGS_KEY := "custom_battle/last"
const SIDES: Array[String] = ["attacker", "defender"]
const SIDE_TITLES := {"attacker": "Camp 1 — assaillant", "defender": "Camp 2 — défenseur"}
## Terrains des champs de bataille (clés de `Terrain` au cœur) et leurs libellés (mêmes que le
## panneau de province).
const TERRAINS := [["plains", "Plaines"], ["hills", "Collines"], ["mountains", "Montagnes"], ["forest", "Forêt"], ["marsh", "Marais"], ["heath", "Lande"], ["bocage", "Bocage"], ["steppe", "Steppe"], ["desert", "Désert"]]
const SEASONS := [["spring", "Printemps"], ["summer", "Été"], ["autumn", "Automne"], ["winter", "Hiver"]]
const WEATHERS := [["", "Selon la saison"], ["clear", "Temps clair"], ["rain", "Pluie"], ["fog", "Brouillard"], ["snow", "Neige"]]
const PLAYER_SIDES := [["attacker", "Camp 1"], ["defender", "Camp 2"], ["", "Aucun (IA contre IA)"]]
const PLACES := [["city", "Cité"], ["borough", "Bourg fortifié"], ["castle", "Château"]]
const CATEGORY_LABELS := {"infantry": "Infanterie", "ranged": "Tireurs", "cavalry": "Cavalerie", "siege": "Siège"}
const DEFAULT_FACTIONS := {"attacker": "fac_france", "defender": "fac_england"}

## Règles du cœur (`custom_rules`), factions jouables, rosters par faction (id -> [entrées]).
var rules: Dictionary = {}
var factions: Array = []
var rosters: Dictionary = {}
## Composition en cours (même forme que la configuration passée au cœur).
var config: Dictionary = {}
## Dernier rapport de `validate_custom` ({ok, errors, attacker: {cost, budget, units, max_units}, …}).
var report: Dictionary = {}

## Contrôles (tests) : par camp.
var faction_options: Dictionary = {}
var budget_spins: Dictionary = {}
var points_labels: Dictionary = {}
var roster_boxes: Dictionary = {}
var army_boxes: Dictionary = {}
var terrain_option: OptionButton
var season_option: OptionButton
var weather_option: OptionButton
var hour_option: OptionButton
var siege_check: CheckBox
var fortification_spin: SpinBox
var place_option: OptionButton
var player_option: OptionButton
var errors_label: Label
var launch_button: Button

var _sim: Object = null
var _hour_keys: Array[String] = [""]
var _refreshing := false


func _ready() -> void:
	theme = load("res://scenes/ui/parchment_theme.tres")
	mouse_filter = Control.MOUSE_FILTER_STOP
	custom_minimum_size = Vector2(1120, 0)
	if ClassDB.class_exists("BattleSim"):
		_sim = ClassDB.instantiate("BattleSim")
		rules = _sim.call("custom_rules")
		factions = _sim.call("custom_factions", data_dir())
	config = _initial_config()
	_build()
	_sync_controls()
	refresh()
	UiZones.put(UiZones.Zone.MODAL, self)
	UiMotion.fade_in(self)


## Dossier des données (mêmes règles que les batailles historiques).
static func data_dir() -> String:
	return HistoricalBattlesMenu.data_dir()


## Dernière composition gardée dans les réglages ({} si aucune ou illisible).
static func saved_config() -> Dictionary:
	var tree := Engine.get_main_loop() as SceneTree
	var settings: Node = tree.root.get_node_or_null("/root/Settings") if tree != null else null
	if settings == null:
		return {}
	var text := str(settings.call("get_value", SETTINGS_KEY))
	if text.strip_edges() == "":
		return {}
	var parsed: Variant = JSON.parse_string(text)
	return parsed if parsed is Dictionary else {}


static func save_config(value: Dictionary) -> void:
	var tree := Engine.get_main_loop() as SceneTree
	var settings: Node = tree.root.get_node_or_null("/root/Settings") if tree != null else null
	if settings != null:
		settings.call("set_value", SETTINGS_KEY, JSON.stringify(value))


func _initial_config() -> Dictionary:
	var out := {
		"attacker": {"faction": DEFAULT_FACTIONS["attacker"], "budget": int(rules.get("default_budget", 6000)), "units": []},
		"defender": {"faction": DEFAULT_FACTIONS["defender"], "budget": int(rules.get("default_budget", 6000)), "units": []},
		"terrain": "plains", "season": "summer", "weather": "", "hour": "",
		"siege": false, "place": "city", "fortification": int(rules.get("default_fortification", 2)), "player_side": "attacker",
	}
	var saved := saved_config()
	for key in out:
		if not saved.has(key):
			continue
		if out[key] is Dictionary and saved[key] is Dictionary:
			var side: Dictionary = saved[key]
			out[key] = {
				"faction": str(side.get("faction", out[key]["faction"])),
				"budget": int(side.get("budget", out[key]["budget"])),
				"units": Array(side.get("units", [])).map(func(u: Variant) -> String: return str(u)),
			}
		elif out[key] is bool:
			out[key] = bool(saved[key])
		elif out[key] is int:
			out[key] = int(saved[key])
		else:
			out[key] = str(saved[key])
	for side in SIDES:
		if not _has_faction(str(out[side]["faction"])):
			out[side]["faction"] = DEFAULT_FACTIONS[side] if _has_faction(DEFAULT_FACTIONS[side]) else (str(factions[0]["id"]) if not factions.is_empty() else "")
	return out


func _has_faction(id: String) -> bool:
	for entry in factions:
		if str(entry["id"]) == id:
			return true
	return false


# --- Construction ------------------------------------------------------------------------------


func _build() -> void:
	var box := VBoxContainer.new()
	box.add_theme_constant_override("separation", 8)
	add_child(box)
	var title := Label.new()
	title.text = "Bataille personnalisée"
	UiType.apply(title, UiType.TITLE)
	box.add_child(title)
	var hint := Label.new()
	hint.text = "Chaque camp achète ses unités sur son budget de points. Hors campagne : le résultat n'est pas conservé."
	UiType.apply(hint, UiType.CAPTION)
	hint.modulate = Color(1, 1, 1, 0.7)
	box.add_child(hint)
	var sides := HBoxContainer.new()
	sides.add_theme_constant_override("separation", 16)
	box.add_child(sides)
	for side in SIDES:
		sides.add_child(_build_side(side))
	box.add_child(_build_field())
	errors_label = Label.new()
	errors_label.name = "Errors"
	errors_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	UiType.apply(errors_label, UiType.CAPTION)
	errors_label.modulate = Color(1.0, 0.72, 0.6)
	box.add_child(errors_label)
	var row := HBoxContainer.new()
	row.alignment = BoxContainer.ALIGNMENT_END
	row.add_theme_constant_override("separation", 8)
	box.add_child(row)
	var close_button := Button.new()
	close_button.name = "CloseButton"
	close_button.text = "Fermer"
	close_button.pressed.connect(close)
	row.add_child(close_button)
	launch_button = Button.new()
	launch_button.name = "LaunchButton"
	launch_button.text = "Lancer la bataille"
	launch_button.pressed.connect(func() -> void: BattlePrologueInvite.gate(self, launch))  # NT4 : invite au didacticiel
	row.add_child(launch_button)
	launch_button.grab_focus.call_deferred()


func _build_side(side: String) -> Control:
	var column := VBoxContainer.new()
	column.name = "Side_" + side
	column.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	column.add_theme_constant_override("separation", 6)
	var heading := Label.new()
	heading.text = SIDE_TITLES[side]
	UiType.apply(heading, UiType.HEADING)
	column.add_child(heading)
	var top := HBoxContainer.new()
	top.add_theme_constant_override("separation", 8)
	column.add_child(top)
	var faction := OptionButton.new()
	faction.name = "Faction"
	faction.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	faction.fit_to_longest_item = false
	for i in factions.size():
		faction.add_item(str(factions[i]["name"]), i)
	faction.item_selected.connect(func(index: int) -> void: set_faction(side, str(factions[index]["id"])))
	top.add_child(faction)
	faction_options[side] = faction
	var budget_label := Label.new()
	budget_label.text = "Budget"
	RichTooltip.attach_plain(budget_label, "custom_battle_budget")
	budget_label.mouse_filter = Control.MOUSE_FILTER_STOP
	top.add_child(budget_label)
	var budget := SpinBox.new()
	budget.name = "Budget"
	budget.min_value = float(rules.get("min_budget", 1000))
	budget.max_value = float(rules.get("max_budget", 30000))
	budget.step = float(rules.get("budget_step", 500))
	budget.value_changed.connect(func(value: float) -> void: set_budget(side, int(value)))
	top.add_child(budget)
	budget_spins[side] = budget
	var points := Label.new()
	points.name = "Points"
	UiType.apply(points, UiType.BODY)
	column.add_child(points)
	points_labels[side] = points
	var lists := HBoxContainer.new()
	lists.add_theme_constant_override("separation", 8)
	column.add_child(lists)
	roster_boxes[side] = _list_column(lists, "Roster (clic : acheter)")
	army_boxes[side] = _list_column(lists, "Armée (clic : retirer)")
	var clear := Button.new()
	clear.name = "Clear"
	clear.text = "Vider l'armée"
	clear.size_flags_horizontal = Control.SIZE_SHRINK_END
	clear.pressed.connect(func() -> void: clear_army(side))
	column.add_child(clear)
	return column


func _list_column(parent: Control, heading_text: String) -> VBoxContainer:
	var column := VBoxContainer.new()
	column.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	parent.add_child(column)
	var heading := Label.new()
	heading.text = heading_text
	UiType.apply(heading, UiType.CAPTION)
	heading.modulate = Color(1, 1, 1, 0.75)
	column.add_child(heading)
	var scroll := ScrollContainer.new()
	scroll.custom_minimum_size = Vector2(0, 220)
	scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	column.add_child(scroll)
	var items := VBoxContainer.new()
	items.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	items.add_theme_constant_override("separation", 2)
	scroll.add_child(items)
	return items


func _build_field() -> Control:
	var grid := GridContainer.new()
	grid.name = "Field"
	grid.columns = 6
	grid.add_theme_constant_override("h_separation", 8)
	grid.add_theme_constant_override("v_separation", 6)
	terrain_option = _field_option(grid, "Terrain", "custom_battle_terrain")
	for entry in TERRAINS:
		terrain_option.add_item(entry[1])
	terrain_option.item_selected.connect(func(i: int) -> void: _set_key("terrain", TERRAINS[i][0]))
	season_option = _field_option(grid, "Saison", "custom_battle_season")
	for entry in SEASONS:
		season_option.add_item(entry[1])
	season_option.item_selected.connect(func(i: int) -> void: _set_key("season", SEASONS[i][0]))
	weather_option = _field_option(grid, "Météo", "custom_battle_weather")
	for entry in WEATHERS:
		weather_option.add_item(entry[1])
	weather_option.item_selected.connect(func(i: int) -> void: _set_key("weather", WEATHERS[i][0]))
	hour_option = _field_option(grid, "Heure", "battle_demo_hour")
	hour_option.add_item("Au hasard (comme en campagne)")
	for phase in BattleDemosMenu.load_day_phases():
		_hour_keys.append(str(phase["key"]))
		hour_option.add_item(str(phase["label"]))
	hour_option.item_selected.connect(func(i: int) -> void: _set_key("hour", _hour_keys[i]))
	player_option = _field_option(grid, "Joueur", "custom_battle_player")
	for entry in PLAYER_SIDES:
		player_option.add_item(entry[1])
	player_option.item_selected.connect(func(i: int) -> void: _set_key("player_side", PLAYER_SIDES[i][0]))
	siege_check = CheckBox.new()
	siege_check.name = "Siege"
	siege_check.text = "Siège : le camp 2 défend une place"
	RichTooltip.attach_plain(siege_check, "custom_battle_siege")
	siege_check.toggled.connect(func(on: bool) -> void:
		config["siege"] = on
		refresh())
	grid.add_child(siege_check)
	var fort_label := Label.new()
	fort_label.text = "Fortification"
	RichTooltip.attach_plain(fort_label, "custom_battle_fortification")
	fort_label.mouse_filter = Control.MOUSE_FILTER_STOP
	grid.add_child(fort_label)
	fortification_spin = SpinBox.new()
	fortification_spin.name = "Fortification"
	fortification_spin.min_value = 0
	fortification_spin.max_value = float(rules.get("max_fortification", 3))
	fortification_spin.step = 1
	fortification_spin.value_changed.connect(func(value: float) -> void:
		config["fortification"] = int(value)
		refresh())
	grid.add_child(fortification_spin)
	place_option = _field_option(grid, "Type de place", "custom_battle_place")
	for entry in PLACES:
		place_option.add_item(entry[1])
	place_option.item_selected.connect(func(i: int) -> void: _set_key("place", PLACES[i][0]))
	return grid


func _field_option(grid: GridContainer, text: String, tooltip_key: String) -> OptionButton:
	var label := Label.new()
	label.text = text
	RichTooltip.attach_plain(label, tooltip_key)
	label.mouse_filter = Control.MOUSE_FILTER_STOP
	grid.add_child(label)
	var option := OptionButton.new()
	option.name = text
	grid.add_child(option)
	return option


## Reporte `config` dans les contrôles (sans déclencher de changement).
func _sync_controls() -> void:
	_refreshing = true
	for side in SIDES:
		var faction_id := str(config[side]["faction"])
		for i in factions.size():
			if str(factions[i]["id"]) == faction_id:
				(faction_options[side] as OptionButton).selected = i
		(budget_spins[side] as SpinBox).set_value_no_signal(float(config[side]["budget"]))
	terrain_option.selected = maxi(_index_of(TERRAINS, str(config["terrain"])), 0)
	season_option.selected = maxi(_index_of(SEASONS, str(config["season"])), 0)
	weather_option.selected = maxi(_index_of(WEATHERS, str(config["weather"])), 0)
	hour_option.selected = maxi(_hour_keys.find(str(config["hour"])), 0)
	player_option.selected = maxi(_index_of(PLAYER_SIDES, str(config["player_side"])), 0)
	siege_check.set_pressed_no_signal(bool(config["siege"]))
	fortification_spin.set_value_no_signal(float(config["fortification"]))
	place_option.selected = maxi(_index_of(PLACES, str(config.get("place", "city"))), 0)
	_refreshing = false


static func _index_of(pairs: Array, key: String) -> int:
	for i in pairs.size():
		if str(pairs[i][0]) == key:
			return i
	return -1


# --- Composition -------------------------------------------------------------------------------


## Roster de `faction` (cache ; `[]` sans extension).
func roster(faction: String) -> Array:
	if not rosters.has(faction):
		rosters[faction] = _sim.call("custom_roster", data_dir(), faction) if _sim != null else []
	return rosters[faction]


func roster_entry(faction: String, unit_id: String) -> Dictionary:
	for entry in roster(faction):
		if str(entry["id"]) == unit_id:
			return entry
	return {}


func set_faction(side: String, faction: String) -> void:
	if _refreshing or str(config[side]["faction"]) == faction:
		return
	config[side]["faction"] = faction
	# Les unités hors du roster de la nouvelle faction sont retirées.
	config[side]["units"] = (config[side]["units"] as Array).filter(func(u: String) -> bool: return not roster_entry(faction, u).is_empty())
	refresh()


func set_budget(side: String, value: int) -> void:
	if _refreshing:
		return
	config[side]["budget"] = value
	refresh()


func _set_key(key: String, value: String) -> void:
	if _refreshing:
		return
	config[key] = value
	refresh()


## Vrai si `side` peut encore acheter `unit_id` (budget et plafond d'unités du rapport du cœur).
func can_buy(side: String, unit_id: String) -> bool:
	var entry := roster_entry(str(config[side]["faction"]), unit_id)
	if entry.is_empty():
		return false
	var side_report: Dictionary = report.get(side, {})
	var cost := int(side_report.get("cost", 0))
	var budget := int(side_report.get("budget", config[side]["budget"]))
	var max_units := int(side_report.get("max_units", rules.get("max_units_per_side", 20)))
	return cost + int(entry["cost"]) <= budget and (config[side]["units"] as Array).size() < max_units


## Achète `unit_id` pour `side` ; faux (rien ne change) si le budget ou le plafond l'interdit.
func buy(side: String, unit_id: String) -> bool:
	if not can_buy(side, unit_id):
		return false
	(config[side]["units"] as Array).append(unit_id)
	refresh()
	return true


## Retire la `index`-ième unité de l'armée de `side`.
func remove(side: String, index: int) -> void:
	var units: Array = config[side]["units"]
	if index >= 0 and index < units.size():
		units.remove_at(index)
		refresh()


func clear_army(side: String) -> void:
	(config[side]["units"] as Array).clear()
	refresh()


## Configuration passée au cœur (copie ; entiers pour les budgets et la fortification).
func battle_config() -> Dictionary:
	var out := config.duplicate(true)
	for side in SIDES:
		out[side]["budget"] = int(out[side]["budget"])
	out["fortification"] = int(out["fortification"])
	return out


## Valide la composition au cœur puis redessine les listes, les points et les erreurs.
func refresh() -> void:
	if _sim != null:
		report = _sim.call("validate_custom", data_dir(), battle_config())
	else:
		report = {"ok": false, "errors": ["Extension de simulation absente."]}
	for side in SIDES:
		_fill_roster(side)
		_fill_army(side)
		var side_report: Dictionary = report.get(side, {})
		var label: Label = points_labels[side]
		label.text = "Points : %d / %d — unités : %d / %d" % [int(side_report.get("cost", 0)), int(side_report.get("budget", 0)), int(side_report.get("units", 0)), int(side_report.get("max_units", 0))]
		label.modulate = Color(1.0, 0.6, 0.5) if int(side_report.get("cost", 0)) > int(side_report.get("budget", 0)) else Color(1, 1, 1)
	fortification_spin.editable = bool(config["siege"])
	place_option.disabled = not bool(config["siege"])
	var errors: Array = report.get("errors", [])
	errors_label.text = "\n".join(PackedStringArray(errors))
	errors_label.visible = not errors.is_empty()
	launch_button.disabled = not bool(report.get("ok", false))


func _fill_roster(side: String) -> void:
	var items: VBoxContainer = roster_boxes[side]
	for child in items.get_children():
		child.queue_free()
	for entry in roster(str(config[side]["faction"])):
		var unit_id := str(entry["id"])
		var button := RichButton.new()
		button.name = "Buy_" + unit_id
		button.text = "%s — %d" % [entry["name"], int(entry["cost"])]
		button.alignment = HORIZONTAL_ALIGNMENT_LEFT
		button.clip_text = true
		button.disabled = not can_buy(side, unit_id)
		RichTooltip.set_tooltip(button, "unit", unit_id)
		button.pressed.connect(func() -> void: buy(side, unit_id))
		items.add_child(button)


func _fill_army(side: String) -> void:
	var items: VBoxContainer = army_boxes[side]
	for child in items.get_children():
		child.queue_free()
	var units: Array = config[side]["units"]
	var faction := str(config[side]["faction"])
	for index in units.size():
		var unit_id := str(units[index])
		var entry := roster_entry(faction, unit_id)
		var button := RichButton.new()
		button.name = "Army_%d" % index
		button.text = "− %s (%d)" % [entry.get("name", unit_id), int(entry.get("cost", 0))]
		button.alignment = HORIZONTAL_ALIGNMENT_LEFT
		button.clip_text = true
		RichTooltip.set_tooltip(button, "unit", unit_id)
		button.pressed.connect(func() -> void: remove(side, index))
		items.add_child(button)


# --- Lancement ---------------------------------------------------------------------------------


## Mémorise la composition puis lance la bataille (même chemin que les démonstrations).
func launch() -> bool:
	if not bool(report.get("ok", false)):
		return false
	var value := battle_config()
	save_config(value)
	BattleScene.custom_config = value
	battle_started.emit(value)
	var audio := get_node_or_null("/root/AudioDirector")
	if audio != null and audio.has_method("stop_all"):
		audio.call("stop_all")
	SceneFader.go(BATTLE_SCENE)
	return true


func _unhandled_input(event: InputEvent) -> void:
	if visible and event.is_action_pressed("ui_cancel"):
		close()
		get_viewport().set_input_as_handled()


func close() -> void:
	save_config(battle_config())
	closed.emit()
	UiMotion.fade_out(self, UiMotion.DURATION, true)
