class_name NextHintController
extends Node

## Lot UX2 — conseil « que faire maintenant » de la carte de campagne. Rassemble l'état déjà
## exposé (alertes calculées pour la cloche, armées du joueur, trésor, chantiers, guide mis de
## côté), choisit un seul conseil (`NextHint.choose`, textes dans `data/ui/next_hints.json`) et
## l'affiche dans `NextHintCard`. Un clic exécute ou ouvre l'action. Masqué pendant le
## tutoriel, sous les panneaux et fenêtres, et quand le réglage `interface/next_hint` est
## décoché ; la croix masque le conseil courant jusqu'à la saison suivante.
## Créé par `campaign_map.gd` (`setup`, `stage_screenshot`). Aucune règle de jeu.

const SETTING := "interface/next_hint"
const REFRESH_SECONDS := 1.0
## Chantiers recomptés au plus toutes les tant de secondes (parcours des provinces).
const CONSTRUCTION_SECONDS := 4.0

var map: Node = null  # CampaignMap
var card: NextHintCard = null
var settings: Node = null
## Identifiants masqués par la croix, valables jusqu'au changement de saison.
var dismissed: Array = []
var _dismissed_turn := -1
var _timer := 0.0
var _construction_timer := 0.0
var _constructions := 0
## Faux en capture d'écran (sauf `--stage=next_hint`) : les captures des autres lots ne changent pas.
var enabled := true


func setup(campaign_map: Node) -> void:
	map = campaign_map
	settings = get_node_or_null("/root/Settings")
	card = NextHintCard.new()
	var ui: Node = map.get("ui")
	ui.add_child(card)
	card.activated.connect(activate)
	card.dismissed.connect(dismiss)
	enabled = not TutorialController.capture_mode()
	if settings != null:
		settings.changed.connect(func(key: String) -> void:
			if key == SETTING:
				refresh())


func _setting_on() -> bool:
	return settings == null or bool(settings.call("get_value", SETTING))


func _process(delta: float) -> void:
	_timer -= delta
	_construction_timer -= delta
	if _timer > 0.0:
		return
	_timer = REFRESH_SECONDS
	refresh()


## Recalcule le conseil et sa visibilité.
func refresh() -> void:
	if card == null or map == null:
		return
	if not enabled or not _setting_on() or map.get("sim") == null or covered():
		card.hide()
		return
	var hint := NextHint.choose(gather_state())
	card.set_hint(hint)
	if hint.is_empty():
		card.hide()
		return
	_place()
	card.show()


## Vrai quand le conseil doit s'effacer : tutoriel actif, panneau ou fenêtre ouverts.
func covered() -> bool:
	if not bool(map.get("visible")):
		return true
	var tutorial: TutorialController = map.get("tutorial")
	if tutorial != null and (tutorial.active or tutorial.modal_open()):
		return true
	var ui: MapUI = map.get("ui")
	if ui == null or not ui.panels.visible_panels().is_empty() or ui.province_panel.visible or ui.is_dialog_open():
		return true
	for panel: Control in ui.docked_panels:
		if panel.visible:
			return true
	var flow: Node = map.get("flow")
	var report: Control = flow.get("season_report") if flow != null else null
	if report != null and report.visible:
		return true
	return ui.turn_banner != null and ui.turn_banner.visible


## Haut gauche, sous la barre (la minicarte et les lettres occupent la droite).
func _place() -> void:
	var ui: MapUI = map.get("ui")
	var top: float = (ui.get_node("TopBar") as Control).size.y + 8.0
	card.reset_size()
	card.position = Vector2(MapUI.HUD_MARGIN, top)


# --- État ------------------------------------------------------------------------------


func _sim() -> Object:
	return map.get("sim")


func _player() -> String:
	return str(map.get("player_faction"))


## État lu pour `NextHint.choose` (voir l'en-tête de `NextHint`).
func gather_state() -> Dictionary:
	var sim := _sim()
	var turn := int(sim.call("get_turn"))
	if turn != _dismissed_turn:
		dismissed.clear()
		_dismissed_turn = turn
	var ui: MapUI = map.get("ui")
	var alerts: Array = []
	for alert: Dictionary in ui.end_turn_cluster.alerts:
		var entry := alert.duplicate()
		var province_id := str(alert.get("province_id", ""))
		if province_id != "":
			entry["province_name"] = str(map.call("province_name_of", province_id))
		alerts.append(entry)
	var summary: Dictionary = sim.call("get_faction_summary", _player())
	var state := {
		"alerts": alerts,
		"treasury": int(summary.get("treasury", 0)),
		"income": int(summary.get("income", summary.get("net_income", 0))),
		"can_build": sim.has_method("get_province_city"),
		"dismissed": dismissed,
		"tutorial_step": -1,
	}
	var idle := idle_army()
	state["idle_army"] = idle
	state["idle_army_name"] = _army_name(idle)
	var tutorial: TutorialController = map.get("tutorial")
	if tutorial != null:
		state["tutorial_step"] = tutorial.postponed_step()
	if bool(state["can_build"]) and NextHint.idle_treasury(state, NextHint.data()):
		# Les chantiers ne se comptent que si le trésor dépasse le seuil (parcours coûteux).
		if _construction_timer <= 0.0 and tutorial != null:
			_constructions = tutorial._constructions()
			_construction_timer = CONSTRUCTION_SECONDS
		state["constructions"] = _constructions
	return state


## Première armée du joueur sans ordre (l'armée royale d'abord), ou `""`.
func idle_army() -> String:
	var sim := _sim()
	var ids: Array = Array(map.call("player_army_ids"))
	var tutorial: TutorialController = map.get("tutorial")
	var royal := tutorial.find_royal_army() if tutorial != null and not ids.is_empty() else ""
	if royal in ids:
		ids.erase(royal)
		ids.push_front(royal)
	for army_id in ids:
		if NextHint.army_is_idle(sim.call("get_army", army_id)):
			return str(army_id)
	return ""


func _army_name(army_id: String) -> String:
	if army_id == "":
		return ""
	var general := str((_sim().call("get_army", army_id) as Dictionary).get("general_name", ""))
	return "l'ost de %s" % general if general != "" else "votre armée"


# --- Actions ------------------------------------------------------------------------------


## Clic sur le conseil : même chemin que la pastille de la cloche, ou action directe.
func activate(hint: Dictionary) -> void:
	var ui: MapUI = map.get("ui")
	match str(hint.get("action", "")):
		"alert":
			ui.alert_activated.emit(hint.get("alert", {}))
		"select_army":
			var army_id := str(hint.get("army_id", ""))
			if army_id != "":
				map.call("select_army", army_id)
				_look_at(map.get("armies").call("world_position_of", army_id))
		"open_city":
			_open_capital_city()
		"resume_tutorial":
			var tutorial: TutorialController = map.get("tutorial")
			if tutorial != null:
				tutorial.resume()
		"end_turn":
			ui.request_end_turn()
	card.hide()
	_timer = 0.3


## Croix : conseil masqué jusqu'à la saison suivante (le suivant par ordre de priorité apparaît).
func dismiss(hint: Dictionary) -> void:
	var id := str(hint.get("id", ""))
	if id != "" and not (id in dismissed):
		dismissed.append(id)
	refresh()


func _look_at(world: Vector3) -> void:
	var rig: CampaignCamera = map.get("camera_rig")
	var map_data: MapData = map.get("map_data")
	if rig != null and map_data != null and world != Vector3.ZERO:
		rig.look_at_point(world, maxf(map_data.size.x, map_data.size.y) * 0.12)


## Ville de la capitale : caméra dessus et panneau de la colonie (onglets Bâtiments…).
func _open_capital_city() -> void:
	var capital := str(GameCatalog.definitions("factions").get(_player(), {}).get("capital", ""))
	var state: Dictionary = _sim().call("get_province_state", capital) if capital != "" else {}
	var city := str(state.get("city", ""))
	var layer: Node = map.get("settlement_layer")
	if city != "" and layer != null:
		_look_at(layer.call("world_position_of", city))
		layer.call("select", city)
	elif capital != "":
		map.call("_focus_capital")


# --- Captures -----------------------------------------------------------------------------


## `--stage=next_hint` : début de partie, conseil affiché (sans tutoriel).
func stage_screenshot() -> void:
	enabled = true
	var tutorial: TutorialController = map.get("tutorial")
	if tutorial != null and tutorial.active:
		tutorial.postpone(false)
	map.call("_focus_first_player_army")
	refresh()
