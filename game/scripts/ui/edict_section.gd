class_name EdictSection
extends ProvinceChoiceSection

## Section « Édit régional » du panneau de province (onglet Ville), construite en code :
## édit actif (icône, nom, description), bandeau si un changement est en attente (délai avant
## effet), sélecteur des édits (`get_edict_options`) avec une infobulle riche par édit, ordre
## `set_edict` et message de refus. Lecture seule hors des provinces du joueur — et hors des
## provinces qui ne sont pas entièrement tenues par leur contrôleur (un édit régional suppose
## l'autorité complète). Aucune règle ici : disponibilité, effets et refus viennent de
## `CampaignSim` (`sim_campaign::edicts`).

signal edict_changed(province_id: String, edict_id: String)

var pending_label: Label


func _init() -> void:
	super("EdictSection", "Édit régional", "Changer d'édit", "get_province_edict", "get_edict_options", "edict", "cat_building", "building")
	pending_label = UiBuild.label("", UiType.size(UiType.CAPTION), MUTED_COLOR, true)
	pending_label.hide()
	add_child(pending_label)
	move_child(pending_label, current_chip.get_parent().get_index() + 1)


func _refresh_extra(state: Dictionary, _current: Dictionary) -> void:
	var pending := bool(state.get("pending", false))
	pending_label.visible = pending and is_player_owner
	if pending:
		pending_label.text = "« %s » entre en vigueur dans %d tour(s) ; l'édit actuel s'applique en attendant." % [str(state.get("pending_name", "")), int(state.get("turns_left", 0))]
	TooltipHost.attach_plain(choose_button, "choose_edict")


func _option_text(option: Dictionary) -> String:
	var text := super(option)
	if bool(option.get("current", false)):
		text += "  (actuel)" if bool(option.get("active", false)) else "  (en attente)"
	return text


func _option_note(option: Dictionary) -> String:
	return "" if bool(option.get("available", false)) or bool(option.get("current", false)) else str(option.get("reason", "indisponible"))


## Infobulle riche d'un édit : effets au format de `RichTooltip._effects_block` (mêmes rangées
## que les bâtiments/régimes), plus le délai s'il n'est pas nul.
func _option_tooltip(option: Dictionary) -> String:
	if option.is_empty():
		return ""
	var lines: PackedStringArray = []
	lines.append("[b]%s[/b]" % RichTooltip.entity_name(str(option.get("id", "")), str(option.get("name", ""))))
	var delay := int(option.get("delay_turns", 0))
	if delay > 0:
		lines.append("Délai avant effet : %d tour(s)" % delay)
	var effects_block: String = RichTooltip._effects_block(option.get("effects", []))
	if effects_block != "":
		lines.append(effects_block)
	return "\n".join(lines)


func _on_option_chosen(id: String) -> void:
	request_edict(id)


## Ordre `set_edict` ; en cas de refus, message de la simulation affiché en rouge.
func request_edict(edict_id: String) -> Dictionary:
	if _submit_choice({"type": "set_edict", "province": province_id, "edict": edict_id}).is_empty():
		return {}
	if bool(last_result.get("ok", false)):
		edict_changed.emit(province_id, edict_id)
	return last_result
