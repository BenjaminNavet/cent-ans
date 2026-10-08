class_name WarDeclarationDialog
extends ConfirmPanel

## Confirmation « Déclarer la guerre ? » quand le joueur attaque, sur la carte de campagne, une
## armée ou une place d'une faction avec laquelle il n'est pas en guerre. Les conséquences
## (motif, réputation, prestige) viennent de `CampaignSim.evaluate_proposal` ; aucune règle ici.
## FE6 : la chaîne « Qui peut entrer en guerre » (`EscalationPreview`) suit les conséquences.

var _consequences: RichTextLabel
var escalation: EscalationPreview


func _init() -> void:
	super("Déclarer la guerre et attaquer", "Renoncer", 520)
	name = "WarDeclarationDialog"
	text_label.hide()
	_consequences = RichTextLabel.new()
	_consequences.bbcode_enabled = true
	_consequences.fit_content = true
	_consequences.scroll_active = false
	_consequences.custom_minimum_size = Vector2(480, 0)
	body.add_child(_consequences)
	escalation = EscalationPreview.new()
	escalation.hide()
	body.add_child(escalation)


## Ouvre la confirmation contre `faction_name` ; `target_label` nomme ce qui est attaqué,
## `status` la relation actuelle (`get_diplomacy`) pour signaler un lien vassalique rompu.
func ask(sim: Object, faction_id: String, faction_name: String, target_label: String, status := "") -> void:
	var lines := PackedStringArray(["Attaquer %s nous met en guerre avec %s." % [target_label, faction_name]])
	if status == "vassal":
		lines.append("%s est notre vassal : la guerre rompt son hommage." % faction_name)
	elif status == "suzerain":
		lines.append("%s est notre suzerain : la guerre rompt notre hommage." % faction_name)
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
	open("Déclarer la guerre à %s ?" % faction_name, "")

