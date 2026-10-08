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
## NT11 : année de la bataille (1337-1453, bornes au cœur) qui filtre le roster selon les dates de
## disponibilité des unités ; engins de l'assiégeant (échelles, bélier, beffrois) choisis dans le
## menu « Engins » de la ligne du siège.

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

## Règles du cœur (`custom_rules`), factions jouables, rosters par « faction@année » (-> [entrées]).
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
var year_spin: SpinBox
var engines_button: MenuButton
## LR-12 (NT4) : l'armée adverse au joueur tient sa position (`set_hold` du cœur, appliqué au lancement).
var hold_check: CheckBox
## LR-12 : menu « Technologies » par camp (technologies accordées en plus de celles de la faction).
var tech_buttons: Dictionary = {}
var tech_catalogs: Dictionary = {}
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
		factions = _sim.call("custom_factions", DataFile.data_dir())
	config = _initial_config()
	_build()
	_sync_controls()
	refresh()
	UiZones.put(UiZones.Zone.MODAL, self)
	UiMotion.fade_in(self)


## Dossier `data/` (utilisé par les tests).
static func data_dir() -> String:
	return DataFile.data_dir()


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
		"attacker": {"faction": DEFAULT_FACTIONS["attacker"], "budget": int(rules.get("default_budget", 12000)), "units": [], "technologies": []},
		"defender": {"faction": DEFAULT_FACTIONS["defender"], "budget": int(rules.get("default_budget", 12000)), "units": [], "technologies": []},
		"terrain": "plains", "season": "summer", "weather": "", "hour": "",
		"siege": false, "place": "city", "fortification": int(rules.get("default_fortification", 2)), "player_side": "attacker",
		"year": int(rules.get("default_year", 1337)), "engines": default_engines(), "hold_opponent": false,
	}
	var saved := saved_config()
	for key in out:
		if not saved.has(key):
			continue
		if key == "engines":
			var engines: Dictionary = saved[key] if saved[key] is Dictionary else {}
			out[key] = {
				"ladders": bool(engines.get("ladders", out[key]["ladders"])),
				"ram": bool(engines.get("ram", out[key]["ram"])),
				"towers": clampi(int(engines.get("towers", out[key]["towers"])), 0, int(rules.get("max_siege_towers", 2))),
			}
		elif out[key] is Dictionary and saved[key] is Dictionary:
			var side: Dictionary = saved[key]
			out[key] = {
				"faction": str(side.get("faction", out[key]["faction"])),
				"budget": int(side.get("budget", out[key]["budget"])),
				"units": Array(side.get("units", [])).map(func(u: Variant) -> String: return str(u)),
				"technologies": Array(side.get("technologies", [])).map(func(t: Variant) -> String: return str(t)),
			}
		elif out[key] is bool:
			out[key] = bool(saved[key])
		elif out[key] is int:
			out[key] = int(saved[key])
		else:
			out[key] = str(saved[key])
	out["year"] = clampi(int(out["year"]), int(rules.get("min_year", 1337)), int(rules.get("max_year", 1453)))
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
	var box := UiBuild.vbox(8)
	add_child(box)
	var title := UiBuild.label("Bataille personnalisée")
	UiType.apply(title, UiType.TITLE)
	box.add_child(title)
	var hint := UiBuild.label("Chaque camp achète ses unités sur son budget de points. Hors campagne : le résultat n'est pas conservé.")
	UiType.apply(hint, UiType.CAPTION)
	hint.modulate = Color(1, 1, 1, 0.7)
	box.add_child(hint)
	var sides := UiBuild.hbox(16, box)
	for side in SIDES:
		sides.add_child(_build_side(side))
	box.add_child(_build_field())
	errors_label = Label.new()
	errors_label.name = "Errors"
	errors_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	UiType.apply(errors_label, UiType.CAPTION)
	errors_label.modulate = Color(1.0, 0.72, 0.6)
	box.add_child(errors_label)
	var row := UiBuild.hbox(8)
	row.alignment = BoxContainer.ALIGNMENT_END
	box.add_child(row)
	hold_check = CheckBox.new()
	hold_check.name = "HoldOpponent"
	hold_check.text = "Adversaire : tenir sa position"
	TooltipHost.attach_plain(hold_check, "custom_battle_hold")
	hold_check.toggled.connect(func(on: bool) -> void:
		if not _refreshing:
			config["hold_opponent"] = on)
	row.add_child(hold_check)
	var close_button := UiBuild.button("Fermer", close)
	close_button.name = "CloseButton"
	row.add_child(close_button)
	launch_button = UiBuild.button("Lancer la bataille")
	launch_button.name = "LaunchButton"
	launch_button.pressed.connect(func() -> void: BattlePrologueInvite.gate(self, launch))  # NT4 : invite au didacticiel
	row.add_child(launch_button)
	launch_button.grab_focus.call_deferred()


func _build_side(side: String) -> Control:
	var column := UiBuild.vbox(6)
	column.name = "Side_" + side
	column.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	var heading := UiBuild.label(SIDE_TITLES[side])
	UiType.apply(heading, UiType.HEADING)
	column.add_child(heading)
	var top := UiBuild.hbox(8, column)
	var faction := OptionButton.new()
	faction.name = "Faction"
	faction.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	faction.fit_to_longest_item = false
	for i in factions.size():
		faction.add_item(str(factions[i]["name"]), i)
	faction.item_selected.connect(func(index: int) -> void: set_faction(side, str(factions[index]["id"])))
	top.add_child(faction)
	faction_options[side] = faction
	var budget_label := UiBuild.label("Budget")
	TooltipHost.attach_plain(budget_label, "custom_battle_budget")
	budget_label.mouse_filter = Control.MOUSE_FILTER_STOP
	top.add_child(budget_label)
	var budget := SpinBox.new()
	budget.name = "Budget"
	budget.min_value = float(rules.get("min_budget", 1000))
	budget.max_value = float(rules.get("max_budget", 60000))
	budget.step = float(rules.get("budget_step", 500))
	budget.value_changed.connect(func(value: float) -> void: set_budget(side, int(value)))
	top.add_child(budget)
	budget_spins[side] = budget
	var tech_button := MenuButton.new()
	tech_button.name = "Technologies"
	tech_button.flat = false
	tech_button.text = "Technologies"
	TooltipHost.attach_plain(tech_button, "custom_battle_technologies")
	var tech_popup := tech_button.get_popup()
	tech_popup.hide_on_checkable_item_selection = false
	tech_popup.index_pressed.connect(func(index: int) -> void: toggle_technology(side, str(tech_popup.get_item_metadata(index))))
	top.add_child(tech_button)
	tech_buttons[side] = tech_button
	var points := Label.new()
	points.name = "Points"
	UiType.apply(points, UiType.BODY)
	column.add_child(points)
	points_labels[side] = points
	var lists := UiBuild.hbox(8, column)
	roster_boxes[side] = _list_column(lists, "Roster (clic : acheter)")
	army_boxes[side] = _list_column(lists, "Armée (clic : retirer)")
	var clear := UiBuild.button("Vider l'armée", func() -> void: clear_army(side))
	clear.name = "Clear"
	clear.size_flags_horizontal = Control.SIZE_SHRINK_END
	column.add_child(clear)
	return column


func _list_column(parent: Control, heading_text: String) -> VBoxContainer:
	var column := VBoxContainer.new()
	column.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	parent.add_child(column)
	var heading := UiBuild.label(heading_text)
	UiType.apply(heading, UiType.CAPTION)
	heading.modulate = Color(1, 1, 1, 0.75)
	column.add_child(heading)
	var scroll := ScrollContainer.new()
	scroll.custom_minimum_size = Vector2(0, 220)
	scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	column.add_child(scroll)
	var items := UiBuild.vbox(2)
	items.size_flags_horizontal = Control.SIZE_EXPAND_FILL
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
	# Fin de la ligne Heure/Joueur : l'année (NT11), puis ligne du siège : case, engins,
	# fortification, type de place.
	var year_label := UiBuild.label("Année")
	TooltipHost.attach_plain(year_label, "custom_battle_year")
	year_label.mouse_filter = Control.MOUSE_FILTER_STOP
	grid.add_child(year_label)
	year_spin = SpinBox.new()
	year_spin.name = "Year"
	year_spin.min_value = float(rules.get("min_year", 1337))
	year_spin.max_value = float(rules.get("max_year", 1453))
	year_spin.step = 1
	year_spin.value_changed.connect(func(value: float) -> void: set_year(int(value)))
	grid.add_child(year_spin)
	siege_check = CheckBox.new()
	siege_check.name = "Siege"
	siege_check.text = "Siège : le camp 2 défend une place"
	TooltipHost.attach_plain(siege_check, "custom_battle_siege")
	siege_check.toggled.connect(func(on: bool) -> void:
		config["siege"] = on
		refresh())
	grid.add_child(siege_check)
	engines_button = MenuButton.new()
	engines_button.name = "Engines"
	engines_button.flat = false
	TooltipHost.attach_plain(engines_button, "custom_battle_engines")
	var popup := engines_button.get_popup()
	popup.hide_on_checkable_item_selection = false
	popup.add_check_item("Échelles", 0)
	popup.add_check_item("Bélier", 1)
	for towers in int(rules.get("max_siege_towers", 2)) + 1:
		popup.add_radio_check_item(_towers_label(towers), 10 + towers)
	popup.id_pressed.connect(_on_engine_pressed)
	grid.add_child(engines_button)
	var fort_label := UiBuild.label("Fortification")
	TooltipHost.attach_plain(fort_label, "custom_battle_fortification")
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
	var label := UiBuild.label(text)
	TooltipHost.attach_plain(label, tooltip_key)
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
	year_spin.set_value_no_signal(float(config["year"]))
	hold_check.set_pressed_no_signal(bool(config.get("hold_opponent", false)))
	_sync_engines()
	_refreshing = false


static func _index_of(pairs: Array, key: String) -> int:
	for i in pairs.size():
		if str(pairs[i][0]) == key:
			return i
	return -1


# --- Composition -------------------------------------------------------------------------------


## Roster de `faction` pour l'année choisie (cache ; `[]` sans extension).
func roster(faction: String, techs: Array = []) -> Array:
	var year := int(config.get("year", rules.get("default_year", 1337)))
	var sorted_techs := techs.duplicate()
	sorted_techs.sort()
	var key := "%s@%d@%s" % [faction, year, ",".join(PackedStringArray(sorted_techs))]
	if not rosters.has(key):
		rosters[key] = _sim.call("custom_roster", DataFile.data_dir(), faction, year, PackedStringArray(techs)) if _sim != null else []
	return rosters[key]


## LR-12 : roster du camp (sa faction et les technologies qui lui sont accordées).
func side_roster(side: String) -> Array:
	return roster(str(config[side]["faction"]), config[side].get("technologies", []))


## LR-12 : technologies que la faction du camp peut recevoir cette année (`[{id, name, units}]`).
func grantable_technologies(side: String) -> Array:
	var year := int(config.get("year", rules.get("default_year", 1337)))
	var key := "%s@%d" % [config[side]["faction"], year]
	if not tech_catalogs.has(key):
		tech_catalogs[key] = _sim.call("custom_technologies", DataFile.data_dir(), str(config[side]["faction"]), year) if _sim != null else []
	return tech_catalogs[key]


## LR-12 : accorde ou retire une technologie ; les unités qu'elle débloquait quittent l'armée.
func toggle_technology(side: String, tech_id: String) -> void:
	if _refreshing:
		return
	var techs: Array = config[side]["technologies"]
	if techs.has(tech_id):
		techs.erase(tech_id)
	else:
		techs.append(tech_id)
	_prune_army(side)
	refresh()


## Retire de l'armée les unités hors roster (faction, année, technologies).
func _prune_army(side: String) -> void:
	config[side]["units"] = (config[side]["units"] as Array).filter(func(u: String) -> bool: return not roster_entry(str(config[side]["faction"]), u, config[side]["technologies"]).is_empty())


## NT11 : année de la bataille ; les unités qui ne sont plus levées cette année-là quittent les armées.
func set_year(year: int) -> void:
	if _refreshing:
		return
	year = clampi(year, int(rules.get("min_year", 1337)), int(rules.get("max_year", 1453)))
	if int(config["year"]) == year:
		return
	config["year"] = year
	for side in SIDES:
		_prune_army(side)
	refresh()


## Engins de l'assiégeant par défaut (règles du cœur).
func default_engines() -> Dictionary:
	var engines: Dictionary = rules.get("default_engines", {})
	return {
		"ladders": bool(engines.get("ladders", true)),
		"ram": bool(engines.get("ram", true)),
		"towers": int(engines.get("towers", 0)),
	}


## NT11 : choisit les engins de l'assiégeant (beffrois bornés par `max_siege_towers`).
func set_engines(ladders: bool, ram: bool, towers: int) -> void:
	config["engines"] = {"ladders": ladders, "ram": ram, "towers": clampi(towers, 0, int(rules.get("max_siege_towers", 2)))}
	_sync_engines()
	refresh()


func _on_engine_pressed(id: int) -> void:
	var engines: Dictionary = config["engines"]
	match id:
		0:
			set_engines(not bool(engines["ladders"]), bool(engines["ram"]), int(engines["towers"]))
		1:
			set_engines(bool(engines["ladders"]), not bool(engines["ram"]), int(engines["towers"]))
		_:
			set_engines(bool(engines["ladders"]), bool(engines["ram"]), id - 10)


static func _towers_label(towers: int) -> String:
	match towers:
		0:
			return "Aucun beffroi"
		1:
			return "1 beffroi"
	return "%d beffrois" % towers


## Coches du menu « Engins » et résumé sur le bouton.
func _sync_engines() -> void:
	if engines_button == null:
		return
	var engines: Dictionary = config["engines"]
	var popup := engines_button.get_popup()
	popup.set_item_checked(popup.get_item_index(0), bool(engines["ladders"]))
	popup.set_item_checked(popup.get_item_index(1), bool(engines["ram"]))
	for towers in int(rules.get("max_siege_towers", 2)) + 1:
		popup.set_item_checked(popup.get_item_index(10 + towers), towers == int(engines["towers"]))
	var parts := PackedStringArray()
	if bool(engines["ladders"]):
		parts.append("échelles")
	if bool(engines["ram"]):
		parts.append("bélier")
	if int(engines["towers"]) > 0:
		parts.append(_towers_label(int(engines["towers"])).to_lower())
	engines_button.text = "Engins : " + (", ".join(parts) if not parts.is_empty() else "aucun")


func roster_entry(faction: String, unit_id: String, techs: Array = []) -> Dictionary:
	for entry in roster(faction, techs):
		if str(entry["id"]) == unit_id:
			return entry
	return {}


func set_faction(side: String, faction: String) -> void:
	if _refreshing or str(config[side]["faction"]) == faction:
		return
	config[side]["faction"] = faction
	config[side]["technologies"] = []
	# Les unités hors du roster de la nouvelle faction sont retirées.
	_prune_army(side)
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
	var entry := roster_entry(str(config[side]["faction"]), unit_id, config[side]["technologies"])
	if entry.is_empty():
		return false
	var side_report: Dictionary = report.get(side, {})
	var cost := int(side_report.get("cost", 0))
	var budget := int(side_report.get("budget", config[side]["budget"]))
	var max_units := int(side_report.get("max_units", rules.get("max_units_per_side", 40)))
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
	out["year"] = int(out["year"])
	out["engines"]["towers"] = int(out["engines"]["towers"])
	return out


## Valide la composition au cœur puis redessine les listes, les points et les erreurs.
func refresh() -> void:
	if _sim != null:
		report = _sim.call("validate_custom", DataFile.data_dir(), battle_config())
	else:
		report = {"ok": false, "errors": ["Extension de simulation absente."]}
	for side in SIDES:
		_fill_technologies(side)
		_fill_roster(side)
		_fill_army(side)
		var side_report: Dictionary = report.get(side, {})
		var label: Label = points_labels[side]
		label.text = "Points : %d / %d — unités : %d / %d" % [int(side_report.get("cost", 0)), int(side_report.get("budget", 0)), int(side_report.get("units", 0)), int(side_report.get("max_units", 0))]
		label.modulate = Color(1.0, 0.6, 0.5) if int(side_report.get("cost", 0)) > int(side_report.get("budget", 0)) else Color(1, 1, 1)
	fortification_spin.editable = bool(config["siege"])
	place_option.disabled = not bool(config["siege"])
	engines_button.disabled = not bool(config["siege"])
	var errors: Array = report.get("errors", [])
	errors_label.text = "\n".join(PackedStringArray(errors))
	errors_label.visible = not errors.is_empty()
	launch_button.disabled = not bool(report.get("ok", false))


## VN : taille (px) des miniatures d'unité des listes (même taille que les lignes de recrutement).
const ROSTER_ICON := 22


## VN : miniature de l'unité sur le bouton (autoload `IconLibrary`, absent des tests isolés).
func _decorate(button: Button, unit_id: String) -> void:
	var library := get_node_or_null("/root/IconLibrary")
	if library != null:
		library.call("decorate_button", button, unit_id, ROSTER_ICON, "unit")


## LR-12 : coches du menu « Technologies » du camp et résumé sur le bouton.
func _fill_technologies(side: String) -> void:
	var button: MenuButton = tech_buttons[side]
	var popup := button.get_popup()
	popup.clear()
	var granted: Array = config[side]["technologies"]
	var catalog := grantable_technologies(side)
	for entry in catalog:
		var index := popup.item_count
		popup.add_check_item("%s (%s)" % [entry["name"], ", ".join(PackedStringArray(entry["units"]))])
		popup.set_item_metadata(index, str(entry["id"]))
		popup.set_item_checked(index, granted.has(str(entry["id"])))
	button.disabled = catalog.is_empty()
	button.text = "Technologies" if granted.is_empty() else "Technologies (%d)" % granted.size()


func _fill_roster(side: String) -> void:
	var items: VBoxContainer = roster_boxes[side]
	for child in items.get_children():
		child.queue_free()
	for entry in side_roster(side):
		var unit_id := str(entry["id"])
		var button := RichButton.new()
		button.name = "Buy_" + unit_id
		button.text = "%s — %d" % [entry["name"], int(entry["cost"])]
		button.alignment = HORIZONTAL_ALIGNMENT_LEFT
		button.clip_text = true
		button.disabled = not can_buy(side, unit_id)
		_decorate(button, unit_id)  # VN : miniature de l'unité
		TooltipHost.set_tooltip(button, "unit", unit_id)
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
		_decorate(button, unit_id)
		TooltipHost.set_tooltip(button, "unit", unit_id)
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
