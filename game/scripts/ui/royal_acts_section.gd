class_name RoyalActsSection
extends PanelSection

## Onglet « Actes royaux » du panneau de cour (WH chars, ADR 0276 et 0277), construit en code :
## liste des actes ouverts à la faction (`CampaignSim.get_royal_acts`) avec coût en prestige et en
## livres, effets, durée, recharge ; bouton « Accomplir » (ordre `royal_act`) ; en bas, le
## recrutement d'un capitaine (`get_captain_info`, ordre `hire_captain`). Aucune règle ici :
## disponibilité, effets, plafond et refus viennent de `sim_campaign::royal_acts` / `captains`.

signal act_performed(act_id: String)
signal captain_hired

var acts_box: VBoxContainer
var captain_label: Label
var captain_button: Button
## Boutons « Accomplir » par id d'acte (tests).
var act_buttons: Dictionary = {}
var _faction: String = ""


func _init() -> void:
	super("RoyalActsSection", 6)
	acts_box = UiBuild.vbox(6)
	add_child(acts_box)
	add_child(HSeparator.new())
	captain_label = UiBuild.label("", UiType.size(UiType.CAPTION), MUTED_COLOR, true)
	add_child(captain_label)
	captain_button = UiBuild.button("Recruter un capitaine", func() -> void: request_captain())
	captain_button.name = "HireCaptainButton"
	add_child(captain_button)
	_add_error_label()


## Remplit l'onglet pour `faction` (`editable` faux : lecture seule).
func show_for(faction: String, editable: bool = true, sim: Object = null) -> void:
	_faction = faction
	_sim = _resolve_sim(sim)
	read_only = not editable
	for child in acts_box.get_children():
		child.queue_free()
	act_buttons.clear()
	if _sim == null or not _sim.has_method("get_royal_acts"):
		captain_label.text = "Actes royaux indisponibles."
		captain_button.hide()
		return
	var acts: Array = _sim.call("get_royal_acts", faction)
	for act in acts:
		acts_box.add_child(_make_act_card(act))
	_refresh_captain()


func _make_act_card(act: Dictionary) -> Control:
	var id := str(act.get("id", ""))
	var card := PanelContainer.new()
	card.name = "Act_%s" % id
	card.add_theme_stylebox_override("panel", HudStyle.card_box(HudStyle.PARCHMENT_LIGHT, HudStyle.INK_SOFT, 1))
	var box := UiBuild.vbox(3)
	card.add_child(box)
	var head := UiBuild.hbox(8)
	box.add_child(head)
	var title := UiBuild.label(str(act.get("name", id)), UiType.size(UiType.BODY))
	title.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	head.add_child(title)
	var button := UiBuild.button("Accomplir", func() -> void: request_act(id))
	button.name = "PerformButton"
	button.disabled = read_only or not bool(act.get("available", false))
	button.tooltip_text = "" if bool(act.get("available", false)) else str(act.get("reason", ""))
	head.add_child(button)
	act_buttons[id] = button
	box.add_child(UiBuild.label(_cost_text(act), UiType.size(UiType.CAPTION), MUTED_COLOR, true))
	var description := _rich_text(UiType.size(UiType.CAPTION), 220.0, true)
	description.text = str(act.get("description", ""))
	box.add_child(description)
	var effects_block: String = RichTooltip._effects_block(act.get("effects", []))
	if effects_block != "":
		var effects := _rich_text(UiType.size(UiType.CAPTION), 220.0)
		effects.text = effects_block
		box.add_child(effects)
	var status := _status_text(act)
	if status != "":
		box.add_child(UiBuild.label(status, UiType.size(UiType.CAPTION), HudStyle.RUBRIC, true))
	return card


static func _cost_text(act: Dictionary) -> String:
	var parts := PackedStringArray()
	if int(act.get("cost_prestige", 0)) > 0:
		parts.append("%d prestige" % int(act["cost_prestige"]))
	if int(act.get("cost_livres", 0)) > 0:
		parts.append("%s %s" % [Money.digits(int(act["cost_livres"])), RichTooltip.POUND])
	var gains := PackedStringArray()
	if int(act.get("gain_prestige", 0)) > 0:
		gains.append("+%d prestige" % int(act["gain_prestige"]))
	if int(act.get("gain_piety", 0)) > 0:
		gains.append("+%d piété" % int(act["gain_piety"]))
	var text := "Coût : %s" % (" + ".join(parts) if not parts.is_empty() else "gratuit")
	if not gains.is_empty():
		text += " — gain : %s" % ", ".join(gains)
	text += " — effets %d tour(s), recharge %d tours" % [int(act.get("duration_turns", 0)), int(act.get("cooldown_turns", 0))]
	return text


static func _status_text(act: Dictionary) -> String:
	var active := int(act.get("active_left", 0))
	var cooldown := int(act.get("cooldown_left", 0))
	if active > 0:
		return "En vigueur : encore %d tour(s) ; refaisable dans %d." % [active, cooldown]
	if cooldown > 0:
		return "Recharge : encore %d tour(s)." % cooldown
	if not bool(act.get("available", false)):
		return str(act.get("reason", ""))
	return ""


func _refresh_captain() -> void:
	var info: Dictionary = _sim.call("get_captain_info", _faction) if _sim.has_method("get_captain_info") else {}
	captain_button.visible = not info.is_empty()
	if info.is_empty():
		captain_label.text = ""
		return
	captain_label.text = "Capitaines recrutés : %d / %d (le prestige du souverain élève le plafond). Un chevalier banneret commande une armée sans chef : %s %s." % [
		int(info.get("count", 0)), int(info.get("cap", 0)), Money.digits(int(info.get("cost", 0))), RichTooltip.POUND]
	captain_button.disabled = read_only or not bool(info.get("can_hire", false))
	captain_button.tooltip_text = str(info.get("reason", ""))


## Ordre `royal_act` ; en cas de refus, message de la simulation affiché en rouge.
func request_act(act_id: String) -> Dictionary:
	if _sim == null or read_only:
		return {}
	if _submit({"type": "royal_act", "act": act_id}, func() -> void: show_for(_faction, not read_only, _sim)):
		act_performed.emit(act_id)
	return last_result


## Ordre `hire_captain` dans la ville de la capitale.
func request_captain() -> Dictionary:
	if _sim == null or read_only:
		return {}
	var info: Dictionary = _sim.call("get_captain_info", _faction)
	if _submit({"type": "hire_captain", "settlement": str(info.get("settlement", ""))}, func() -> void: show_for(_faction, not read_only, _sim)):
		captain_hired.emit()
	return last_result
