class_name RansomPanel
extends PanelContainer

## H11 — fenêtre « Captifs et rançons » (ouverte depuis le panneau de faction), construite en
## code : « Nos captifs » (payer comptant ou en 2 à 6 échéances, ou céder la province exigée),
## « Nos prisonniers » (termes : argent, province cessible, garder ; libérer sur parole), dettes
## de rançon et échéances. Noms cliquables vers la fiche du personnage et, s'il en a une, vers
## le Codex. Aucune règle ici : montants, plans, provinces cessibles et refus viennent de
## `CampaignSim.get_ransoms` / `submit_order` (`docs/design/h5-h6-api.md` § 3 et 5).

signal character_requested(character_id: String)
signal ransoms_changed
signal closed

const ERROR_COLOR := Color(0.55, 0.20, 0.15)
const MUTED_COLOR := Color(0.42, 0.33, 0.20)
const RUBRIC_COLOR := Color(0.45, 0.12, 0.08)
const LINK_COLOR := Color(0.10, 0.23, 0.55)
const PORTRAIT_SIZE := Vector2(40, 40)
const MAX_LIST_HEIGHT := 520.0
const TERMS_LABELS := {"money": "rançon en argent", "province": "exige une province", "hold": "refuse toute rançon", "parole": "libération sur parole"}

var data: Dictionary = {}
var last_result: Dictionary = {}
## Contrôles par personnage, pour les tests : `{character: {pay, plan, pay_plan, terms, set_terms, parole}}`.
var rows: Dictionary = {}

var title_label: Label
var error_label: Label
var list_box: VBoxContainer
var scroll: ScrollContainer
var _sim: Object = null


func _init() -> void:
	name = "RansomPanel"
	custom_minimum_size = Vector2(560, 0)
	var box := UiBuild.vbox(6)
	add_child(box)
	var header := HBoxContainer.new()
	box.add_child(header)
	title_label = UiBuild.label("Captifs et rançons", UiType.size(UiType.HEADING))
	title_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	header.add_child(title_label)
	var close_button := UiBuild.button("×")
	TooltipHost.attach_plain(close_button, "close")
	close_button.pressed.connect(close)
	header.add_child(close_button)
	box.add_child(HSeparator.new())
	error_label = Label.new()
	error_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	error_label.custom_minimum_size = Vector2(520, 0)
	error_label.add_theme_font_size_override("font_size", UiType.size(UiType.CAPTION))
	error_label.add_theme_color_override("font_color", ERROR_COLOR)
	error_label.hide()
	box.add_child(error_label)
	scroll = ScrollContainer.new()
	scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	box.add_child(scroll)
	list_box = UiBuild.vbox(4)
	list_box.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	scroll.add_child(list_box)


func _ready() -> void:
	if ResourceLoader.exists("res://scenes/ui/parchment_theme.tres"):
		theme = load("res://scenes/ui/parchment_theme.tres")


func close() -> void:
	hide()
	closed.emit()


## Relit `get_ransoms` (`sim` : `SimFacade.sim` si null) et affiche la fenêtre.
func refresh(sim: Object = null) -> void:
	if sim != null:
		_sim = sim
	elif _sim == null:
		_sim = _facade_sim()
	var ransoms: Dictionary = _sim.call("get_ransoms") if _sim != null and _sim.has_method("get_ransoms") else {}
	show_data(ransoms)


## Affiche `ransoms` (format de `get_ransoms` : `{ours[], held[], debts[]}`), réel ou simulé.
func show_data(ransoms: Dictionary) -> void:
	data = ransoms
	rows.clear()
	for child in list_box.get_children():
		list_box.remove_child(child)
		child.queue_free()
	var ours: Array = ransoms.get("ours", [])
	var held: Array = ransoms.get("held", [])
	var debts: Array = ransoms.get("debts", [])
	list_box.add_child(_heading("Nos captifs", "Nos gens aux mains de l'ennemi : payez leur rançon (voir [[cdx_rancon]])."))
	if ours.is_empty():
		list_box.add_child(_muted("Aucun des nôtres n'est captif."))
	for captive in ours:
		list_box.add_child(_captive_row(captive, true))
	list_box.add_child(_heading("Nos prisonniers", "Captifs ennemis que nous détenons : fixez leurs termes."))
	if held.is_empty():
		list_box.add_child(_muted("Nous ne détenons aucun prisonnier."))
	for captive in held:
		list_box.add_child(_captive_row(captive, false))
	list_box.add_child(_heading("Dettes de rançon", "Échéances annuelles restant dues."))
	if debts.is_empty():
		list_box.add_child(_muted("Aucune dette de rançon."))
	for debt in debts:
		list_box.add_child(_debt_row(debt))
	show()
	_fit_height.call_deferred()


func _fit_height() -> void:
	scroll.custom_minimum_size.y = minf(list_box.get_combined_minimum_size().y, MAX_LIST_HEIGHT)
	reset_size()


func _heading(text: String, hint: String) -> Control:
	var box := UiBuild.vbox(0)
	var label := UiBuild.label(text, UiType.size(UiType.BODY), RUBRIC_COLOR, false, 0.0, box)
	var rich := RichTextLabel.new()
	rich.bbcode_enabled = true
	rich.fit_content = true
	rich.scroll_active = false
	rich.custom_minimum_size = Vector2(520, 0)
	rich.add_theme_color_override("default_color", MUTED_COLOR)
	rich.add_theme_font_size_override("normal_font_size", UiType.size(UiType.CAPTION))
	rich.text = CodexText.format(hint)
	box.add_child(rich)
	var bubbles := _root_node("/root/CodexBubbles")
	if bubbles != null:
		bubbles.call("attach", rich)
	return box


func _muted(text: String) -> Label:
	var label := UiBuild.label(text, UiType.size(UiType.CAPTION), MUTED_COLOR)
	return label


## Portrait (ou blason) + nom cliquable + bouton Codex si le personnage a une fiche.
func _identity(character_id: String, name_text: String, faction_id: String, subtitle: String) -> Control:
	var line := UiBuild.hbox(8)
	var frame := PanelContainer.new()
	frame.custom_minimum_size = PORTRAIT_SIZE
	var swatch := ColorRect.new()
	swatch.custom_minimum_size = PORTRAIT_SIZE
	swatch.color = _faction_color(faction_id)
	frame.add_child(swatch)
	PortraitLoader.overlay_portrait(swatch, character_id, faction_id, PORTRAIT_SIZE)
	line.add_child(frame)
	var text_box := UiBuild.vbox(0)
	text_box.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	line.add_child(text_box)
	var name_row := HBoxContainer.new()
	text_box.add_child(name_row)
	var link := LinkButton.new()
	link.text = name_text
	TooltipHost.attach_plain(link, "open_character_sheet")
	link.add_theme_font_size_override("font_size", UiType.size(UiType.CAPTION))
	style_link(link)
	link.pressed.connect(func() -> void: request_character(character_id))
	name_row.add_child(link)
	var codex_id := codex_entry_for(character_id)
	if codex_id != "":
		var codex_button := UiBuild.button("✠")
		InkGlyph.apply_button(codex_button, "glyph_cross", "✠")
		codex_button.flat = true
		TooltipHost.attach_plain(codex_button, "historical_sheet_codex")
		codex_button.pressed.connect(func() -> void:
			var bubbles := _root_node("/root/CodexBubbles")
			if bubbles != null:
				bubbles.call("open_entry", codex_id))
		name_row.add_child(codex_button)
	var sub := UiBuild.label(subtitle, UiType.size(UiType.CAPTION), MUTED_COLOR, true, 440, text_box)
	return line


func _captive_row(captive: Dictionary, ours: bool) -> Control:
	var character_id := str(captive.get("character", ""))
	var terms: Dictionary = captive.get("terms", {})
	var kind := str(terms.get("kind", "money"))
	var ransom := int(captive.get("ransom", 0))
	var other := str(captive.get("captor", "")) if ours else str(captive.get("faction", ""))
	var subtitle := "%s · %s %s · prestige %d · rançon %s" % [
		str(captive.get("rank_label", captive.get("rank", ""))),
		"détenu par" if ours else "de", _faction_name(other), int(captive.get("prestige", 0)), _pounds(ransom)]
	var terms_text := str(TERMS_LABELS.get(kind, kind))
	if kind == "province":
		terms_text += " : " + _province_name(str(terms.get("province", "")))
	subtitle += "\nTermes : " + terms_text
	var box := UiBuild.vbox(2)
	box.add_child(_identity(character_id, str(captive.get("name", character_id)), str(captive.get("faction", "")), subtitle))
	var actions := UiBuild.hbox(6, box)
	var controls := {}
	if ours:
		var pay := RichButton.new()
		pay.text = "Céder %s" % _province_name(str(terms.get("province", ""))) if kind == "province" else "Payer comptant (%s)" % _pounds(ransom)
		TooltipHost.attach_plain(pay, "ransom_pay_full", {"body": "Paiement intégral : le captif rentre aussitôt." if kind != "province" else "La province exigée passe au geôlier ; le captif est libéré."})
		pay.pressed.connect(func() -> void: pay_ransom(character_id, 1))
		actions.add_child(pay)
		controls["pay"] = pay
		var plans: Array = captive.get("plans", [])
		if kind == "money" and not plans.is_empty():
			var plan := OptionButton.new()
			for entry in plans:
				var count := int(entry.get("installments", 0))
				plan.add_item("%d échéances de %s (total %s)" % [count, _pounds(int(entry.get("installment", 0))), _pounds(int(entry.get("total", 0)))], count)
			plan.tooltip_text = RuleValues.format("Échéances annuelles, total +{rule.ransom_installment_surcharge_percent} % ; la première est payée tout de suite et le captif rentre alors.")
			plan.size_flags_horizontal = Control.SIZE_EXPAND_FILL
			plan.fit_to_longest_item = false
			actions.add_child(plan)
			var pay_plan := RichButton.new()
			pay_plan.text = "Payer par échéances"
			pay_plan.pressed.connect(func() -> void: pay_ransom(character_id, plan.get_selected_id()))
			actions.add_child(pay_plan)
			controls["plan"] = plan
			controls["pay_plan"] = pay_plan
	else:
		var choice := OptionButton.new()
		choice.add_item("Rançon en argent (%s)" % _pounds(ransom))
		choice.set_item_metadata(0, {"kind": "money"})
		for province in captive.get("cedable_provinces", []):
			choice.add_item("Exiger %s" % _province_name(str(province)))
			choice.set_item_metadata(choice.item_count - 1, {"kind": "province", "province": str(province)})
		choice.add_item("Garder le prisonnier")
		choice.set_item_metadata(choice.item_count - 1, {"kind": "hold"})
		for index in choice.item_count:
			var meta: Dictionary = choice.get_item_metadata(index)
			if str(meta.get("kind", "")) == kind and str(meta.get("province", "")) == str(terms.get("province", "")):
				choice.select(index)
		choice.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		TooltipHost.attach_plain(choice, "ransom_cedable_province")
		actions.add_child(choice)
		var set_terms := RichButton.new()
		set_terms.text = "Fixer les termes"
		set_terms.pressed.connect(func() -> void: set_ransom_terms(character_id, choice.get_item_metadata(choice.selected)))
		actions.add_child(set_terms)
		var parole := RichButton.new()
		parole.text = "Libérer sur parole"
		TooltipHost.attach_plain(parole, "ransom_parole")
		parole.pressed.connect(func() -> void: release_on_parole(character_id))
		actions.add_child(parole)
		controls["terms"] = choice
		controls["set_terms"] = set_terms
		controls["parole"] = parole
	rows[character_id] = controls
	box.add_child(HSeparator.new())
	return box


func _debt_row(debt: Dictionary) -> Control:
	var character_id := str(debt.get("character", ""))
	var due := int(debt.get("next_due_turn", 0))
	var turn := int(_sim.call("get_turn")) if _sim != null and _sim.has_method("get_turn") else -1
	var when := "prochaine échéance au tour %d" % (due + 1)
	if turn >= 0:
		var seasons := due - turn
		when = "prochaine échéance cette saison" if seasons <= 0 else "prochaine échéance dans %s" % FrText.count(seasons, "saison")
	var subtitle := "Due à %s : reste %s, échéance %s · %s" % [
		_faction_name(str(debt.get("creditor", ""))), _pounds(int(debt.get("remaining", 0))),
		_pounds(int(debt.get("installment", 0))), when]
	var missed := int(debt.get("missed", 0))
	if missed > 0:
		subtitle += "\nÉchéances impayées : %d (dette +%s %% chacune)" % [missed, RuleValues.text("ransom_default_surcharge_percent")]
	return _identity(character_id, "Rançon de %s" % str(debt.get("name", character_id)), "", subtitle)


# --- Ordres (tous soumis à la simulation ; refus affichés en rouge) ----------------------------


func pay_ransom(character_id: String, installments: int) -> Dictionary:
	return _submit({"type": "pay_ransom", "character": character_id, "installments": installments})


func set_ransom_terms(character_id: String, terms: Dictionary) -> Dictionary:
	return _submit({"type": "set_ransom_terms", "character": character_id, "terms": terms})


func release_on_parole(character_id: String) -> Dictionary:
	return _submit({"type": "release_on_parole", "character": character_id})


func _submit(order: Dictionary) -> Dictionary:
	if _sim == null:
		_sim = _facade_sim()
	if _sim == null:
		return {}
	last_result = _sim.call("submit_order", order)
	var ok := bool(last_result.get("ok", false))
	if ok:
		refresh()
		ransoms_changed.emit()
	error_label.visible = not ok
	error_label.text = "Refusé : %s" % str(last_result.get("error", "?")) if not ok else ""
	return last_result


# --- Liens -------------------------------------------------------------------------------------


## Fiche du personnage : signal local, puis `character_selected` du premier ancêtre qui l'expose
## (`MapUI`, qui ouvre la fiche) — sans toucher à la carte.
func request_character(character_id: String) -> void:
	character_requested.emit(character_id)
	var node := get_parent()
	while node != null:
		if node.has_signal("character_selected"):
			node.emit_signal("character_selected", character_id)
			return
		node = node.get_parent()


## Encre des noms cliquables (le thème parchemin éclaircit les LinkButton).
static func style_link(link: LinkButton) -> void:
	link.add_theme_color_override("font_color", LINK_COLOR)
	link.add_theme_color_override("font_hover_color", RUBRIC_COLOR)
	link.add_theme_color_override("font_pressed_color", RUBRIC_COLOR)
	link.add_theme_color_override("font_focus_color", LINK_COLOR)


static func codex_entry_for(entity_id: String) -> String:
	var loop := Engine.get_main_loop() as SceneTree
	var store: Node = loop.root.get_node_or_null("/root/CodexStore") if loop != null else null
	return str(store.call("entry_for_entity", entity_id)) if store != null and entity_id != "" else ""


static func _root_node(path: String) -> Node:
	var loop := Engine.get_main_loop() as SceneTree
	return loop.root.get_node_or_null(path) if loop != null else null


static func _faction_name(faction_id: String) -> String:
	var facade := _root_node("/root/SimFacade")
	return str(facade.call("faction_short_name", faction_id)) if facade != null and faction_id != "" else faction_id


static func _faction_color(faction_id: String) -> Color:
	var facade := _root_node("/root/SimFacade")
	return facade.call("faction_color", faction_id) if facade != null and faction_id != "" else Color(0.5, 0.45, 0.35)


static func _province_name(province_id: String) -> String:
	var facade := _root_node("/root/SimFacade")
	var store: Object = facade.get("store") if facade != null else null
	var info: Dictionary = store.call("get_province", province_id) if store != null else {}
	return str(info.get("display_name", province_id))


static func _pounds(value: int) -> String:
	return "%s %s" % [Money.digits(value), RichTooltip.POUND]


func _facade_sim() -> Object:
	var facade := _root_node("/root/SimFacade")
	return facade.get("sim") if facade != null else null
