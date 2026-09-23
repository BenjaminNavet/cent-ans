class_name PreBattleDialog
extends PanelContainer

## Dialogue d'avant-bataille (fin de tour, spec M7 § 4) : forces en présence, terrain, météo
## prévue, « Livrer bataille » / « Résolution automatique ». La météo est lue sur un
## `BattleSim` construit avec la même graine que la bataille qui sera livrée.

signal fight_requested(index: int, seed: int)
signal auto_requested(index: int)

const INK := Color(0.22, 0.14, 0.07)
const TERRAIN_FR := {
	"plains": "plaines", "hills": "collines", "mountains": "montagnes", "forest": "forêt",
	"marsh": "marais", "heath": "lande", "bocage": "bocage",
}
const SEASON_FR := {"spring": "printemps", "summer": "été", "autumn": "automne", "winter": "hiver"}

var battle: Dictionary = {}
var weather_label_text: String = ""
var title_label: Label
var body_label: Label
var fight_button: Button
var auto_button: Button


func _ready() -> void:
	theme = load("res://scenes/ui/parchment_theme.tres")
	custom_minimum_size = Vector2(680, 0)
	anchor_left = 0.5
	anchor_right = 0.5
	anchor_top = 0.5
	anchor_bottom = 0.5
	offset_left = -340
	offset_right = 340
	offset_top = -200
	var box := VBoxContainer.new()
	box.add_theme_constant_override("separation", 12)
	add_child(box)
	title_label = Label.new()
	title_label.add_theme_font_size_override("font_size", 26)
	title_label.add_theme_color_override("font_color", INK)
	title_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	box.add_child(title_label)
	body_label = Label.new()
	body_label.add_theme_font_size_override("font_size", 16)
	body_label.add_theme_color_override("font_color", INK)
	body_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	box.add_child(body_label)
	var buttons := HBoxContainer.new()
	buttons.alignment = BoxContainer.ALIGNMENT_CENTER
	buttons.add_theme_constant_override("separation", 16)
	box.add_child(buttons)
	fight_button = Button.new()
	fight_button.text = "Livrer bataille"
	fight_button.pressed.connect(func() -> void:
		visible = false
		fight_requested.emit(int(battle.get("index", 0)), int(battle.get("seed", 1))))
	buttons.add_child(fight_button)
	auto_button = Button.new()
	auto_button.text = "Résolution automatique"
	auto_button.pressed.connect(func() -> void:
		visible = false
		auto_requested.emit(int(battle.get("index", 0))))
	buttons.add_child(auto_button)
	visible = false


## Remplit le dialogue pour `p_battle` (entrée de `get_pending_battles`).
func show_battle(sim: Object, p_battle: Dictionary) -> void:
	battle = p_battle
	var index := int(battle.get("index", 0))
	var setup: Dictionary = sim.call("get_battle_setup", index)
	title_label.text = "Bataille en vue : %s" % str(battle.get("province_name", ""))
	var lines: Array[String] = []
	var player_side := str(battle.get("player_side", ""))
	for side in ["attacker", "defender"]:
		var side_setup: Dictionary = setup.get(side, {})
		var general: Variant = side_setup.get("general", null)
		var general_text := ""
		if general is Dictionary:
			general_text = ", menée par %s" % str(general.get("name", ""))
		var you := " (vous)" if side == player_side else ""
		lines.append("%s%s : %d hommes en %d régiments%s." % [
			str(battle.get("%s_name" % side, side)), you, int(battle.get("%s_strength" % side, 0)),
			(side_setup.get("units", []) as Array).size(), general_text,
		])
		lines.append("    " + _composition(side_setup.get("units", [])))
	var terrain := str(setup.get("terrain", "plains"))
	var terrain_text: String = TERRAIN_FR.get(terrain, terrain)
	if bool(setup.get("river", false)):
		terrain_text += ", rivière et gués"
	lines.append("Terrain : %s ; saison : %s." % [terrain_text, SEASON_FR.get(str(setup.get("season", "")), "")])
	weather_label_text = _forecast(setup, int(battle.get("seed", 1)))
	lines.append("Météo prévue : %s." % weather_label_text)
	body_label.text = "\n".join(lines)
	fight_button.disabled = setup.is_empty()
	visible = true


## « 3 hommes d'armes, 2 archers… » depuis les régiments du setup.
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


func _forecast(setup: Dictionary, seed: int) -> String:
	if setup.is_empty() or not ClassDB.class_exists("BattleSim"):
		return "inconnue"
	var preview: Object = ClassDB.instantiate("BattleSim")
	if not preview.call("setup", setup, seed):
		return "inconnue"
	return str(preview.call("get_weather").get("label", "inconnue")).to_lower()
