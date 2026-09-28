class_name WarDeclarationDialog
extends PanelContainer

## Confirmation « Déclarer la guerre ? » quand le joueur attaque, sur la carte de campagne, une
## armée ou une place d'une faction avec laquelle il n'est pas en guerre. Les conséquences
## (motif, réputation, prestige) viennent de `CampaignSim.evaluate_proposal` ; aucune règle ici.
## FE6 : la chaîne « Qui peut entrer en guerre » (`EscalationPreview`) suit les conséquences.

signal confirmed
signal cancelled

var _title: Label
var _consequences: RichTextLabel
var _confirm_button: Button
var escalation: EscalationPreview


func _init() -> void:
	name = "WarDeclarationDialog"
	PanelStack.set_tier(self, PanelStack.Tier.MODAL, true)
	theme = load("res://scenes/ui/parchment_theme.tres")
	set_anchors_preset(Control.PRESET_CENTER_TOP)
	offset_left = -260
	offset_right = 260
	offset_top = 110
	mouse_filter = Control.MOUSE_FILTER_STOP
	var box := VBoxContainer.new()
	box.add_theme_constant_override("separation", 10)
	add_child(box)
	_title = HudStyle.label("", HudStyle.FONT_TITLE, HudStyle.RUBRIC)
	_title.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	_title.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	box.add_child(_title)
	_consequences = RichTextLabel.new()
	_consequences.bbcode_enabled = true
	_consequences.fit_content = true
	_consequences.scroll_active = false
	_consequences.custom_minimum_size = Vector2(480, 0)
	box.add_child(_consequences)
	escalation = EscalationPreview.new()
	escalation.hide()
	box.add_child(escalation)
	var row := HBoxContainer.new()
	row.alignment = BoxContainer.ALIGNMENT_CENTER
	row.add_theme_constant_override("separation", 12)
	box.add_child(row)
	var cancel_button := Button.new()
	cancel_button.text = "Renoncer"
	cancel_button.pressed.connect(_on_cancel)
	row.add_child(cancel_button)
	_confirm_button = Button.new()
	_confirm_button.text = "Déclarer la guerre et attaquer"
	_confirm_button.pressed.connect(_on_confirm)
	row.add_child(_confirm_button)
	hide()


## Ouvre la confirmation contre `faction_name` ; `target_label` nomme ce qui est attaqué.
func ask(sim: Object, faction_id: String, faction_name: String, target_label: String) -> void:
	_title.text = "Déclarer la guerre à %s ?" % faction_name
	var lines := PackedStringArray(["Attaquer %s nous met en guerre avec %s." % [target_label, faction_name]])
	if sim != null and sim.has_method("evaluate_proposal"):
		var verdict: Dictionary = sim.call("evaluate_proposal", {"type": "declare_war", "target": faction_id})
		var parts := PackedStringArray()
		for reason in verdict.get("reasons", []):
			var value := int(reason.get("value", 0))
			var text := CodexText.format(str(reason.get("text", "")), true)
			parts.append(("[color=%s]%+d[/color] %s" % ["#2a6a2a" if value >= 0 else "#8b1a1a", value, text]) if value != 0 else text)
		if not parts.is_empty():
			lines.append("[b]Conséquences :[/b] " + " · ".join(parts))
	_consequences.text = "\n".join(lines)
	var attacker := str(sim.call("get_player_faction")) if sim != null and sim.has_method("get_player_faction") else ""
	escalation.show_for(sim, attacker, faction_id)
	show()
	_confirm_button.grab_focus()


func _on_confirm() -> void:
	hide()
	confirmed.emit()


func _on_cancel() -> void:
	hide()
	cancelled.emit()


func _unhandled_input(event: InputEvent) -> void:
	if visible and event.is_action_pressed("ui_cancel"):
		_on_cancel()
		get_viewport().set_input_as_handled()
