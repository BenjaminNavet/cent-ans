class_name DiplomacyActionsSection
extends DiplomacyView

## Actions unilatérales sans négociation (guerre, embargo, présents, ruptures, médiation) ; le
## survol d'une action affiche ses conséquences (`evaluate_proposal`).

const GIFT_AMOUNT := 1000

var _actions: HFlowContainer
var _hint: RichTextLabel


func _init() -> void:
	super("Actions", 6)
	add_child(_rule())
	add_child(_label("Actions unilatérales", UiType.HEADING, HudStyle.INK))
	_actions = HFlowContainer.new()
	_actions.add_theme_constant_override("h_separation", 6)
	_actions.add_theme_constant_override("v_separation", 6)
	add_child(_actions)
	_hint = RichTextLabel.new()
	_hint.bbcode_enabled = true
	_hint.fit_content = true
	_hint.scroll_active = false
	add_child(_hint)
	_codex_labels.append(_hint)


## Actions sans négociation (guerre, embargo, présents, ruptures, médiation). Survol = conséquences.
func _render() -> void:
	UiBuild.clear_children(_actions)
	_hint.text = ""
	var id := str(entry["id"])
	var status := str(entry["status"])
	if status == "war":
		_add_action("Médiation pontificale (%s)" % Money.amount(int(RuleValues.value("mediation_cost", 0.0))), {"type": "request_papal_mediation", "target": id}, "Le pape obtient une trêve.", false)
	elif status != "alliance" and status != "vassal" and status != "suzerain":
		_add_action("Déclarer la guerre", {"type": "declare_war", "target": id}, "La guerre est déclarée.", true)
	if status == "alliance":
		_add_action("Rompre l'alliance", {"type": "break_alliance", "target": id}, "Alliance rompue.", true)
	if status == "vassal":
		_add_action("Libérer le vassal", {"type": "release_vassal", "target": id}, "Vassal libéré.", true)
	var embargo := bool(entry.get("embargo_by_us", false))
	_add_action("Lever l'embargo" if embargo else "Imposer un embargo",
		{"type": "set_embargo", "target": id, "active": not embargo},
		"Embargo levé." if embargo else "Embargo imposé.", true)
	_add_action("Présents (%s)" % Money.amount(GIFT_AMOUNT), {"type": "send_gift", "target": id, "amount": GIFT_AMOUNT}, "Présents envoyés.", true)
	# C5 : l'accord commercial se conclut par un article de traité ; la rupture est unilatérale.
	if bool(entry.get("trade_agreement", false)):
		_add_action("Rompre l'accord commercial", {"type": "break_trade_agreement", "target": id}, "Accord commercial rompu.", true)


func _add_action(label: String, order: Dictionary, success_text: String, unilateral: bool) -> void:
	var button := UiBuild.button(label, func() -> void: order_requested.emit(order, success_text))
	button.focus_mode = Control.FOCUS_NONE
	button.mouse_entered.connect(func() -> void: _show_consequences(order, unilateral))
	_actions.add_child(button)


func _show_consequences(order: Dictionary, unilateral: bool) -> void:
	if sim == null:
		return
	var type := str(order.get("type", ""))
	if unilateral and type != "declare_war":
		_hint.text = ""
		return
	var verdict: Dictionary = sim.call("evaluate_proposal", order)
	var text := "[b]Conséquences :[/b] " if type == "declare_war" else ("[b][color=#2a6a2a]Accepterait[/color][/b] : " if bool(verdict.get("accept", false)) else "[b][color=#8b1a1a]Refuserait[/color][/b] : ")
	var parts := PackedStringArray()
	for reason in verdict.get("reasons", []):
		var v := int(reason["value"])
		var reason_text := CodexText.format(str(reason["text"]), true)
		parts.append(("[color=%s]%+d[/color] %s" % ["#2a6a2a" if v >= 0 else "#8b1a1a", v, reason_text]) if v != 0 else reason_text)
	_hint.text = text + " · ".join(parts)
	if type == "declare_war":  # FE6 : chaîne d'escalade avant la déclaration
		var player := str(sim.call("get_player_faction")) if sim.has_method("get_player_faction") else ""
		var chain := EscalationPreview.bbcode(sim, player, str(order.get("target", "")))
		if chain != "":
			_hint.text += "\n" + chain
