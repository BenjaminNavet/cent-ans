class_name EconomyController
extends Node

## RG12 : fenêtre Économie sur la carte (ADR 0346) : bouton « Économie » de la barre, touche M
## (`map_toggle_economy`), ouverture directe sur l'onglet Commerce depuis la touche `I`
## (`campaign_commerce`), le bouton « Commerce → Économie » de la diplomatie, le panneau de
## faction et l'avis « commerce ». Instancie `EconomyWindow`, l'enregistre comme panneau central
## et transmet ses ordres au cœur. Aucune règle ici.

const ACTION := "map_toggle_economy"

var map: Node = null  # CampaignMap
var panel: EconomyWindow = null
var button: Button = null


func setup(campaign_map: Node) -> void:
	map = campaign_map
	var ui: MapUI = map.get("ui")
	panel = EconomyWindow.new()
	panel.order_requested.connect(_on_order_requested)
	panel.trade_layer_requested.connect(show_trade_layer)
	ui.add_child(panel)
	panel.hide()
	ui.register_panel(panel, PanelStack.Kind.CENTRAL)
	button = ui.call("_add_action_button", "EconomyButton", "₶", "Économie", ACTION,
		"[b]Économie[/b]\nRecettes et dépenses, prévisions, accords commerciaux, routes et monnaie.",
		ui.tech_button.get_index() + 2)
	ui.top_fit.register_label(button, "Économie", "₶")
	if ui.faction_panel != null and ui.faction_panel.has_signal("economy_requested"):
		ui.faction_panel.economy_requested.connect(open_panel)


func is_open() -> bool:
	return panel != null and panel.visible


## Ouvre la fenêtre (sur `tab` : `EconomyWindow.TAB_*`, "" = l'onglet courant).
func open_panel(tab: String = "") -> void:
	if panel == null:
		return
	panel.setup(map.get("sim"), str(map.get("player_faction")))
	panel.open(tab)


func close_panel() -> void:
	if is_open():
		panel.close()


func toggle(tab: String = "") -> void:
	if is_open() and (tab == "" or panel.current_tab() == tab):
		close_panel()
	else:
		open_panel(tab)


## À rappeler après chaque changement d'état (tour, ordre) : relit l'onglet visible.
func refresh() -> void:
	if is_open():
		panel.refresh()


## « Voir sur la carte » : active le calque des routes commerciales et laisse la carte à l'écran.
func show_trade_layer() -> void:
	if not bool(map.get("trade_mode")):
		map.call("_toggle_trade_layer")
	close_panel()


func _unhandled_input(event: InputEvent) -> void:
	if event.is_action_pressed(ACTION) and not event.is_echo():
		toggle()
		get_viewport().set_input_as_handled()


func _on_order_requested(order: Dictionary, success_text: String) -> void:
	var sim: Object = map.get("sim")
	var result: Dictionary = sim.call("submit_order", order)
	var ui: MapUI = map.get("ui")
	if result.get("ok", false):
		ui.show_toast(success_text)
	else:
		ui.show_toast(str(result.get("error", "Ordre refusé")), true)
	map.call("refresh_all")  # relit aussi la fenêtre (`refresh`)
