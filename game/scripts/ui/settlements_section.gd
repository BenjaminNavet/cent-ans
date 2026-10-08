class_name SettlementsSection
extends ProvinceSection

## Onglet « Colonies » du panneau de province (C5) : un bouton par place de la province (nom,
## genre, tenant, statut d'occupation, garnison, siège) qui ouvre la fiche de la colonie.
## `data` : `rows` (lignes de `SettlementController`), `label_of` (nom d'une faction) et `help`
## (RJ-c : phrase d'aide et places tenues, vide si la possession est inconnue).

signal settlement_requested(settlement_id: String)


func _init() -> void:
	super("SettlementsList", 4)


func _render(data: Dictionary) -> void:
	UiBuild.clear_children(self)
	var rows: Array = data.get("rows", [])
	if rows.is_empty():
		PanelWidgets.placeholder(self, "Colonies indisponibles.")
		return
	var help_text := str(data.get("help", ""))
	if help_text != "":  # RJ-c : la cité donne le contrôle, le traité la possession
		var help := Label.new()
		help.name = "PossessionHelp"
		help.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		help.text = help_text
		help.add_theme_color_override("font_color", Color(RichTooltip.MUTED))
		UiType.apply(help, UiType.CAPTION)
		add_child(help)
	var label_of: Callable = data.get("label_of", Callable())
	for row in rows:
		add_child(_make_settlement_row(row, label_of))


func _make_settlement_row(row: Dictionary, label_of: Callable) -> Control:
	var settlement_id := str(row.get("id", ""))
	var button := RichButton.new()
	button.name = settlement_id
	button.alignment = HORIZONTAL_ALIGNMENT_LEFT
	button.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART  # Q6 : zone `SIDE_PANEL` étroite
	button.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	var kind := str(row.get("kind", ""))
	var controller := PanelWidgets.faction_label(str(row.get("controller", "")), "", label_of)
	var text := "%s — %s, %s" % [str(row.get("name", settlement_id)), str(SettlementPanel.KIND_LABELS.get(kind, kind)), controller]
	# RJ-c : « occupée par … » (statut du cœur, `province_possession`).
	var owner_label := PanelWidgets.faction_label(str(row.get("owner", "")), "", label_of)
	var mention := PossessionText.occupied_mention(str(row.get("possession_status", "")), owner_label, controller)
	if mention != "":
		text += " — " + mention
	if bool(row.get("is_city", false)):
		text += " (cité : donne la province)"
	text += "\n    garnison : %s, %s hommes" % [FrText.count(int(row.get("garrison_units", 0)), "unité"), Money.digits(int(row.get("garrison_strength", 0)))]
	var siege: Dictionary = row.get("siege", {}) if row.get("siege") is Dictionary else {}
	if not siege.is_empty():
		text += " — assiégée par %s (%s)" % [PanelWidgets.faction_label(str(siege.get("attacker", "")), "", label_of), FrText.count(int(siege.get("turns_left", 0)), "tour")]
	button.text = text
	TooltipHost.attach_plain(button, "colony_focus_open")
	button.pressed.connect(func() -> void: settlement_requested.emit(settlement_id))
	return button


