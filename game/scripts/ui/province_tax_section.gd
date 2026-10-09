class_name ProvinceTaxSection
extends ProvinceSection

## Section « Impôt de la province » du panneau de province (WH econ) : taux en vigueur (celui de
## la faction ou un taux propre à la province) et trois boutons Bas / Normal / Haut, plus
## « Taux de la faction » pour revenir au taux commun. Ordre `set_province_tax` ; aucune règle
## ici : `CampaignSim.get_province_tax` donne l'état, le cœur valide l'ordre.

signal tax_changed(province_id: String, rate: String)

const RATE_LABELS := {"low": "Bas", "normal": "Normal", "high": "Haut"}

var summary_label: Label
var rate_buttons: Dictionary = {}
var reset_button: Button


func _init() -> void:
	super("ProvinceTaxSection", 4)
	var header := UiBuild.label("Impôt de la province", UiType.size(UiType.BODY), null, false, 0.0, self)
	TooltipHost.attach_plain(header, "province_tax")
	header.mouse_filter = Control.MOUSE_FILTER_PASS
	summary_label = UiBuild.label("", UiType.size(UiType.CAPTION), MUTED_COLOR, true, 0.0, self)
	var row := UiBuild.hbox(4, self)
	for rate in ["low", "normal", "high"]:
		var button := RichButton.new()
		button.text = str(RATE_LABELS[rate])
		button.toggle_mode = true
		button.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		button.pressed.connect(request_rate.bind(rate))
		row.add_child(button)
		rate_buttons[rate] = button
	reset_button = RichButton.new()
	reset_button.text = "Taux de la faction"
	reset_button.pressed.connect(request_rate.bind(""))
	add_child(reset_button)
	_add_error_label()


func _render(_data: Dictionary) -> void:
	error_label.hide()
	if _sim == null or not _sim.has_method("get_province_tax") or province_id == "":
		hide()
		return
	var state: Dictionary = _sim.call("get_province_tax", province_id)
	if state.is_empty():
		hide()
		return
	show()
	var rate := str(state.get("rate", "normal"))
	var own := bool(state.get("own", false))
	var faction_rate := str(state.get("faction_rate", "normal"))
	summary_label.text = "Taux en vigueur : %s (%s)" % [RATE_LABELS.get(rate, rate), "propre à la province" if own else "taux de la faction"]
	for key in rate_buttons:
		var button: Button = rate_buttons[key]
		button.set_pressed_no_signal(key == rate)
		button.disabled = not is_player_owner
	reset_button.visible = is_player_owner and own
	reset_button.tooltip_text = "Revenir au taux de la faction (%s)." % RATE_LABELS.get(faction_rate, faction_rate)


## Ordre `set_province_tax` ; `rate` vide : retour au taux de la faction. Refus affiché en rouge.
func request_rate(rate: String) -> Dictionary:
	if _sim == null or province_id == "":
		return {}
	var order := {"type": "set_province_tax", "province": province_id, "rate": rate if rate != "" else null}
	_submit(order, func() -> void: show_for(province_id, is_player_owner, _sim))
	if bool(last_result.get("ok", false)):
		tax_changed.emit(province_id, rate)
	return last_result
