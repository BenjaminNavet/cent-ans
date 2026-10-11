class_name EconomyTradeTab
extends VBoxContainer

## RG12 : onglet « Commerce » de la fenêtre Économie (le contenu de l'ancien accès commerce de la
## diplomatie y est déplacé). Accords en cours avec leur gain par saison, nouvelles routes
## (partenaires possibles avec la projection avant signature, triés par gain), routes actives
## du joueur, biens produits, et « Voir sur la carte » (calque des routes). Aucune règle ici :
## `get_trade_partner_options`, `get_trade_overview`, `get_faction_economy`, et les ordres
## `propose_treaty` / `break_trade_agreement` partent par le signal `order_requested`.
##
## Hors champ (ADR 0346) : le cœur ne tient aucun stock de ressources, donc pas d'échange
## acheter/vendre contre de l'or.

signal order_requested(order: Dictionary, success_text: String)
signal show_on_map_requested

const MAX_ROUTES_SHOWN := 14
const MAX_PARTNERS_SHOWN := 12
const MUTED := Color(0.42, 0.33, 0.20)

var summary_label: Label
var map_button: Button
var agreements_box: VBoxContainer
var partners_box: VBoxContainer
var routes_box: VBoxContainer
var goods_box: HFlowContainer
## Données affichées (tests) : listes de lignes `{id, text}`.
var agreement_rows: Array = []
var partner_rows: Array = []
var route_rows: Array = []
var goods_shown: Array = []
## Boutons par partenaire (tests) : `{"propose": {id: Button}, "break": {id: Button}}`.
var buttons: Dictionary = {"propose": {}, "break": {}}
var _faction := ""
var _break_armed := ""


func _init() -> void:
	name = "Trade"
	add_theme_constant_override("separation", 8)
	var head := UiBuild.hbox(8)
	add_child(head)
	summary_label = UiBuild.label("", UiType.size(UiType.BODY), null, true)
	summary_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	head.add_child(summary_label)
	map_button = UiBuild.button("Voir sur la carte", func() -> void: show_on_map_requested.emit())
	map_button.name = "ShowOnMapButton"
	TooltipHost.attach_plain(map_button, "map_mode_trade_overlay")
	head.add_child(map_button)
	agreements_box = _section("Accords en cours")
	partners_box = _section("Nouvelles routes")
	routes_box = _section("Routes commerciales actives")
	_section("Échanges de ressources")
	var note := UiBuild.label("Les biens voyagent avec les routes ci-dessus ; le royaume ne tient pas de stocks, donc pas d'achat ni de vente directs de ressources.", UiType.size(UiType.CAPTION), MUTED, true)
	add_child(note)
	goods_box = HFlowContainer.new()
	add_child(goods_box)


func show_data(sim: Object, faction: String) -> void:
	_faction = faction
	_break_armed = ""
	var overview: Dictionary = sim.call("get_trade_overview", faction) if sim != null and sim.has_method("get_trade_overview") else {}
	var options: Array = sim.call("get_trade_partner_options", faction) if sim != null and sim.has_method("get_trade_partner_options") else []
	var economy: Dictionary = sim.call("get_faction_economy", faction) if sim != null and sim.has_method("get_faction_economy") else {}
	summary_label.text = "Revenu des routes : %s par saison (saison passée : %s)." % [
		Money.amount(int(overview.get("income", economy.get("trade_income", 0)))), Money.amount(int(economy.get("trade_income_last_turn", 0)))]
	_show_agreements(options)
	_show_partners(options)
	_show_routes(overview.get("routes", []))
	_show_goods(economy.get("goods", {}))


## Une ligne d'accord (testable) : « Angleterre — +420 ₶ par saison pour nous ».
static func agreement_text(option: Dictionary) -> String:
	return "%s — %s par saison pour nous" % [str(option.get("name", "")), Money.signed(int(option.get("current_self", 0)))]


## Une ligne de partenaire possible (testable), projection avant signature comprise.
static func partner_text(option: Dictionary) -> String:
	return "%s — %s par saison pour nous, %s pour eux" % [
		str(option.get("name", "")), Money.signed(int(option.get("gain_self", 0))), Money.signed(int(option.get("gain_partner", 0)))]


func _show_agreements(options: Array) -> void:
	UiBuild.clear_children(agreements_box)
	agreement_rows.clear()
	buttons["break"].clear()
	for option in options:
		if not bool(option.get("signed", false)):
			continue
		var id := str(option.get("faction", ""))
		var row := UiBuild.hbox(8)
		var label := EconomyIcons.chip("agreement", agreement_text(option))
		label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		row.add_child(label)
		var drop := UiBuild.button("Rompre")
		drop.name = "Break_" + id
		drop.pressed.connect(_on_break_pressed.bind(id, drop))
		row.add_child(drop)
		buttons["break"][id] = drop
		agreements_box.add_child(row)
		agreement_rows.append({"id": id, "text": agreement_text(option)})
	if agreement_rows.is_empty():
		agreements_box.add_child(_muted("Aucun accord commercial en vigueur."))


func _show_partners(options: Array) -> void:
	UiBuild.clear_children(partners_box)
	partner_rows.clear()
	buttons["propose"].clear()
	var shown := 0
	for option in options:
		if bool(option.get("signed", false)) or int(option.get("gain_self", 0)) <= 0:
			continue
		if shown >= MAX_PARTNERS_SHOWN:
			break
		var id := str(option.get("faction", ""))
		var row := UiBuild.hbox(8)
		var label := EconomyIcons.chip("trade", partner_text(option))
		label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		row.add_child(label)
		var propose := UiBuild.button("Proposer l'accord")
		propose.name = "Propose_" + id
		propose.pressed.connect(func() -> void:
			order_requested.emit({"type": "propose_treaty", "target": id, "articles": [{"kind": "trade_agreement"}]}, "Accord commercial signé."))
		row.add_child(propose)
		buttons["propose"][id] = propose
		partners_box.add_child(row)
		partner_rows.append({"id": id, "text": partner_text(option), "gain_self": int(option.get("gain_self", 0))})
		shown += 1
	if partner_rows.is_empty():
		partners_box.add_child(_muted("Aucun nouveau partenaire : routes coupées, guerre ou accords déjà signés."))


func _show_routes(routes: Array) -> void:
	UiBuild.clear_children(routes_box)
	route_rows.clear()
	var cut := 0
	for route in routes:
		if not bool(route.get("mine", false)):
			continue
		if bool(route.get("cut", false)):
			cut += 1
		if route_rows.size() >= MAX_ROUTES_SHOWN:
			continue
		var text := TradePanel.route_text(route)
		var label := UiBuild.label(text, UiType.size(UiType.CAPTION), null, true)
		if bool(route.get("cut", false)):
			label.add_theme_color_override("font_color", Money.LOSS_COLOR)
		routes_box.add_child(label)
		route_rows.append({"route": str(route.get("id", "")), "text": text})
	if route_rows.is_empty():
		routes_box.add_child(_muted("Aucune route commerciale."))
	elif cut > 0:
		routes_box.add_child(_muted("%d route(s) coupée(s) : voir la raison sur la ligne." % cut))


func _show_goods(goods: Dictionary) -> void:
	UiBuild.clear_children(goods_box)
	goods_shown.clear()
	var keys := goods.keys()
	keys.sort()
	for resource in keys:
		var text := "%s × %d" % [str(resource).trim_prefix("res_").capitalize(), int(goods[resource])]
		goods_box.add_child(IconChip.create(EconomyIcons.resource_icon_id(str(resource)), text, "", 18.0, 13))
		goods_shown.append(str(resource))


func _on_break_pressed(id: String, button: Button) -> void:
	if _break_armed != id:
		_break_armed = id
		button.text = "Confirmer ?"
		return
	_break_armed = ""
	order_requested.emit({"type": "break_trade_agreement", "target": id}, "Accord commercial rompu.")


func _section(title: String) -> VBoxContainer:
	add_child(HSeparator.new())
	var label := UiBuild.label(title, UiType.size(UiType.BODY))
	label.add_theme_color_override("font_color", BudgetTable.HEADER_COLOR)
	add_child(label)
	var box := UiBuild.vbox(3)
	add_child(box)
	return box


func _muted(text: String) -> Label:
	return UiBuild.label(text, UiType.size(UiType.CAPTION), MUTED, true)
