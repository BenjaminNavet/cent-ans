class_name EncounterWindow
extends ChronicleWindow

## Lot CV3-4 : fenêtre de choix d'une rencontre sur la carte, au gabarit de la chronique
## (`ChronicleWindow` : rubrique, titre, lieu, texte d'époque, un bouton par option avec ses
## effets en petit, « Plus tard »). Une option que le cœur refuse est grisée avec sa raison.
## Aucune règle : la rencontre vient de `CampaignSim.get_pending_encounters()` (`{site,
## encounter, army, army_name, title, text, province_name, options[{index, label, available,
## reason, effects_text, outcome, default}]}`), le choix repart par `encounter_option_chosen`.

signal encounter_option_chosen(army_id: String, site: int, option: int)

const ENCOUNTER_ART_DIR := "res://assets/encounters/"

var _encounter: Dictionary = {}


func _ready() -> void:
	name = "EncounterWindow"
	theme_type_variation = &"IlluminatedPanel"
	super._ready()
	for node in find_children("*", "Button", true, false):
		var button := node as Button
		if button.text == "Plus tard":
			RichTooltip.attach_plain(button, "decision_default_choice_turn")
		elif button.text == "×":
			RichTooltip.attach_plain(button, "close_encounter_pending")


## Affiche `encounter` (un élément de `get_pending_encounters`) ; `queue_size` en attente.
func show_encounter(encounter: Dictionary, queue_size: int) -> void:
	_encounter = encounter.duplicate(true)
	_decision_id = int(encounter.get("site", -1))
	_kind_label.text = "✠ Rencontre en chemin"
	_title_label.text = str(encounter.get("title", ""))
	var art_path := ENCOUNTER_ART_DIR + str(encounter.get("encounter", "")) + ".jpg"
	_art.texture = PortraitLoader.load_texture(art_path) if ResourceLoader.exists(art_path) else null
	_art.visible = _art.texture != null
	var meta := PackedStringArray()
	var army_name := str(encounter.get("army_name", ""))
	if army_name != "":
		meta.append(army_name)
	var province_name := str(encounter.get("province_name", ""))
	if province_name != "":
		meta.append(province_name)
	meta.append("à décider avant la fin du tour")
	_meta_label.text = " — ".join(meta)
	_text_label.text = "[i]%s[/i]" % CodexText.format(str(encounter.get("text", "")), true)
	for child in _options_box.get_children():
		_options_box.remove_child(child)
		child.queue_free()
	for option in encounter.get("options", []):
		_options_box.add_child(_encounter_option_row(option))
	_queue_label.text = "Rencontre 1 sur %d" % queue_size if queue_size > 1 else ""
	show()
	reset_size()
	_center_on_screen()
	call_deferred("_fit")


## Armée et site de la rencontre affichée (`{}` si fermée).
func current_encounter() -> Dictionary:
	return _encounter if visible else {}


## Boutons d'option, dans l'ordre (tests).
func option_buttons() -> Array[Button]:
	var buttons: Array[Button] = []
	for row in _options_box.get_children():
		for child in row.get_children():
			if child is Button:
				buttons.append(child)
	return buttons


func _encounter_option_row(option: Dictionary) -> Control:
	var row := VBoxContainer.new()
	row.add_theme_constant_override("separation", 1)
	var button := Button.new()
	button.name = "Option_%d" % int(option.get("index", 0))
	var text := str(option.get("label", ""))
	if bool(option.get("default", false)):
		text += "  (par défaut)"
	button.text = text
	button.add_theme_font_size_override("font_size", 17)
	button.alignment = HORIZONTAL_ALIGNMENT_LEFT
	var available := bool(option.get("available", true))
	var reason := str(option.get("reason", ""))
	button.disabled = not available
	var effects := str(option.get("effects_text", ""))
	var tip := effects if effects != "" else "Sans effet notable."
	if not available and reason != "":
		tip = "Impossible : %s\n%s" % [reason, tip]
	button.tooltip_text = tip
	var index := int(option.get("index", 0))
	button.pressed.connect(func() -> void:
		encounter_option_chosen.emit(str(_encounter.get("army", "")), int(_encounter.get("site", -1)), index))
	row.add_child(button)
	var lines := PackedStringArray()
	if effects != "":
		lines.append(effects.replace("\n", " · "))
	if not available and reason != "":
		lines.append("Impossible : " + reason)
	if not lines.is_empty():
		var summary := Label.new()
		summary.text = "   " + " — ".join(lines)
		summary.add_theme_font_size_override("font_size", 13)
		summary.add_theme_color_override("font_color", RUBRIC if not available else FADED_INK)
		summary.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		summary.custom_minimum_size = Vector2(580, 0)
		row.add_child(summary)
	return row
