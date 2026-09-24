class_name AgentController
extends Node

## Lot C6 : agents de campagne (espions, hérauts, prédicateurs) sur la carte. Rendu, UI et
## entrées seulement ; toute règle vient de `CampaignSim` (`get_agents`, `get_agent_actions`,
## `get_agent_reachable`, ordres `recruit_agent` / `move_agent` / `agent_action` /
## `dismiss_agent`) :
## - jetons ronds au style parchemin (sceau de cire, initiale du type, points de sceau) posés
##   à côté des colonies, couche 2D sous l'interface ; agents étrangers hors de vue masqués ;
## - clic gauche sur un jeton : sélection, anneaux des colonies atteignables
##   (`ReachableMarkers`, teinte or), barre d'actions avec pourcentage de réussite ;
## - clic droit sur une colonie, agent sélectionné : ordre `move_agent` ;
## - touche G : registre des agents (liste, recrutement dans la colonie ouverte ou la capitale).
## Inactif si la simulation n'expose pas `get_agents` (mock).

signal agent_selected(agent_id: String)

const TOKEN_SIZE := 30.0
const PICK_RADIUS_PX := 17.0
const LAYER := 0
const INK := Color(0.16, 0.10, 0.05)
const PARCHMENT := Color(0.95, 0.90, 0.77)
const GOLD := Color(0.86, 0.66, 0.18)
const KIND_LETTERS := {"spy": "E", "emissary": "H", "preacher": "P"}
const KIND_WAX := {"spy": Color(0.20, 0.22, 0.26), "emissary": Color(0.62, 0.12, 0.10), "preacher": Color(0.36, 0.20, 0.46)}
const KIND_LABELS := {"spy": "Espion", "emissary": "Héraut", "preacher": "Prédicateur"}

var map: Node = null  # CampaignMap
var selected_agent: String = ""
var markers: ReachableMarkers = null
var bar: PanelContainer = null
var registry: PanelContainer = null
var reachable: Dictionary = {}

var _layer: CanvasLayer
var _tokens: Dictionary = {}  # agent_id → Token
var _agents: Dictionary = {}  # agent_id → dict
var _bar_title: Label
var _bar_info: Label
var _bar_actions: HBoxContainer
var _bar_report: Label
var _registry_list: VBoxContainer
var _registry_recruit: VBoxContainer
var _registry_place: Label
var _last_distance := -1.0


## Jeton d'agent : disque parchemin cerclé aux couleurs de la faction, sceau de cire à
## l'initiale du type, points du sceau (niveau) en bas.
class Token:
	extends Control

	var agent_id := ""
	var kind := "spy"
	var level := 1
	var faction_color := Color.WHITE
	var selected := false
	var acted := false
	var world := Vector3.ZERO
	var slot := 0

	func _init() -> void:
		mouse_filter = Control.MOUSE_FILTER_IGNORE
		custom_minimum_size = Vector2(AgentController.TOKEN_SIZE, AgentController.TOKEN_SIZE)
		size = custom_minimum_size

	func _draw() -> void:
		var center := size * 0.5
		var radius := size.x * 0.5 - 1.0
		if selected:
			draw_circle(center, radius + 4.0, AgentController.GOLD)
		draw_circle(center, radius, faction_color.darkened(0.15))
		draw_circle(center, radius - 3.0, AgentController.PARCHMENT)
		var wax: Color = AgentController.KIND_WAX.get(kind, Color.DIM_GRAY)
		if acted:
			wax = wax.lerp(Color(0.5, 0.5, 0.5), 0.5)
		draw_circle(center, radius - 7.0, wax)
		var font := get_theme_default_font()
		var letter: String = AgentController.KIND_LETTERS.get(kind, "?")
		var font_size := 13
		var text_size := font.get_string_size(letter, HORIZONTAL_ALIGNMENT_CENTER, -1, font_size)
		draw_string(font, center + Vector2(-text_size.x * 0.5, text_size.y * 0.32), letter, HORIZONTAL_ALIGNMENT_CENTER, -1, font_size, AgentController.PARCHMENT)
		for i in 5:
			var angle := PI * (0.62 + 0.19 * i)
			var dot := center + Vector2(cos(angle), sin(angle)) * (radius - 1.5) * Vector2(-1, 1)
			draw_circle(dot, 1.8, AgentController.GOLD if i < level else AgentController.INK.lerp(AgentController.PARCHMENT, 0.5))


func setup(campaign_map: Node) -> void:
	map = campaign_map
	name = "AgentController"
	_layer = CanvasLayer.new()
	_layer.name = "AgentTokens"
	_layer.layer = LAYER
	add_child(_layer)
	markers = ReachableMarkers.new()
	markers.name = "AgentReachableMarkers"
	markers.color = Color(0.95, 0.78, 0.25, 0.95)
	markers.target_color = Color(0.55, 0.25, 0.75, 1.0)
	map.add_child(markers)
	_build_bar()
	_build_registry()
	if map.picker != null:
		var previous_click: Callable = map.picker.click_interceptor
		map.picker.click_interceptor = func(screen_position: Vector2) -> bool:
			if _try_pick(screen_position):
				return true
			return previous_click.call(screen_position) if previous_click.is_valid() else false
		var previous_right: Callable = map.picker.right_click_interceptor
		map.picker.right_click_interceptor = func(screen_position: Vector2) -> bool:
			if _try_right_click(screen_position):
				return true
			return previous_right.call(screen_position) if previous_right.is_valid() else false


## Vrai si la simulation expose l'API des agents (vraie `CampaignSim`).
func available() -> bool:
	var sim: Object = map.sim if map != null else null
	return sim != null and sim.has_method("get_agents") and map.settlement_layer != null


func token_count() -> int:
	return _tokens.size()


func has_token(agent_id: String) -> bool:
	return _tokens.has(agent_id)


# --- Jetons -------------------------------------------------------------------------------


## Reconstruit les jetons depuis la simulation (après chaque ordre ou fin de tour).
func refresh() -> void:
	for token in _tokens.values():
		token.queue_free()
	_tokens.clear()
	_agents.clear()
	if not available():
		return
	var hidden: Dictionary = map.armies.hidden_provinces if map.armies != null else {}
	var per_place: Dictionary = {}
	for entry in map.sim.call("get_agents"):
		var agent: Dictionary = entry
		var id := str(agent.get("id", ""))
		var faction := str(agent.get("faction", ""))
		if faction != map.player_faction and hidden.has(str(agent.get("province", ""))):
			continue
		var location := str(agent.get("location", ""))
		var world: Vector3 = map.settlement_layer.world_position_of(location)
		if world == Vector3.ZERO:
			continue
		_agents[id] = agent
		var token := Token.new()
		token.name = "Agent_" + id
		token.agent_id = id
		token.kind = str(agent.get("kind", "spy"))
		token.level = int(agent.get("level", 1))
		token.acted = bool(agent.get("acted", false)) or int(agent.get("movement_points", 0)) == 0
		token.faction_color = SimFacade.faction_color(faction)
		token.selected = id == selected_agent
		token.world = world
		token.slot = int(per_place.get(location, 0))
		token.tooltip_text = "%s — %s" % [agent.get("kind_name", ""), agent.get("name", "")]
		per_place[location] = token.slot + 1
		_layer.add_child(token)
		_tokens[id] = token
	_place_tokens()
	if selected_agent != "":
		if _agents.has(selected_agent) and str(_agents[selected_agent].get("faction", "")) == map.player_faction:
			select_agent(selected_agent, false)
		else:
			deselect()
	if registry.visible:
		_fill_registry()


func _place_tokens() -> void:
	var camera: Camera3D = map.camera
	if camera == null:
		return
	for token in _tokens.values():
		if camera.is_position_behind(token.world):
			token.visible = false
			continue
		var screen := camera.unproject_position(token.world)
		token.visible = true
		# À droite de la colonie (l'armée occupe son centre), en rangée.
		token.position = screen + Vector2(16.0 + token.slot * (TOKEN_SIZE + 2.0), -TOKEN_SIZE - 4.0)


func _process(_delta: float) -> void:
	if map == null or _tokens.is_empty() and not markers.visible:
		return
	_place_tokens()
	var distance: float = map.camera_rig.distance
	if markers.visible and not is_equal_approx(distance, _last_distance):
		_last_distance = distance
		markers.update_scale(distance)


func _try_pick(screen_position: Vector2) -> bool:
	var best := ""
	var best_distance := PICK_RADIUS_PX
	for id in _tokens:
		var token: Token = _tokens[id]
		if not token.visible:
			continue
		var distance := (token.position + token.size * 0.5).distance_to(screen_position)
		if distance < best_distance:
			best_distance = distance
			best = id
	if best == "":
		return false
	select_agent(best)
	return true


# --- Sélection et déplacement ------------------------------------------------------------


func select_agent(agent_id: String, from_click: bool = true) -> void:
	if not available():
		return
	var agent: Dictionary = map.sim.call("get_agent", agent_id)
	if agent.is_empty():
		deselect()
		return
	if from_click and map.selected_army != "":
		map.deselect_army()
	selected_agent = agent_id
	for id in _tokens:
		_tokens[id].selected = id == agent_id
		_tokens[id].queue_redraw()
	var is_player: bool = str(agent.get("faction", "")) == str(map.player_faction)
	reachable = map.sim.call("get_agent_reachable", agent_id) if is_player else {}
	var ids := PackedStringArray()
	var positions := PackedVector3Array()
	for id in reachable:
		var world: Vector3 = map.settlement_layer.world_position_of(str(id))
		if world != Vector3.ZERO:
			ids.append(str(id))
			positions.append(world)
	markers.show_markers(ids, positions, map.camera_rig.distance)
	if is_player:
		_show_bar(agent)
	else:
		bar.hide()
		map.ui.show_toast("%s — %s (%s)" % [agent.get("kind_name", ""), agent.get("name", ""), SimFacade.faction_short_name(str(agent.get("faction", "")))])
	agent_selected.emit(agent_id)


func deselect() -> void:
	selected_agent = ""
	reachable = {}
	markers.clear()
	if bar != null:
		bar.hide()
	for token in _tokens.values():
		token.selected = false
		token.queue_redraw()


func _try_right_click(screen_position: Vector2) -> bool:
	if not available() or selected_agent == "":
		return false
	var id: String = map.settlement_layer.pick_screen(screen_position)
	if id == "":
		return false
	var result := order_move(selected_agent, id)
	if not result.get("ok", false):
		map.ui.show_toast(str(result.get("error", "Ordre refusé")), true)
	return true


## Ordre `move_agent` : l'agent marche aussitôt et garde sa destination.
func order_move(agent_id: String, settlement_id: String) -> Dictionary:
	var result: Dictionary = map.sim.call("submit_order", {"type": "move_agent", "agent": agent_id, "target": settlement_id})
	if result.get("ok", false):
		map.refresh_all()
	return result


func _unhandled_input(event: InputEvent) -> void:
	if not available():
		return
	if event is InputEventKey and event.pressed and not event.echo and event.physical_keycode == KEY_G:
		toggle_registry()
		get_viewport().set_input_as_handled()
	elif event.is_action_pressed("ui_cancel") and selected_agent != "":
		deselect()
		get_viewport().set_input_as_handled()


# --- Barre d'actions ----------------------------------------------------------------------


func _panel_style() -> StyleBoxFlat:
	var style := StyleBoxFlat.new()
	style.bg_color = Color(0.94, 0.89, 0.76, 0.97)
	style.border_color = Color(0.45, 0.30, 0.14)
	style.set_border_width_all(2)
	style.set_corner_radius_all(6)
	style.content_margin_left = 12
	style.content_margin_right = 12
	style.content_margin_top = 8
	style.content_margin_bottom = 8
	return style


func _build_bar() -> void:
	bar = PanelContainer.new()
	bar.name = "AgentBar"
	bar.theme = load("res://scenes/ui/parchment_theme.tres")
	bar.add_theme_stylebox_override("panel", _panel_style())
	bar.set_anchors_and_offsets_preset(Control.PRESET_CENTER_BOTTOM)
	bar.grow_horizontal = Control.GROW_DIRECTION_BOTH
	bar.grow_vertical = Control.GROW_DIRECTION_BEGIN
	bar.offset_bottom = -150
	var box := VBoxContainer.new()
	box.add_theme_constant_override("separation", 4)
	bar.add_child(box)
	var head := HBoxContainer.new()
	box.add_child(head)
	_bar_title = Label.new()
	_bar_title.add_theme_font_size_override("font_size", 18)
	_bar_title.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	head.add_child(_bar_title)
	var dismiss := Button.new()
	dismiss.text = "Renvoyer"
	dismiss.tooltip_text = "Congédier cet agent (plus d'entretien)."
	dismiss.pressed.connect(func() -> void: dismiss_selected())
	head.add_child(dismiss)
	var close := Button.new()
	close.text = "✕"
	close.pressed.connect(deselect)
	head.add_child(close)
	_bar_info = Label.new()
	_bar_info.add_theme_color_override("font_color", Color(0.35, 0.26, 0.15))
	box.add_child(_bar_info)
	_bar_actions = HBoxContainer.new()
	_bar_actions.add_theme_constant_override("separation", 6)
	box.add_child(_bar_actions)
	_bar_report = Label.new()
	_bar_report.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	_bar_report.custom_minimum_size = Vector2(560, 0)
	_bar_report.add_theme_font_size_override("font_size", 13)
	box.add_child(_bar_report)
	bar.hide()
	map.ui.add_child(bar)


func _seal_text(level: int) -> String:
	return "●".repeat(level) + "○".repeat(maxi(0, 5 - level))


func _show_bar(agent: Dictionary) -> void:
	_bar_title.text = "%s — %s   %s" % [agent.get("kind_name", ""), agent.get("name", ""), _seal_text(int(agent.get("level", 1)))]
	var info := "À %s · marche %d/%d · expérience %d" % [
		agent.get("location_name", ""), int(agent.get("movement_points", 0)),
		int(agent.get("max_movement_points", 0)), int(agent.get("experience", 0))]
	if str(agent.get("destination", "")) != "":
		info += " · en route vers %s" % map.settlements_ctl.settlement_name(str(agent["destination"])) if map.settlements_ctl != null else ""
	if bool(agent.get("acted", false)):
		info += " · a déjà agi cette saison"
	_bar_info.text = info
	for child in _bar_actions.get_children():
		child.queue_free()
	for option in map.sim.call("get_agent_actions", str(agent.get("id", ""))):
		_bar_actions.add_child(_action_button(option))
	var report: Dictionary = agent.get("last_report", {})
	_bar_report.text = str(report.get("text", "")) if not report.is_empty() and int(report.get("turn", -1)) == int(map.sim.call("get_turn")) else ""
	_bar_report.visible = _bar_report.text != ""
	bar.show()
	bar.reset_size()


func _action_button(option: Dictionary) -> Button:
	var button := Button.new()
	var available_now := bool(option.get("available", false))
	button.name = "Action_" + str(option.get("action", ""))
	button.custom_minimum_size = Vector2(118, 46)
	button.text = "%s\n%s" % [option.get("name", ""), "%d %%" % int(option.get("chance", 0)) if available_now else "—"]
	button.disabled = not available_now
	var tip := PackedStringArray([str(option.get("description", ""))])
	if available_now:
		tip.append("Cible : %s" % option.get("target_name", ""))
		tip.append("Réussite : %d %%" % int(option.get("chance", 0)))
		if int(option.get("cost", 0)) > 0:
			tip.append("Coût : %s" % Money.amount(int(option.get("cost", 0))))
		if int(option.get("death_risk", 0)) > 0:
			tip.append("Risque de perdre l'agent en cas d'échec : %d %%" % int(option.get("death_risk", 0)))
	else:
		tip.append("Indisponible : %s" % option.get("reason", ""))
	button.tooltip_text = "\n".join(tip)
	button.pressed.connect(func() -> void: perform_action(str(option.get("action", ""))))
	return button


## Tente l'action `action` de l'agent sélectionné sur la meilleure cible proposée.
## Renvoie le rapport (`get_last_agent_report`) ou `{error}`.
func perform_action(action: String) -> Dictionary:
	if selected_agent == "":
		return {"error": "aucun agent sélectionné"}
	var agent_id := selected_agent
	var chosen: Dictionary = {}
	for option in map.sim.call("get_agent_actions", agent_id):
		if str(option.get("action", "")) == action:
			chosen = option
	if chosen.is_empty() or not bool(chosen.get("available", false)):
		var reason := str(chosen.get("reason", "action indisponible"))
		map.ui.show_toast(reason, true)
		return {"error": reason}
	var order := {"type": "agent_action", "agent": agent_id, "action": action}
	if str(chosen.get("target", "")) != "":
		order["target"] = str(chosen["target"])
	if str(chosen.get("character", "")) != "":
		order["character"] = str(chosen["character"])
	var result: Dictionary = map.sim.call("submit_order", order)
	if not result.get("ok", false):
		map.ui.show_toast(str(result.get("error", "Ordre refusé")), true)
		return {"error": str(result.get("error", ""))}
	var report: Dictionary = map.sim.call("get_last_agent_report")
	map.ui.show_toast(str(report.get("text", "")), not bool(report.get("success", false)))
	if bool(report.get("lost", false)):
		selected_agent = ""
	map.refresh_all()
	return report


func dismiss_selected() -> void:
	if selected_agent == "":
		return
	var result: Dictionary = map.sim.call("submit_order", {"type": "dismiss_agent", "agent": selected_agent})
	if result.get("ok", false):
		map.ui.show_toast("Agent congédié.")
		deselect()
		map.refresh_all()


func action_button_count() -> int:
	return _bar_actions.get_child_count() if _bar_actions != null else 0


# --- Registre (touche G) ------------------------------------------------------------------


func _build_registry() -> void:
	registry = PanelContainer.new()
	registry.name = "AgentRegistry"
	registry.theme = load("res://scenes/ui/parchment_theme.tres")
	registry.add_theme_stylebox_override("panel", _panel_style())
	registry.set_anchors_and_offsets_preset(Control.PRESET_TOP_LEFT)
	registry.position = Vector2(20, 90)
	registry.custom_minimum_size = Vector2(360, 0)
	var box := VBoxContainer.new()
	box.add_theme_constant_override("separation", 6)
	registry.add_child(box)
	var head := HBoxContainer.new()
	box.add_child(head)
	var title := Label.new()
	title.text = "Agents (G)"
	title.add_theme_font_size_override("font_size", 20)
	title.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	head.add_child(title)
	var close := Button.new()
	close.text = "✕"
	close.pressed.connect(func() -> void: registry.hide())
	head.add_child(close)
	_registry_list = VBoxContainer.new()
	box.add_child(_registry_list)
	box.add_child(HSeparator.new())
	_registry_place = Label.new()
	_registry_place.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	box.add_child(_registry_place)
	_registry_recruit = VBoxContainer.new()
	box.add_child(_registry_recruit)
	registry.hide()
	map.ui.add_child(registry)


func toggle_registry() -> void:
	if registry.visible:
		registry.hide()
		return
	_fill_registry()
	registry.show()
	registry.size = Vector2.ZERO
	registry.call_deferred("reset_size")


## Colonie où recruter : celle du panneau de colonie ouvert (si elle est au joueur), sinon la
## cité de la capitale.
func recruit_place() -> String:
	var ctl: Node = map.settlements_ctl
	if ctl != null and ctl.panel != null and ctl.panel.visible:
		var detail: Dictionary = map.sim.call("settlement_detail", ctl.panel.settlement_id)
		if str(detail.get("controller", "")) == map.player_faction:
			return str(detail.get("id", ""))
	var capital := str(GameCatalog.definitions("factions").get(map.player_faction, {}).get("capital", ""))
	var province: Dictionary = map.sim.call("get_province_state", capital) if capital != "" else {}
	return str(province.get("city", ""))


func _fill_registry() -> void:
	for child in _registry_list.get_children():
		child.queue_free()
	for child in _registry_recruit.get_children():
		child.queue_free()
	var mine := 0
	for entry in map.sim.call("get_agents"):
		if str(entry.get("faction", "")) != map.player_faction:
			continue
		mine += 1
		var row := Button.new()
		row.alignment = HORIZONTAL_ALIGNMENT_LEFT
		row.text = "%s %s — %s (%s)" % [_seal_text(int(entry.get("level", 1))), entry.get("name", ""), entry.get("kind_name", ""), entry.get("location_name", "")]
		var id := str(entry.get("id", ""))
		var location := str(entry.get("location", ""))
		row.pressed.connect(func() -> void:
			select_agent(id)
			var world: Vector3 = map.settlement_layer.world_position_of(location)
			if world != Vector3.ZERO:
				map.camera_rig.look_at_point(world, minf(map.camera_rig.distance, 220.0)))
		_registry_list.add_child(row)
	if mine == 0:
		var none := Label.new()
		none.text = "Aucun agent à votre service."
		_registry_list.add_child(none)
	var place := recruit_place()
	var place_name: String = map.settlements_ctl.settlement_name(place) if map.settlements_ctl != null else place
	_registry_place.text = "Recruter à %s (ouvrez une colonie pour en choisir une autre) :" % place_name
	for option in map.sim.call("get_agent_recruit_options", place):
		var button := Button.new()
		button.alignment = HORIZONTAL_ALIGNMENT_LEFT
		button.text = "%s — %s (entretien %s) · %d/%d" % [option.get("name", ""), Money.amount(int(option.get("cost", 0))), Money.amount(int(option.get("upkeep", 0))), int(option.get("count", 0)), int(option.get("max", 0))]
		button.disabled = not bool(option.get("available", false))
		button.tooltip_text = str(option.get("reason", "")) if button.disabled else "Recruter un %s à %s." % [str(option.get("name", "")).to_lower(), place_name]
		var kind := str(option.get("kind", ""))
		button.pressed.connect(func() -> void: recruit(place, kind))
		_registry_recruit.add_child(button)
	registry.call_deferred("reset_size")


## Ordre `recruit_agent` ; l'agent apparaît aussitôt (sans marche ce tour-ci).
func recruit(settlement_id: String, kind: String) -> Dictionary:
	var result: Dictionary = map.sim.call("submit_order", {"type": "recruit_agent", "settlement": settlement_id, "kind": kind})
	if result.get("ok", false):
		map.ui.show_toast("%s recruté : il se mettra en route à la prochaine saison." % KIND_LABELS.get(kind, "Agent"))
		map.refresh_all()
		if registry.visible:
			_fill_registry()
	else:
		map.ui.show_toast(str(result.get("error", "Recrutement refusé")), true)
	return result


# --- Capture (`--stage=agents`) -----------------------------------------------------------


## Recrute un espion dans la capitale, lui donne une saison, le sélectionne et cadre la carte.
func stage_screenshot(open_registry: bool = false) -> void:
	if not available():
		return
	var place := recruit_place()
	for kind in ["spy", "emissary", "preacher"]:
		recruit(place, kind)
	map.sim.call("end_turn")
	map.refresh_all()
	var spy := ""
	for entry in map.sim.call("get_agents"):
		if str(entry.get("faction", "")) == map.player_faction and str(entry.get("kind", "")) == "spy":
			spy = str(entry.get("id", ""))
	if spy == "":
		return
	select_agent(spy)
	var world: Vector3 = map.settlement_layer.world_position_of(str(_agents.get(spy, {}).get("location", "")))
	map.camera_rig.look_at_point(world, 260.0)
	map.camera_rig.snap()
	if open_registry:
		toggle_registry()
