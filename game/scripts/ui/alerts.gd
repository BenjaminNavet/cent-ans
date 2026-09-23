class_name AlertsPanel
extends VBoxContainer

## F3 — alertes persistantes de la carte (colonne à droite, sous la barre) : armée ennemie
## aux frontières, province assiégée, dette, recherche inactive, bâtiment terminé, décision de
## chronique en attente. Chaque alerte reste tant que sa condition tient (recalculée à chaque
## rafraîchissement de la carte) ; un clic émet `alert_pressed` (la carte centre la caméra
## ou ouvre le panneau). Les conditions ne font que lire l'état exposé par la simulation.

signal alert_pressed(alert: Dictionary)

const MAX_ALERTS := 8
const SEVERITY_COLORS := {
	"danger": Color(0.55, 0.10, 0.08),
	"warning": Color(0.50, 0.32, 0.05),
	"info": Color(0.18, 0.28, 0.42),
}

var alerts: Array = []


func _ready() -> void:
	theme = load("res://scenes/ui/parchment_theme.tres")
	set_anchors_preset(Control.PRESET_TOP_RIGHT)
	offset_left = -330
	offset_right = -12
	offset_top = 64
	grow_horizontal = Control.GROW_DIRECTION_BEGIN
	add_theme_constant_override("separation", 4)
	mouse_filter = Control.MOUSE_FILTER_IGNORE


func set_alerts(new_alerts: Array) -> void:
	alerts = new_alerts
	for child in get_children():
		remove_child(child)
		child.queue_free()
	for index in mini(new_alerts.size(), MAX_ALERTS):
		add_child(_alert_button(new_alerts[index]))
	if new_alerts.size() > MAX_ALERTS:
		var more := Label.new()
		more.text = "… %d autres alertes" % (new_alerts.size() - MAX_ALERTS)
		more.add_theme_font_size_override("font_size", 13)
		more.horizontal_alignment = HORIZONTAL_ALIGNMENT_RIGHT
		add_child(more)


func _alert_button(alert: Dictionary) -> Button:
	var button := Button.new()
	button.text = "%s  %s" % [alert.get("glyph", "!"), alert.get("text", "")]
	button.alignment = HORIZONTAL_ALIGNMENT_LEFT
	button.text_overrun_behavior = TextServer.OVERRUN_TRIM_ELLIPSIS
	button.tooltip_text = str(alert.get("tooltip", alert.get("text", "")))
	button.custom_minimum_size = Vector2(318, 0)
	button.add_theme_font_size_override("font_size", 14)
	var color: Color = SEVERITY_COLORS.get(str(alert.get("severity", "info")), SEVERITY_COLORS["info"])
	button.add_theme_color_override("font_color", color)
	button.add_theme_color_override("font_hover_color", color.darkened(0.3))
	button.pressed.connect(func() -> void: alert_pressed.emit(alert))
	return button


## Alertes du joueur d'après l'état de la carte (`CampaignMap`) et les événements du dernier
## tour. Chaque alerte : `{id, kind, glyph, text, severity, province?, army?}`.
static func collect(map: Node, last_events: Array) -> Array:
	var result: Array = []
	var sim: Object = map.get("sim")
	var player := str(map.get("player_faction"))
	var map_data: MapData = map.get("map_data")
	if sim == null or player == "" or map_data == null:
		return result
	var summary: Dictionary = sim.call("get_faction_summary", player)
	var enemies: PackedStringArray = summary.get("at_war_with", PackedStringArray())
	var owned: Dictionary = {}  # province id → true (possédée et tenue par le joueur)
	for index in range(1, map_data.province_count + 1):
		var province_id := str(map_data.get_province(index).get("id", ""))
		var state: Dictionary = sim.call("get_province_state", province_id)
		if str(state.get("owner", "")) != player:
			continue
		owned[province_id] = true
		var siege: Dictionary = state.get("siege", {})
		if not siege.is_empty() and str(siege.get("attacker", "")) != player:
			result.append({
				"id": "siege:" + province_id, "kind": "siege", "glyph": "⚑", "severity": "danger",
				"province": province_id,
				"text": "%s assiégée (vivres : %d)" % [map.call("province_name_of", province_id), int(siege.get("supplies", 0))],
				"tooltip": "Siège mené par %s depuis %d tour(s)." % [_faction_name(str(siege.get("attacker", ""))), int(siege.get("turns_elapsed", 0))],
			})
	# Armées ennemies dans une province du joueur ou adjacente.
	for army_id in sim.call("get_army_ids"):
		var army: Dictionary = sim.call("get_army", army_id)
		var faction := str(army.get("faction", ""))
		if not (faction in enemies):
			continue
		var location := str(army.get("location", ""))
		var threatened := location if owned.has(location) else ""
		if threatened == "":
			for neighbor in map_data.get_province(map_data.index_of_id(location)).get("neighbors", []):
				if owned.has(str(neighbor)):
					threatened = str(neighbor)
					break
		if threatened == "":
			continue
		var strength := 0
		for unit in army.get("units", []):
			if unit is Dictionary:
				strength += int(unit.get("strength", 0))
		var where := "en %s" % map.call("province_name_of", location) if threatened == location else "aux portes de %s" % map.call("province_name_of", threatened)
		result.append({
			"id": "army:" + str(army_id), "kind": "enemy_army", "glyph": "⚔", "severity": "danger",
			"army": str(army_id), "province": location,
			"text": "Armée ennemie %s (%s)" % [where, _thousands(strength)],
			"tooltip": "Armée de %s, %s hommes." % [_faction_name(faction), _thousands(strength)],
		})
	if int(summary.get("treasury", 0)) < 0:
		result.append({
			"id": "debt", "kind": "debt", "glyph": "℔", "severity": "danger",
			"text": "Trésor endetté : %s ℔" % _thousands(int(summary.get("treasury", 0))),
			"tooltip": "En dette, les troupes perdent du moral : licenciez ou relevez l'impôt.",
		})
	if sim.has_method("get_research") and sim.has_method("get_tech_tree"):
		var research: Dictionary = sim.call("get_research", player)
		if research.is_empty() or str(research.get("technology", "")) == "":
			var available := false
			for node in sim.call("get_tech_tree", player):
				if str(node.get("state", "")) == "available":
					available = true
					break
			if available:
				result.append({
					"id": "research", "kind": "research", "glyph": "⚙", "severity": "warning",
					"text": "Aucune recherche en cours",
					"tooltip": "Ouvrir les technologies (T).",
				})
	if sim.has_method("get_pending_decisions"):
		var decisions: Array = sim.call("get_pending_decisions")
		if not decisions.is_empty():
			result.append({
				"id": "chronicle", "kind": "chronicle", "glyph": "§", "severity": "warning",
				"text": "Chronique : %d décision%s en attente" % [decisions.size(), "s" if decisions.size() > 1 else ""],
			})
	for event in last_events:
		if not (event is Dictionary) or str(event.get("kind", "")) != "building_completed":
			continue
		var province_id := str(event.get("province", ""))
		if province_id == "" or not owned.has(province_id):
			continue
		result.append({
			"id": "building:%s:%d" % [province_id, result.size()], "kind": "building", "glyph": "⚒", "severity": "info",
			"province": province_id, "text": str(event.get("text_fr", "Bâtiment terminé")),
		})
	return result


static func _faction_name(faction_id: String) -> String:
	var tree := Engine.get_main_loop() as SceneTree
	var facade: Node = tree.root.get_node_or_null("/root/SimFacade") if tree != null else null
	return str(facade.call("faction_short_name", faction_id)) if facade != null else faction_id


static func _thousands(value: int) -> String:
	var digits := str(absi(value))
	var groups := PackedStringArray()
	while digits.length() > 3:
		groups.insert(0, digits.substr(digits.length() - 3))
		digits = digits.substr(0, digits.length() - 3)
	groups.insert(0, digits)
	return ("-" if value < 0 else "") + " ".join(groups)
