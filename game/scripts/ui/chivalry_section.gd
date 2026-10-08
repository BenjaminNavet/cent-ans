class_name ChivalrySection
extends PanelSection

## Section « Ordre de chevalerie » du panneau de faction, construite en code : ordre fondé
## (nom lié au Codex, membres cliquables, bonus, « brisé » après un désastre) ou options de
## fondation (coût, prestige requis, raison du refus, infobulle `RichTooltip.chivalric_order`).
## Aucune règle ici : tout vient de `CampaignSim.get_chivalric_orders` / `submit_order`
## (`docs/design/h5-h6-api.md` § 4 et 5).

signal order_founded(order_id: String)

## Fiches Codex des ordres historiques (repli si `entry_for_entity` ne connaît pas l'ordre).
const ORDER_CODEX := {"ord_garter": "cdx_ordre_de_la_jarretiere", "ord_star": "cdx_ordre_de_l_etoile", "ord_golden_fleece": "cdx_toison_or", "ord_court_company": "cdx_chevalerie"}

var orders: Dictionary = {}
var found_buttons: Dictionary = {}
var member_links: Array = []

var header_label: Label
var body: VBoxContainer
var _texts: Array = []


func _init() -> void:
	super("ChivalrySection", 3)
	header_label = UiBuild.label("Ordre de chevalerie")
	UiType.apply(header_label, UiType.BODY)
	add_child(header_label)
	body = UiBuild.vbox(3)
	add_child(body)
	_add_error_label(300.0)


## Remplit la section (ordres du joueur) ; masquée si la simulation n'expose pas l'API.
func show_for(player_owned: bool = true, sim: Object = null) -> void:
	read_only = not player_owned
	_sim = _resolve_sim(sim)
	if _sim == null or not _sim.has_method("get_chivalric_orders") or read_only:
		hide()
		return
	orders = _sim.call("get_chivalric_orders")
	show()
	UiBuild.clear_children(body)
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
	body.add_child(_rich("[b]%s[/b]%s" % [_codex_name(order_id, str(founded.get("name", order_id))), status], UiType.CAPTION))
	if not option.is_empty():
		var bonus := "Membres : loyauté +%d, moral +%d · souverain : prestige +%d par an" % [int(option.get("member_loyalty", 0)), int(option.get("member_morale", 0)), int(option.get("yearly_prestige", 0))]
		if bool(founded.get("collapsed", false)):
			bonus = "Plus aucun bonus depuis la perte de la moitié de ses membres."
		body.add_child(_label(bonus, UiType.CAPTION, MUTED_COLOR))
	var members: Array = founded.get("members", [])
	body.add_child(_label("Membres (%d%s) :" % [members.size(), "/%d" % int(option.get("members", 0)) if not option.is_empty() else ""], UiType.CAPTION, MUTED_COLOR))
	var flow := HFlowContainer.new()
	flow.add_theme_constant_override("h_separation", 8)
	for member in members:
		var character_id := str(member.get("character", ""))
		var link := LinkButton.new()
		link.text = str(member.get("name", character_id))
		UiType.apply(link, UiType.CAPTION)
		RansomPanel.style_link(link)
		TooltipHost.attach_plain(link, "open_character_sheet")
		link.pressed.connect(func() -> void: _request_character(character_id))
		flow.add_child(link)
		member_links.append(link)
	body.add_child(flow)


func _show_options(options: Array) -> void:
	if options.is_empty():
		body.add_child(_label("Aucun ordre ne peut être fondé.", UiType.CAPTION, MUTED_COLOR))
		return
	for option in options:
		var id := str(option.get("id", ""))
		body.add_child(_rich("%s — %s %s, prestige requis %d" % [_codex_name(id, str(option.get("name", id))), Money.digits(int(option.get("cost", 0))), RichTooltip.POUND, int(option.get("prestige_required", 0))], UiType.CAPTION))
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
			body.add_child(_label("Impossible : %s" % reason, UiType.CAPTION, ERROR_COLOR))


## Ordre `found_chivalric_order` ; en cas de refus, message de la simulation affiché en rouge.
func request_found(order_id: String) -> Dictionary:
	if _sim == null:
		return {}
	if _submit({"type": "found_chivalric_order", "order": order_id}, func() -> void: show_for(true, _sim)):
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


## `variation` : une taille `UiType` (`UiType.CAPTION` ici).
func _label(text: String, variation: String, color: Color) -> Label:
	var label := UiBuild.label(text, 0, null, true, 300)
	UiType.apply(label, variation)
	label.add_theme_color_override("font_color", color)
	return label


func _rich(bbcode: String, variation: String) -> RichTextLabel:
	var label := RichTextLabel.new()
	label.bbcode_enabled = true
	label.fit_content = true
	label.scroll_active = false
	label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	label.custom_minimum_size = Vector2(300, 0)
	label.add_theme_color_override("default_color", RichTooltip.INK)
	UiType.apply(label, variation)
	label.add_theme_font_size_override("bold_font_size", UiType.size(variation))
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
