class_name DiplomacyHeadSection
extends DiplomacyView

## En-tête de la fiche d'une faction : blason, souverain, statut et attitude, puis faits
## (religion, puissance, trêve, accord commercial, passage, loyauté, embargos) et routes
## commerciales communes.


func _init() -> void:
	super("Head", 2, false)


func _render() -> void:
	UiBuild.clear_children(self)
	if entry.is_empty():
		return
	var status := str(entry["status"])
	var top := UiBuild.hbox(10)
	top.add_child(_heraldry(faction_id, 64, str(entry.get("color", "#888888"))))
	var names := UiBuild.vbox(0)
	names.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	names.add_child(_label(str(entry["name"]), UiType.HEADING, HudStyle.INK))
	var ruler := str(entry.get("ruler", ""))
	names.add_child(_label(ruler if ruler != "" else "Souverain inconnu", UiType.BODY, HudStyle.INK_SOFT))
	var line := "%s — attitude %+d" % [STATUS_LABELS.get(status, status), int(entry["attitude"])]
	names.add_child(_label(line, UiType.BODY, STATUS_COLORS.get(status, HudStyle.INK)))
	top.add_child(names)
	add_child(top)
	var facts := PackedStringArray()
	facts.append("Religion : %s" % entry.get("religion_name", "?"))
	facts.append("Puissance : %s" % Money.digits(int(entry.get("power", 0))))
	if int(entry.get("truce_turns_left", 0)) > 0:
		facts.append("Trêve : encore %s" % FrText.count(int(entry["truce_turns_left"]), "tour", "tours"))
	if bool(entry.get("trade_agreement", false)):
		# C5 : un embargo suspend les routes sans rompre l'accord (la guerre le rompt).
		var suspended := bool(entry.get("embargo_by_us", false)) or bool(entry.get("embargo_on_us", false))
		facts.append("Accord commercial" + (" (suspendu)" if suspended else ""))
	if bool(entry.get("access_received", false)):
		facts.append("Accès militaire accordé")
	facts.append_array(_passage_facts(faction_id))
	if int(entry.get("loyalty", -1)) >= 0:
		facts.append("Loyauté %d/100" % int(entry["loyalty"]))
	if bool(entry.get("embargo_by_us", false)):
		facts.append("Sous notre embargo")
	if bool(entry.get("embargo_on_us", false)):
		facts.append("Nous impose un embargo")
	var facts_label := _label(" · ".join(facts), UiType.CAPTION, HudStyle.INK_SOFT)
	facts_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	add_child(facts_label)
	_render_trade_routes(faction_id)


## Position diplomatique, droit de passage et intrusions entre nous et `id`.
func _passage_facts(id: String) -> PackedStringArray:
	var facts := PackedStringArray()
	if sim == null:
		return facts
	if sim.has_method("get_faction_stance"):
		var stance: Dictionary = sim.call("get_faction_stance", id)
		if not stance.is_empty():
			facts.append("Position : %s" % stance.get("label", ""))
	if not sim.has_method("get_trespass"):
		return facts
	var info: Dictionary = sim.call("get_trespass", id)
	if info.is_empty():
		return facts
	var theirs: Dictionary = info.get("theirs", {})
	var ours: Dictionary = info.get("ours", {})
	if bool(info.get("access_given", false)):
		facts.append("Droit de passage donné")
	if int(theirs.get("seasons", 0)) > 0:
		facts.append("Leurs armées campent sur nos terres (%s)" % FrText.count(int(theirs["seasons"]), "saison", "saisons"))
	if bool(theirs.get("grievance", false)):
		facts.append("Casus belli : violation de nos frontières")
	if int(ours.get("seasons", 0)) > 0:
		facts.append("Nos armées campent chez eux sans droit de passage (%s)" % FrText.count(int(ours["seasons"]), "saison", "saisons"))
	if bool(ours.get("grievance", false)):
		facts.append("Ils tiennent un casus belli contre nous (intrusion)")
	return facts


## Routes commerciales entre nous et cette faction (revenu par saison, biens, coupure).
func _render_trade_routes(id: String) -> void:
	if sim == null or not sim.has_method("get_trade_routes"):
		return
	var lines := PackedStringArray()
	for route_variant in sim.call("get_trade_routes"):
		var route: Dictionary = route_variant
		var from_f := str(route.get("from_faction", ""))
		var to_f := str(route.get("to_faction", ""))
		if not ((from_f == player_faction and to_f == id) or (from_f == id and to_f == player_faction)):
			continue
		if bool(route.get("cut", false)):
			lines.append("%s ↔ %s : coupée (%s)" % [route["from_hub_name"], route["to_hub_name"], route["cut_reason"]])
		else:
			var goods: PackedStringArray = route.get("goods", PackedStringArray())
			lines.append("%s ↔ %s : %s/saison (%s)" % [route["from_hub_name"], route["to_hub_name"],
				Money.amount(int(route["total_value"])), ", ".join(goods)])
	if lines.is_empty():
		return
	var label := _label("Commerce — " + " · ".join(lines), UiType.CAPTION, HudStyle.INK_SOFT)
	label.name = "TradeRoutes"
	label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	add_child(label)
