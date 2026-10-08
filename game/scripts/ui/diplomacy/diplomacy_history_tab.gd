class_name DiplomacyHistoryTab
extends DiplomacyView

## Onglet « Traités » : historique des traités du joueur avec la faction choisie.


func _init() -> void:
	super("HistoryTab", 6)


func _render() -> void:
	UiBuild.clear_children(self)
	if sim == null or not sim.has_method("get_treaty_history"):
		return
	var shown := 0
	for record in sim.call("get_treaty_history", player_faction):
		if str(record.get("with", "")) != faction_id:
			continue
		var accepted := bool(record.get("accepted", false))
		var head := "%s — %s, %s" % [record.get("date", ""), record.get("with_name", ""), "signé" if accepted else "refusé"]
		var head_label := _label(head, UiType.BODY, HudStyle.INK if accepted else HudStyle.RUBRIC)
		var wax := FaUi.seal_rect("treaty", float(TREATY_SEAL_SIZE)) if accepted else null
		if wax != null:  # FA5 : un traité signé porte son sceau
			var head_row := UiBuild.hbox(6)
			head_row.add_child(wax)
			head_row.add_child(head_label)
			add_child(head_row)
		else:
			add_child(head_label)
		var body := _label(str(record.get("text", "")), UiType.CAPTION, HudStyle.INK_SOFT)
		body.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		add_child(body)
		shown += 1
	if shown == 0:
		add_child(_label("Aucun traité avec cette faction.", UiType.BODY, HudStyle.INK_FADED))
