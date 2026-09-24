class_name ChivalrySection
extends VBoxContainer

## H11 — section « Ordre de chevalerie » du panneau de faction, construite en code : ordre fondé
## (nom lié au Codex, membres cliquables, bonus, « brisé » après un désastre) ou options de
## fondation (coût, prestige requis, raison du refus, infobulle `RichTooltip.chivalric_order`).
## Aucune règle ici : tout vient de `CampaignSim.get_chivalric_orders` / `submit_order`
## (`docs/design/h5-h6-api.md` § 4 et 5).

signal order_founded(order_id: String)

const ERROR_COLOR := Color(0.55, 0.20, 0.15)
const MUTED_COLOR := Color(0.42, 0.33, 0.20)
## Fiches Codex des ordres historiques (repli si `entry_for_entity` ne connaît pas l'ordre).
const ORDER_CODEX := {"ord_garter": "cdx_ordre_de_la_jarretiere", "ord_star": "cdx_ordre_de_l_etoile", "ord_golden_fleece": "cdx_toison_or", "ord_court_company": "cdx_chevalerie"}

var orders: Dictionary = {}
var found_buttons: Dictionary = {}
var member_links: Array = []
var last_result: Dictionary = {}
var read_only := false

var header_label: Label
var body: VBoxContainer
var error_label: Label
var _sim: Object = null
var _texts: Array = []


func _init() -> void:
	name = "ChivalrySection"
	add_theme_constant_override("separation", 3)
	size_flags_horizontal = Control.SIZE_EXPAND_FILL
	header_label = Label.new()
	header_label.text = "Ordre de chevalerie"
	header_label.add_theme_font_size_override("font_size", 16)
	add_child(header_label)
	body = VBoxContainer.new()
	body.add_theme_constant_override("separation", 3)
	add_child(body)
	error_label = Label.new()
	error_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	error_label.custom_minimum_size = Vector2(300, 0)
	error_label.add_theme_font_size_override("font_size", 12)
	error_label.add_theme_color_override("font_color", ERROR_COLOR)
	error_label.hide()
	add_child(error_label)


## Remplit la section (ordres du joueur) ; masquée si la simulation n'expose pas l'API.
func show_for(player_owned: bool = true, sim: Object = null) -> void:
	read_only = not player_owned
	_sim = sim if sim != null else _facade_sim()
	if _sim == null or not _sim.has_method("get_chivalric_orders") or read_only:
		hide()
		return
	orders = _sim.call("get_chivalric_orders")
	show()
	for child in body.get_children():
		body.remove_child(child)
		child.queue_free()
	found_buttons.clear()
	member_links.clear()
	_texts.clear()
	var founded: Dictionary = orders.get("founded", {})
	if not founded.is_empty():
		_show_founded(founded)
	else:
		_show_options(orders.get("options", []))
	_attach_texts.call_deferred()


func _show_founded(founded: Dictionary) -> void:
	var order_id := str(founded.get("order", ""))
	var option := _option(order_id)
	var status := " — [color=#8b1a1a]brisé[/color]" if bool(founded.get("collapsed", false)) else ""
	body.add_child(_rich("[b]%s[/b]%s" % [_codex_name(order_id, str(founded.get("name", order_id))), status], 14))
	if not option.is_empty():
		var bonus := "Membres : loyauté +%d, moral +%d · souverain : prestige +%d par an" % [int(option.get("member_loyalty", 0)), int(option.get("member_morale", 0)), int(option.get("yearly_prestige", 0))]
		if bool(founded.get("collapsed", false)):
			bonus = "Plus aucun bonus depuis la perte de la moitié de ses membres."
		body.add_child(_label(bonus, 12, MUTED_COLOR))
	var members: Array = founded.get("members", [])
	body.add_child(_label("Membres (%d%s) :" % [members.size(), "/%d" % int(option.get("members", 0)) if not option.is_empty() else ""], 12, MUTED_COLOR))
	var flow := HFlowContainer.new()
	flow.add_theme_constant_override("h_separation", 8)
	for member in members:
		var character_id := str(member.get("character", ""))
		var link := LinkButton.new()
		link.text = str(member.get("name", character_id))
		link.add_theme_font_size_override("font_size", 13)
		RansomPanel.style_link(link)
		link.tooltip_text = "Ouvrir la fiche du personnage"
		link.pressed.connect(func() -> void: _request_character(character_id))
		flow.add_child(link)
		member_links.append(link)
	body.add_child(flow)


func _show_options(options: Array) -> void:
	if options.is_empty():
		body.add_child(_label("Aucun ordre ne peut être fondé.", 12, MUTED_COLOR))
		return
	for option in options:
		var id := str(option.get("id", ""))
		body.add_child(_rich("%s — %s %s, prestige requis %d" % [_codex_name(id, str(option.get("name", id))), RichTooltip.thousands(int(option.get("cost", 0))), RichTooltip.POUND, int(option.get("prestige_required", 0))], 13))
		var button := RichButton.new()
		button.text = "Fonder l'ordre"
		button.alignment = HORIZONTAL_ALIGNMENT_LEFT
		button.tooltip_text = RichTooltip.chivalric_order(option)
		# Un ordre indisponible reste cliquable : la simulation motive le refus.
		button.pressed.connect(func() -> void: request_found(id))
		body.add_child(button)
		found_buttons[id] = button
		var reason := str(option.get("reason", ""))
		if not bool(option.get("available", false)) and reason != "":
			body.add_child(_label("Impossible : %s" % reason, 12, ERROR_COLOR))


## Ordre `found_chivalric_order` ; en cas de refus, message de la simulation affiché en rouge.
func request_found(order_id: String) -> Dictionary:
	if _sim == null:
		return {}
	last_result = _sim.call("submit_order", {"type": "found_chivalric_order", "order": order_id})
	var ok := bool(last_result.get("ok", false))
	show_for(true, _sim)
	error_label.visible = not ok
	error_label.text = "Refusé : %s" % str(last_result.get("error", "?")) if not ok else ""
	if ok:
		order_founded.emit(order_id)
	return last_result


## Fiche Codex de l'ordre : `entry_for_entity`, sinon la fiche historique connue.
static func codex_entry(order_id: String) -> String:
	var entry := RansomPanel.codex_entry_for(order_id)
	return entry if entry != "" else str(ORDER_CODEX.get(order_id, ""))


static func _codex_name(order_id: String, display: String) -> String:
	var entry := codex_entry(order_id)
	return CodexText.format("[[%s|%s]]" % [entry, display]) if entry != "" else display


func _option(order_id: String) -> Dictionary:
	for option in orders.get("options", []):
		if str(option.get("id", "")) == order_id:
			return option
	return {}


func _label(text: String, font_size: int, color: Color) -> Label:
	var label := Label.new()
	label.text = text
	label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	label.custom_minimum_size = Vector2(300, 0)
	label.add_theme_font_size_override("font_size", font_size)
	label.add_theme_color_override("font_color", color)
	return label


func _rich(bbcode: String, font_size: int) -> RichTextLabel:
	var label := RichTextLabel.new()
	label.bbcode_enabled = true
	label.fit_content = true
	label.scroll_active = false
	label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	label.custom_minimum_size = Vector2(300, 0)
	label.add_theme_color_override("default_color", RichTooltip.INK)
	for key in ["normal_font_size", "bold_font_size"]:
		label.add_theme_font_size_override(key, font_size)
	label.text = bbcode
	_texts.append(label)
	return label


func _attach_texts() -> void:
	var bubbles := RansomPanel._root_node("/root/CodexBubbles")
	if bubbles == null:
		return
	for label in _texts:
		if is_instance_valid(label):
			bubbles.call("attach", label)


func _request_character(character_id: String) -> void:
	var node := get_parent()
	while node != null:
		if node.has_signal("character_selected"):
			node.emit_signal("character_selected", character_id)
			return
		node = node.get_parent()


func _facade_sim() -> Object:
	var facade := RansomPanel._root_node("/root/SimFacade")
	return facade.get("sim") if facade != null else null
