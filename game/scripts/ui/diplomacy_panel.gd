class_name DiplomacyPanel
extends PanelContainer

## Panneau « Diplomatie » (bouton de la barre, touche P) : factions vivantes avec statut et
## attitude envers le joueur, fiche de la faction choisie (raisons de l'attitude, score de guerre,
## prétentions, religion) et actions diplomatiques. Chaque action affiche avant envoi le verdict
## de la simulation (`evaluate_proposal`). Aucune règle ici : tout vient de `CampaignSim`.

signal order_requested(order: Dictionary, success_text: String)
signal offer_answered(offer_id: int, accept: bool)
signal closed

const STATUS_LABELS := {
	"war": "En guerre", "truce": "Trêve", "peace": "Paix", "alliance": "Alliance",
	"vassal": "Notre vassal", "suzerain": "Notre suzerain",
}
const STATUS_COLORS := {
	"war": Color(0.62, 0.12, 0.10), "truce": Color(0.70, 0.55, 0.10), "peace": Color(0.35, 0.33, 0.30),
	"alliance": Color(0.15, 0.32, 0.62), "vassal": Color(0.42, 0.20, 0.55), "suzerain": Color(0.42, 0.20, 0.55),
}
const GIFT_AMOUNT := 1000
const DONATION_AMOUNT := 1000

var sim: Object = null
var player_faction: String = ""
var province_name_of: Callable = Callable()

var _entries: Array = []
var _selected: String = ""
var _list: VBoxContainer
var _detail: VBoxContainer
var _religion_label: Label
var _offers_box: VBoxContainer
var _peace_checks: Dictionary = {}  # province_id -> CheckBox
var _tribute_spin: SpinBox
var _verdict_label: RichTextLabel


func _ready() -> void:
	theme = load("res://scenes/ui/parchment_theme.tres")
	set_anchors_preset(Control.PRESET_CENTER)
	custom_minimum_size = Vector2(980, 640)
	offset_left = -490
	offset_right = 490
	offset_top = -330
	offset_bottom = 330
	var root := VBoxContainer.new()
	root.add_theme_constant_override("separation", 6)
	add_child(root)

	var header := HBoxContainer.new()
	var title := Label.new()
	title.text = "Diplomatie"
	title.add_theme_font_size_override("font_size", 22)
	title.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	header.add_child(title)
	var close := Button.new()
	close.text = "×"
	close.pressed.connect(func() -> void:
		hide()
		closed.emit())
	header.add_child(close)
	root.add_child(header)

	var religion_row := HBoxContainer.new()
	_religion_label = Label.new()
	_religion_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	religion_row.add_child(_religion_label)
	var donate := Button.new()
	donate.text = "Don à l'Église (%d)" % DONATION_AMOUNT
	donate.tooltip_text = "Augmente la faveur pontificale (+1 par 200 livres)."
	donate.pressed.connect(func() -> void:
		order_requested.emit({"type": "donate_to_church", "amount": DONATION_AMOUNT}, "Don versé à l'Église."))
	religion_row.add_child(donate)
	root.add_child(religion_row)

	_offers_box = VBoxContainer.new()
	root.add_child(_offers_box)
	root.add_child(HSeparator.new())

	var body := HBoxContainer.new()
	body.size_flags_vertical = Control.SIZE_EXPAND_FILL
	body.add_theme_constant_override("separation", 12)
	root.add_child(body)

	var list_scroll := ScrollContainer.new()
	list_scroll.custom_minimum_size = Vector2(360, 0)
	list_scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	_list = VBoxContainer.new()
	_list.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	list_scroll.add_child(_list)
	body.add_child(list_scroll)

	var detail_scroll := ScrollContainer.new()
	detail_scroll.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	detail_scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	_detail = VBoxContainer.new()
	_detail.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_detail.add_theme_constant_override("separation", 4)
	detail_scroll.add_child(_detail)
	body.add_child(detail_scroll)


## Recharge tout depuis la simulation (appelé à l'ouverture et après chaque ordre).
func refresh() -> void:
	if sim == null:
		return
	_entries = sim.call("get_diplomacy", player_faction)
	_entries.sort_custom(func(a: Dictionary, b: Dictionary) -> bool:
		var rank_a := _status_rank(str(a["status"]))
		var rank_b := _status_rank(str(b["status"]))
		if rank_a != rank_b:
			return rank_a < rank_b
		return str(a["name"]) < str(b["name"]))
	if _selected == "" and not _entries.is_empty():
		_selected = str(_entries[0]["id"])
	_render_religion()
	_render_offers()
	_render_list()
	_render_detail()


func select_faction(faction_id: String) -> void:
	_selected = faction_id
	_render_list()
	_render_detail()


static func _status_rank(status: String) -> int:
	return ["war", "suzerain", "vassal", "alliance", "truce", "peace"].find(status)


func _render_religion() -> void:
	var religion: Dictionary = sim.call("get_religion_state", player_faction)
	var text := "Religion : %s — faveur pontificale %d/100" % [religion.get("religion_name", "?"), int(religion.get("papal_favor", 0))]
	if bool(religion.get("excommunicated", false)):
		text += " — EXCOMMUNIÉ (%d tours)" % int(religion.get("turns_left", 0))
	if bool(religion.get("schism", false)):
		text += " — Grand Schisme"
	_religion_label.text = text


func _render_offers() -> void:
	for child in _offers_box.get_children():
		child.queue_free()
	var offers: Array = sim.call("get_offers")
	if offers.is_empty():
		return
	var title := Label.new()
	title.text = "Propositions reçues"
	title.add_theme_font_size_override("font_size", 17)
	_offers_box.add_child(title)
	for offer in offers:
		var row := HBoxContainer.new()
		var text := Label.new()
		text.text = "%s (%d tour(s))" % [str(offer["text"]), int(offer["expires_in"])]
		text.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		text.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		row.add_child(text)
		var offer_id := int(offer["id"])
		var yes := Button.new()
		yes.text = "Accepter"
		yes.pressed.connect(func() -> void: offer_answered.emit(offer_id, true))
		row.add_child(yes)
		var no := Button.new()
		no.text = "Refuser"
		no.pressed.connect(func() -> void: offer_answered.emit(offer_id, false))
		row.add_child(no)
		_offers_box.add_child(row)


func _render_list() -> void:
	for child in _list.get_children():
		child.queue_free()
	for entry in _entries:
		var id := str(entry["id"])
		var row := Button.new()
		row.toggle_mode = true
		row.button_pressed = id == _selected
		row.custom_minimum_size = Vector2(0, 40)
		row.pressed.connect(func() -> void: select_faction(id))
		var line := HBoxContainer.new()
		line.set_anchors_preset(Control.PRESET_FULL_RECT)
		line.mouse_filter = Control.MOUSE_FILTER_IGNORE
		line.add_theme_constant_override("separation", 8)
		var swatch := ColorRect.new()
		swatch.color = Color.html(str(entry["color"]))
		swatch.custom_minimum_size = Vector2(14, 0)
		swatch.mouse_filter = Control.MOUSE_FILTER_IGNORE
		PortraitLoader.overlay_heraldry(swatch, id, Vector2(28, 0))  # M10 assets
		line.add_child(swatch)
		var name_label := Label.new()
		name_label.text = str(entry["name"])
		name_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		name_label.mouse_filter = Control.MOUSE_FILTER_IGNORE
		line.add_child(name_label)
		var status := str(entry["status"])
		var status_label := Label.new()
		status_label.text = STATUS_LABELS.get(status, status)
		status_label.add_theme_color_override("font_color", STATUS_COLORS.get(status, Color.BLACK))
		status_label.mouse_filter = Control.MOUSE_FILTER_IGNORE
		line.add_child(status_label)
		line.add_child(_attitude_bar(int(entry["attitude"])))
		row.add_child(line)
		_list.add_child(row)


func _attitude_bar(attitude: int) -> Control:
	var bar := ProgressBar.new()
	bar.min_value = -100
	bar.max_value = 100
	bar.value = attitude
	bar.show_percentage = false
	bar.custom_minimum_size = Vector2(70, 12)
	bar.size_flags_vertical = Control.SIZE_SHRINK_CENTER
	bar.mouse_filter = Control.MOUSE_FILTER_IGNORE
	var fill := StyleBoxFlat.new()
	fill.bg_color = Color(0.2, 0.55, 0.2) if attitude >= 0 else Color(0.7, 0.15, 0.1)
	bar.add_theme_stylebox_override("fill", fill)
	bar.tooltip_text = "Attitude %+d" % attitude
	return bar


func _entry(id: String) -> Dictionary:
	for entry in _entries:
		if str(entry["id"]) == id:
			return entry
	return {}


func _render_detail() -> void:
	for child in _detail.get_children():
		child.queue_free()
	_peace_checks.clear()
	var entry := _entry(_selected)
	if entry.is_empty():
		return
	var status := str(entry["status"])
	var title := Label.new()
	title.text = "%s — %s" % [entry["name"], STATUS_LABELS.get(status, status)]
	title.add_theme_font_size_override("font_size", 19)
	title.add_theme_color_override("font_color", STATUS_COLORS.get(status, Color.BLACK))
	_detail.add_child(title)

	var facts := PackedStringArray()
	facts.append("Religion : %s" % entry["religion_name"])
	facts.append("Puissance militaire : %d" % int(entry["power"]))
	if int(entry["truce_turns_left"]) > 0:
		facts.append("Trêve : encore %d tour(s)" % int(entry["truce_turns_left"]))
	if status == "war":
		facts.append("Score de guerre : %+d" % int(entry["war_score"]))
	if int(entry["loyalty"]) >= 0:
		facts.append("Loyauté du vassal : %d/100" % int(entry["loyalty"]))
	if bool(entry["embargo_by_us"]):
		facts.append("Nous lui imposons un embargo")
	if bool(entry["embargo_on_us"]):
		facts.append("Elle nous impose un embargo")
	if str(entry["casus_belli"]) != "":
		facts.append("Casus belli : %s" % entry["casus_belli"])
	for claim in entry["claims"]:
		facts.append("Prétention : %s" % claim)
	var facts_label := Label.new()
	facts_label.text = "\n".join(facts)
	facts_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	_detail.add_child(facts_label)

	var reasons := RichTextLabel.new()
	reasons.bbcode_enabled = true
	reasons.fit_content = true
	reasons.scroll_active = false
	var text := "[b]Attitude envers nous : %+d[/b]\n" % int(entry["attitude"])
	for reason in entry["attitude_reasons"]:
		var value := int(reason["value"])
		var color := "#2a6a2a" if value >= 0 else "#8b1a1a"
		text += "[color=%s]%+d[/color] %s\n" % [color, value, reason["text"]]
	reasons.text = text
	_detail.add_child(reasons)
	_detail.add_child(HSeparator.new())
	_render_actions(entry)


func _render_actions(entry: Dictionary) -> void:
	var id := str(entry["id"])
	var status := str(entry["status"])
	var actions := VBoxContainer.new()
	_detail.add_child(actions)
	_verdict_label = RichTextLabel.new()
	_verdict_label.bbcode_enabled = true
	_verdict_label.fit_content = true
	_verdict_label.scroll_active = false

	if status == "war":
		_render_peace_form(actions, id)
		_add_action(actions, "Médiation pontificale (1 000)", {"type": "request_papal_mediation", "target": id}, "Le pape obtient une trêve.")
	else:
		if status != "alliance" and status != "vassal" and status != "suzerain":
			_add_action(actions, "Déclarer la guerre", {"type": "declare_war", "target": id}, "La guerre est déclarée.", true)
			_add_action(actions, "Proposer une alliance", {"type": "propose_alliance", "target": id}, "Alliance conclue.")
			_add_action(actions, "Exiger la vassalité", {"type": "demand_vassalage", "target": id}, "Nouveau vassal.")
		if status == "alliance":
			_add_action(actions, "Rompre l'alliance", {"type": "break_alliance", "target": id}, "Alliance rompue.", true)
		if status == "vassal":
			_add_action(actions, "Libérer le vassal", {"type": "release_vassal", "target": id}, "Vassal libéré.", true)
	var embargo := bool(entry["embargo_by_us"])
	_add_action(actions, "Lever l'embargo" if embargo else "Imposer un embargo",
		{"type": "set_embargo", "target": id, "active": not embargo},
		"Embargo levé." if embargo else "Embargo imposé.", true)
	_add_action(actions, "Envoyer des présents (%d)" % GIFT_AMOUNT, {"type": "send_gift", "target": id, "amount": GIFT_AMOUNT}, "Présents envoyés.", true)
	actions.add_child(_verdict_label)


## Un bouton d'action : survol = verdict de la simulation, clic = envoi. `unilateral` : pas de
## verdict d'acceptation (seulement les conséquences, pour la guerre).
func _add_action(parent: Control, label: String, order: Dictionary, success_text: String, unilateral: bool = false) -> void:
	var button := Button.new()
	button.text = label
	button.alignment = HORIZONTAL_ALIGNMENT_LEFT
	button.mouse_entered.connect(func() -> void: _show_verdict(order, unilateral))
	button.focus_entered.connect(func() -> void: _show_verdict(order, unilateral))
	button.pressed.connect(func() -> void: order_requested.emit(order, success_text))
	parent.add_child(button)


func _show_verdict(order: Dictionary, unilateral: bool) -> void:
	if sim == null or _verdict_label == null:
		return
	var verdict: Dictionary = sim.call("evaluate_proposal", order)
	var text := ""
	var type := str(order.get("type", ""))
	if type == "declare_war":
		text = "[b]Conséquences :[/b]\n"
	elif unilateral:
		_verdict_label.text = ""
		return
	elif bool(verdict.get("accept", false)):
		text = "[b][color=#2a6a2a]Accepterait[/color][/b] (score %+d)\n" % int(verdict.get("score", 0))
	else:
		text = "[b][color=#8b1a1a]Refuserait[/color][/b] (score %+d)\n" % int(verdict.get("score", 0))
	for reason in verdict.get("reasons", []):
		var value := int(reason["value"])
		var color := "#2a6a2a" if value >= 0 else "#8b1a1a"
		text += "[color=%s]%+d[/color] %s\n" % [color, value, reason["text"]] if value != 0 else "• %s\n" % reason["text"]
	_verdict_label.text = text


## Formulaire de paix : provinces occupées de part et d'autre (cases à cocher), tribut.
func _render_peace_form(parent: Control, enemy: String) -> void:
	var title := Label.new()
	title.text = "Conditions de paix"
	title.add_theme_font_size_override("font_size", 16)
	parent.add_child(title)
	var provinces: Array = _occupied_between(enemy)
	if provinces.is_empty():
		var none := Label.new()
		none.text = "Aucune province occupée : paix blanche possible."
		parent.add_child(none)
	for item in provinces:
		var check := CheckBox.new()
		var ours: bool = bool(item["ours"])
		check.text = ("Céder %s" if ours else "Exiger %s") % item["name"]
		check.button_pressed = not ours
		check.toggled.connect(func(_on: bool) -> void: _show_verdict(_peace_order(enemy), false))
		_peace_checks[str(item["id"])] = check
		parent.add_child(check)
	var tribute_row := HBoxContainer.new()
	var tribute_label := Label.new()
	tribute_label.text = "Tribut (positif = exigé, négatif = offert) :"
	tribute_row.add_child(tribute_label)
	_tribute_spin = SpinBox.new()
	_tribute_spin.min_value = -20000
	_tribute_spin.max_value = 20000
	_tribute_spin.step = 500
	_tribute_spin.value_changed.connect(func(_v: float) -> void: _show_verdict(_peace_order(enemy), false))
	tribute_row.add_child(_tribute_spin)
	parent.add_child(tribute_row)
	var send := Button.new()
	send.text = "Proposer la paix"
	send.mouse_entered.connect(func() -> void: _show_verdict(_peace_order(enemy), false))
	send.pressed.connect(func() -> void: order_requested.emit(_peace_order(enemy), "La paix est signée."))
	parent.add_child(send)


func _peace_order(enemy: String) -> Dictionary:
	var provinces: Array = []
	for id in _peace_checks:
		if (_peace_checks[id] as CheckBox).button_pressed:
			provinces.append(id)
	var tribute := int(_tribute_spin.value) if _tribute_spin != null else 0
	return {"type": "propose_peace", "target": enemy, "provinces": provinces, "tribute": tribute}


## Provinces de l'ennemi que nous occupons (à exiger) et les nôtres qu'il occupe (à céder).
func _occupied_between(enemy: String) -> Array:
	var result: Array = []
	# Autoload lu à l'exécution : ce script est aussi compilé par le smoke test (--script).
	var facade: Node = get_node_or_null("/root/SimFacade")
	if facade == null or not facade.call("store_loaded"):
		return result
	for id in facade.get("store").call("get_province_ids"):
		var state: Dictionary = sim.call("get_province_state", id)
		if state.is_empty():
			continue
		var owner := str(state.get("owner", ""))
		var controller := str(state.get("controller", ""))
		var name: String = province_name_of.call(str(id)) if province_name_of.is_valid() else str(id)
		if owner == enemy and controller == player_faction:
			result.append({"id": str(id), "name": name, "ours": false})
		elif owner == player_faction and controller == enemy:
			result.append({"id": str(id), "name": name, "ours": true})
	return result
