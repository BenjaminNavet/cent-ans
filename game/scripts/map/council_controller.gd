class_name CouncilController
extends Node

## Conseil du royaume sur la carte (RG13c, ADR 0341) : bouton « Conseil » de la barre, touche J
## (K est prise par le Codex), pastille d'alerte `council_vacancy`. Instancie `CouncilChamber`,
## l'enregistre comme panneau central et lui fournit les conseils déjà calculés (alertes de la
## cloche et indice « que faire maintenant »). Aucune règle ici.

const SCENE := preload("res://scenes/ui/council_chamber.tscn")
const ACTION := "map_toggle_council"

var map: Node = null  # CampaignMap
var panel: CouncilChamber = null
var button: Button = null


func setup(campaign_map: Node) -> void:
	map = campaign_map
	var ui: MapUI = map.get("ui")
	panel = SCENE.instantiate() as CouncilChamber
	panel.name = "CouncilChamber"
	panel.advice_provider = Callable(self, "collect_advice")
	panel.order_submitter = Callable(self, "_submit")
	panel.advice_activated.connect(_on_advice_activated)
	panel.closed.connect(func() -> void: panel.hide())
	ui.add_child(panel)
	panel.hide()
	ui.register_panel(panel, PanelStack.Kind.CENTRAL)
	button = ui.call("_add_action_button", "CouncilButton", "⚜", "Conseil", ACTION,
		"[b]Conseil[/b]\nVos conseillers, leurs charges et leurs conseils ; nommer ou révoquer.",
		ui.tech_button.get_index() + 1)
	ui.top_fit.register_label(button, "Conseil", "⚜")


func is_open() -> bool:
	return panel != null and panel.visible


func open_panel() -> void:
	var faction := str(map.get("player_faction"))
	var sim: Object = map.get("sim")
	var label := faction
	var color := Color(0.6, 0.15, 0.15)
	if sim != null:
		var summary: Dictionary = sim.call("get_faction_summary", faction)
		label = str(summary.get("name", faction))
		var raw: Variant = summary.get("color", null)
		if raw is Color:
			color = raw
		elif raw is String and str(raw) != "":
			color = Color.html(str(raw))
	panel.open_for(faction, label, color)


func close_panel() -> void:
	if is_open():
		panel.hide()


func toggle() -> void:
	if is_open():
		close_panel()
	else:
		open_panel()


func _unhandled_input(event: InputEvent) -> void:
	if event.is_action_pressed(ACTION) and not event.is_echo():
		toggle()
		get_viewport().set_input_as_handled()


## Alertes de la cloche + indice courant (converti en élément de conseil).
func collect_advice() -> Array:
	var items: Array = []
	var hud: Node = map.get("hud")
	items.append_array(CampaignAlerts.collect(map, hud.get("last_events") if hud != null else []))
	var next_hint: Node = map.get("next_hint")
	if next_hint != null and next_hint.has_method("gather_state"):
		var hint: Dictionary = NextHint.choose(next_hint.call("gather_state"))
		if not hint.is_empty():
			var alert: Dictionary = hint.get("alert", {})
			items.append({"id": "hint:%s" % hint.get("id", ""), "kind": str(alert.get("kind", hint.get("id", ""))),
				"severity": "info", "text": str(hint.get("text", hint.get("title", ""))), "hint": hint})
	return items


func _on_advice_activated(item: Dictionary) -> void:
	var hint: Dictionary = item.get("hint", {})
	var next_hint: Node = map.get("next_hint")
	if not hint.is_empty() and next_hint != null:
		next_hint.call("activate", hint)
		return
	var hud: Node = map.get("hud")
	if hud != null:
		hud.call("_on_alert_activated", item)


func _submit(order: Dictionary) -> Dictionary:
	if map.has_method("_submit"):
		var text := "Conseiller nommé." if str(order.get("type", "")) == "appoint_councillor" else "Conseiller révoqué."
		return map.call("_submit", order, text)
	return {"ok": false, "error": "Carte absente"}
