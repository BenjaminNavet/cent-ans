class_name TableSection
extends ProvinceChoiceSection

## H9 — section « La Table » du panneau de province (onglet Ville), construite en code :
## régime actuel (icône, nom, coût par saison, description aux mots du Codex cliquables),
## bandeau « Carême » au printemps, sélecteur des régimes (`get_diet_options`) avec une
## infobulle riche par régime (`RichTooltip.diet`), ordre `set_diet` et message de refus.
## Lecture seule hors des provinces du joueur. Aucune règle ici : disponibilité, coûts,
## raisons et refus viennent de `CampaignSim` (`docs/design/h3-h4-api.md` § 5).

signal diet_changed(province_id: String, diet_id: String)

const LENT_TEXT := "[b]Carême[/b] — quarante jours de [[cdx_careme|jeûne]] avant Pâques : une table de viande ou de laitages coûte {rule.lent_piety_penalty} de piété au souverain et fâche le clergé de la province (+{rule.lent_clergy_unrest} de mécontentement) ; le poisson de carême rapporte +{rule.lent_fish_piety} de piété."

var lent_banner: PanelContainer
var lent_text: RichTextLabel
var cost_label: Label
var changed_label: Label
var _changed_this_turn := false


func _init() -> void:
	super("TableSection", "La Table", "Changer de régime", "get_province_diet", "get_diet_options", "diet", "cat_resource", "resource")
	lent_banner = PanelContainer.new()
	var style := StyleBoxFlat.new()
	style.bg_color = Color(0.86, 0.80, 0.66)
	style.border_color = Color(0.45, 0.12, 0.08)
	style.border_width_left = 4
	style.set_content_margin_all(6)
	lent_banner.add_theme_stylebox_override("panel", style)
	lent_text = _rich_text(13, 160.0, true)
	lent_banner.add_child(lent_text)
	lent_banner.hide()
	add_child(lent_banner)
	move_child(lent_banner, 1)
	cost_label = UiBuild.label("", UiType.size(UiType.CAPTION), null, false, 0.0, current_chip.get_parent())
	changed_label = UiBuild.label("Régime déjà changé ce tour-ci (effet à la fin du tour).", UiType.size(UiType.CAPTION), MUTED_COLOR, true)
	changed_label.hide()
	add_child(changed_label)
	move_child(changed_label, current_chip.get_parent().get_index() + 1)


func _refresh_extra(state: Dictionary, _current: Dictionary) -> void:
	var cost := int(state.get("cost", 0))
	cost_label.text = "%s %s / saison" % [Money.digits(cost), RichTooltip.POUND] if cost > 0 else "gratuit"
	_changed_this_turn = bool(state.get("changed_this_turn", false))
	changed_label.visible = _changed_this_turn and is_player_owner
	var lent := bool(_sim.call("is_lent")) if _sim.has_method("is_lent") else false
	lent_banner.visible = lent
	if lent:
		lent_text.text = CodexText.format(RuleValues.format(LENT_TEXT))
	choose_button.disabled = _changed_this_turn
	TooltipHost.attach_plain(choose_button, "choose_diet", {"body": "Un seul changement par province et par tour." if _changed_this_turn else "Choisir la table de la province (effet à la fin du tour)."})


func _option_text(option: Dictionary) -> String:
	var cost := int(option.get("cost", 0))
	var text := "%s — %s" % [str(option.get("name", option.get("id", ""))), "%s %s" % [Money.digits(cost), RichTooltip.POUND] if cost > 0 else "gratuit"]
	return text + "  (actuel)" if bool(option.get("current", false)) else text


func _option_tooltip(option: Dictionary) -> String:
	return RichTooltip.diet(option) if not option.is_empty() else ""


func _option_locked(option: Dictionary) -> bool:
	return bool(option.get("current", false)) or _changed_this_turn


func _on_option_chosen(id: String) -> void:
	request_diet(id)


## Ordre `set_diet` ; en cas de refus, message de la simulation affiché en rouge.
func request_diet(diet_id: String) -> Dictionary:
	if _submit_choice({"type": "set_diet", "province": province_id, "diet": diet_id}).is_empty():
		return {}
	if bool(last_result.get("ok", false)):
		diet_changed.emit(province_id, diet_id)
	return last_result
