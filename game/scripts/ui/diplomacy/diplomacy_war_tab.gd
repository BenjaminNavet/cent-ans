class_name DiplomacyWarTab
extends DiplomacyView

## Onglet « Guerre » : casus belli, prétentions, score et fatigue de guerre, buts de guerre
## (`get_war_summary`).


func _init() -> void:
	super("WarTab", 6)


func _render() -> void:
	UiBuild.clear_children(self)
	var status := str(entry["status"])
	if str(entry.get("casus_belli", "")) != "":
		add_child(_label("Casus belli : %s" % entry["casus_belli"], UiType.BODY, HudStyle.INK))
	for claim in entry.get("claims", []):
		var claim_label := _label("Prétention : %s" % claim, UiType.BODY, HudStyle.INK_SOFT)
		claim_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		add_child(claim_label)
	if status != "war" or not sim.has_method("get_war_summary"):
		add_child(_label("Pas de guerre en cours avec cette faction.", UiType.BODY, HudStyle.INK_FADED))
		return
	var summary: Dictionary = sim.call("get_war_summary", faction_id)
	var score := int(summary.get("war_score", 0))
	add_child(_section("Score de guerre : %+d" % score))
	var bar := ProgressBar.new()
	bar.min_value = -100
	bar.max_value = 100
	bar.value = score
	bar.show_percentage = false
	bar.custom_minimum_size = Vector2(0, 16)
	var fill := StyleBoxFlat.new()
	fill.bg_color = HudStyle.GOOD if score >= 0 else HudStyle.POOR
	bar.add_theme_stylebox_override("fill", fill)
	add_child(bar)
	add_child(_label("Batailles, sièges et provinces occupées remplissent le score ; les buts de guerre tenus comptent double. Plus il est haut, plus l'ennemi cédera de terres. À la paix, seules les places cédées par le traité deviennent vôtres ; les autres places occupées sont rendues.", UiType.CAPTION, HudStyle.INK_FADED))
	(get_child(get_child_count() - 1) as Label).autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	add_child(_label("Fatigue de guerre : nous %d/100, eux %d/100" % [int(summary.get("weariness_ours", 0)), int(summary.get("weariness_theirs", 0))], UiType.BODY, HudStyle.INK))
	for pair in [["Nos buts de guerre", "goals_ours"], ["Leurs buts de guerre", "goals_theirs"]]:
		add_child(_label(str(pair[0]), UiType.HEADING, HudStyle.RUBRIC))
		var goals: Array = summary.get(pair[1], [])
		if goals.is_empty():
			add_child(_label("—", UiType.BODY, HudStyle.INK_FADED))
		for goal in goals:
			var held := bool(goal.get("held", false))
			add_child(_label("%s %s%s" % ["★" if held else "☆", goal["name"], " (tenue)" if held else ""], UiType.BODY, HudStyle.INK))
