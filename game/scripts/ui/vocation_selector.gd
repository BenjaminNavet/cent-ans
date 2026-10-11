class_name VocationSelector
extends HBoxContainer

## RG10 (ADR 0344) : sélecteur de vocation d'une colonie (en-tête du panneau Bâtiments).
## Un bouton « Vocation : Agricole (1/2) » dont l'infobulle donne les effets et le palier de
## cohérence ; un clic ouvre un menu : une entrée par vocation (coût, adéquation avec les
## bâtiments actuels, raison du refus). Toute la règle est dans le cœur : le widget lit
## `SimFacade.get_settlement_vocation` / `get_vocation_options` et envoie l'ordre
## `SimFacade.set_settlement_vocation`.

signal vocation_changed(settlement_id: String, vocation_id: String)

var settlement_id: String = ""
var editable := false
var view: Dictionary = {}
var options: Array = []
var button: MenuButton
var _last_error := ""


func _init() -> void:
	name = "VocationSelector"
	alignment = BoxContainer.ALIGNMENT_CENTER
	size_flags_horizontal = Control.SIZE_EXPAND_FILL
	button = MenuButton.new()
	button.name = "VocationButton"
	button.flat = false
	button.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	add_child(button)
	button.get_popup().id_pressed.connect(_on_item_pressed)
	hide()


## Lit le cœur pour `id` ; `can_edit` : colonie du joueur (sinon lecture seule).
func set_settlement(id: String, can_edit: bool) -> void:
	settlement_id = id
	editable = can_edit
	_last_error = ""
	refresh()


func refresh() -> void:
	view = _facade().call("get_settlement_vocation", settlement_id) if settlement_id != "" else {}
	options = _facade().call("get_vocation_options", settlement_id) if settlement_id != "" else []
	visible = not options.is_empty()
	if not visible:
		return
	button.text = button_text(view)
	button.tooltip_text = tooltip_text_for(view, options)
	button.disabled = not editable
	var popup := button.get_popup()
	popup.clear()
	for index in options.size():
		var option: Dictionary = options[index]
		popup.add_item(option_label(option), index)
		popup.set_item_disabled(index, not bool(option.get("available", false)))
		popup.set_item_tooltip(index, option_tooltip(option))


## Choisit la vocation `vocation_id` (ordre au cœur) ; `{ok, error}`.
func choose(vocation_id: String) -> Dictionary:
	var result: Dictionary = _facade().call("set_settlement_vocation", settlement_id, vocation_id)
	_last_error = "" if bool(result.get("ok", false)) else str(result.get("error", ""))
	if bool(result.get("ok", false)):
		vocation_changed.emit(settlement_id, vocation_id)
	refresh()
	return result


## L'autoload `SimFacade` (absent à la compilation d'un test, d'où la recherche à l'exécution).
func _facade() -> Node:
	return (Engine.get_main_loop() as SceneTree).root.get_node("SimFacade")


func last_error() -> String:
	return _last_error


func _on_item_pressed(index: int) -> void:
	if index >= 0 and index < options.size():
		choose(str((options[index] as Dictionary).get("id", "")))


# --- Textes (statiques : testables sans scène) --------------------------------------------------

## « Vocation : aucune » ou « Vocation : Agricole (paliers 1/2) ».
static func button_text(vocation_view: Dictionary) -> String:
	if str(vocation_view.get("vocation", "")) == "":
		return "Vocation : aucune"
	var tiers: Array = vocation_view.get("tiers", [])
	return "Vocation : %s (%d/%d)" % [str(vocation_view.get("name", "")), int(vocation_view.get("reached_tiers", 0)), tiers.size()]


## Infobulle : effets, palier de cohérence, délai de changement, coût du changement.
static func tooltip_text_for(vocation_view: Dictionary, vocation_options: Array = []) -> String:
	var lines := PackedStringArray()
	if str(vocation_view.get("vocation", "")) == "":
		lines.append("Aucune vocation : aucun bonus ni surcoût.")
		lines.append("Spécialiser la colonie réduit le coût de ses bâtiments de vocation et renforce leurs effets.")
	else:
		lines.append("%s — %s" % [str(vocation_view.get("name", "")), str(vocation_view.get("description", ""))])
		lines.append("Bâtiments de la vocation : effets %s, coût %s ; autres bâtiments : coût %s." % [
			_signed_percent(float(vocation_view.get("effect_bonus_percent", 0))),
			_signed_percent(-float(vocation_view.get("cost_reduction_percent", 0))),
			_signed_percent(float(vocation_view.get("off_vocation_surcharge_percent", 0)))])
		lines.append("Cohérence : %d bâtiment(s) de la vocation." % int(vocation_view.get("count", 0)))
		for tier in vocation_view.get("tiers", []):
			var mark := "✔" if bool((tier as Dictionary).get("reached", false)) else "·"
			lines.append("%s %s (%d bâtiments) : %s" % [mark, str(tier.get("label", "")), int(tier.get("min_buildings", 0)), effects_text(tier.get("effects", []))])
		var next_tier: Variant = vocation_view.get("next_tier")
		if next_tier is Dictionary:
			var missing := int((next_tier as Dictionary).get("min_buildings", 0)) - int(vocation_view.get("count", 0))
			lines.append("Prochain palier : encore %d bâtiment(s)." % maxi(missing, 1))
		var active: Array = vocation_view.get("active_effects", [])
		if not active.is_empty():
			lines.append("Effets actuels : " + effects_text(active))
	var until := _int(vocation_view.get("locked_until"))
	if until > 0:
		lines.append("Changement verrouillé jusqu'au tour %d." % until)
	elif not vocation_options.is_empty():
		var option: Dictionary = vocation_options[0]
		lines.append("Changer : %s l.t., puis verrouillé %d tours." % [Money.digits(int(option.get("cost", 0))), int(option.get("lock_turns", 0))])
	return "\n".join(lines)


## « Marchande — 350 l.t. — 3 bât. concordants ».
static func option_label(option: Dictionary) -> String:
	var text := str(option.get("name", ""))
	if bool(option.get("current", false)):
		return text + " (actuelle)"
	text += " — %s l.t." % Money.digits(int(option.get("cost", 0)))
	var fit := int(option.get("fit", 0))
	if fit > 0:
		text += " — %d bât. concordant%s" % [fit, "s" if fit > 1 else ""]
	return text


static func option_tooltip(option: Dictionary) -> String:
	var lines := PackedStringArray()
	lines.append(str(option.get("description", "")))
	lines.append("Effets des bâtiments concernés %s, coût %s ; hors vocation coût %s." % [
		_signed_percent(float(option.get("effect_bonus_percent", 0))),
		_signed_percent(-float(option.get("cost_reduction_percent", 0))),
		_signed_percent(float(option.get("off_vocation_surcharge_percent", 0)))])
	for tier in option.get("tiers", []):
		lines.append("%d bâtiments — %s : %s" % [int((tier as Dictionary).get("min_buildings", 0)), str(tier.get("label", "")), effects_text(tier.get("effects", []))])
	var preview: Array = option.get("preview_effects", [])
	if not preview.is_empty():
		lines.append("Avec vos bâtiments actuels : " + effects_text(preview))
	var reason := _text(option.get("reason"))
	if reason != "":
		lines.append("Impossible : " + reason)
	return "\n".join(lines)


static func effects_text(effects: Array) -> String:
	var parts := PackedStringArray()
	for effect in effects:
		parts.append(RichTooltip.effect_text(effect))
	return ", ".join(parts)


static func _signed_percent(value: float) -> String:
	var rounded := roundf(value * 10.0) / 10.0
	return "%s%s %%" % ["+" if rounded >= 0 else "−", str(absf(rounded)).trim_suffix(".0")]


## Les `Option` du cœur arrivent en `null` : 0 / "" dans ce cas.
static func _int(value: Variant) -> int:
	return int(value) if value is int or value is float else 0


static func _text(value: Variant) -> String:
	return str(value) if value is String else ""
