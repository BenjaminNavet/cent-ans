class_name TooltipView
extends RefCounted

## Chantier IB (ADR 0109) : rendu en sections d'une spec d'infobulle produite par `RichTooltip`.
## Spec (dictionnaire, présentation pure) : `id`, `kind`, `title`, `subtitle`, `icon`,
## `headline` [{icon, label, value}], `stats` [{key, value}], `effects` [{text, sign, before,
## after}], `traits` {strengths, weaknesses, abilities}, `requires` [{text, met}], `warnings`,
## `flavour`, `footer` {cost, upkeep, time}, `detail` (lignes de la version complète).
## Blocs de haut en bas : en-tête, vedettes, effets, stats, forces/faiblesses, conditions,
## ambiance, pied ; filet entre blocs non vides. `detailed` : version complète (bulle
## verrouillée) ; sinon version courte (survol, `short_max_body_lines` lignes au plus).
## Style : `data/ui/tooltip_style.json`. Aucune règle de jeu ici.
##
## Nœuds nommés (tests) : `Header` (`Title`, `Subtitle`, icône), `Headline`, `Effects`, `Stats`,
## `Traits`, `Conditions`, `Detail`, `Flavour`, `Footer` (`Hint`, `Costs`), filets `Rule*`.
## Métadonnées du panneau : `ib_blocks` (noms des blocs affichés), `ib_body_lines` (lignes de
## corps, en-tête et pied exclus), `ib_detailed`.
## Tailles : `UiType` (Heading pour le titre, Body pour le corps, Caption pour sous-titre, libellés
## et pied, Title pour les chiffres vedettes). Largeur `width_px` en unités d'interface : la
## fenêtre racine applique l'échelle d'interface (`content_scale_factor`), la largeur à l'écran
## vaut donc `width_px` × échelle.

## Lu comme `CameraFeel` : `MapPaths` donne le dossier `data/`.
const DATA_FILE := "ui/tooltip_style.json"
const MAP_PATHS_SCRIPT := preload("res://scripts/map/map_paths.gd")
const FALLBACK := {
	"width_px": 360, "max_height_screen_share": 0.7, "header_icon_px": 48, "short_max_body_lines": 8,
	"kind_colors": {}, "headline": {}, "lower_is_better": [],
}
const HINT := "Alt : explorer"
const CHECK := "✓"
const CROSS := "✗"
const UNKNOWN := "•"
const WARNING := "⚠"
const HEADLINE_ICON_PX := 28

static var _cache: Dictionary = {}


## Contrôle d'infobulle pour `spec` ; `detailed` : version complète (bulle verrouillée). Met à
## jour `RichTooltip.last_panel`, `last_bbcode` (BBCode complet, pour l'épinglage T) et `last_spec`.
static func build(spec: Dictionary, detailed: bool = false) -> Control:
	var style_data := style()
	var width := float(style_data.get("width_px", 360))
	var panel := PanelContainer.new()
	panel.name = "TooltipView"
	if ResourceLoader.exists(RichTooltip.THEME_PATH):
		panel.theme = load(RichTooltip.THEME_PATH)
	panel.add_theme_stylebox_override("panel", RichTooltip.panel_style())
	var outer := VBoxContainer.new()
	outer.add_theme_constant_override("separation", 6)
	outer.custom_minimum_size = Vector2(width, 0)
	panel.add_child(outer)
	outer.add_child(_header(spec, style_data))
	var blocks := blocks_for(spec, detailed)
	var body := VBoxContainer.new()
	body.name = "Body"
	body.add_theme_constant_override("separation", 6)
	var lines := 0
	var shown := PackedStringArray()
	for entry in blocks:
		var block := _block(str(entry[0]), entry[1], spec, width)
		if block == null:
			continue
		if body.get_child_count() > 0:
			body.add_child(_rule("Rule%s" % entry[0]))
		body.add_child(block)
		shown.append(str(entry[0]))
		lines += int(entry[2])
	var max_body := _max_body_height(style_data)
	if detailed:
		# Bulle verrouillée : le corps défile au-delà de la hauteur bornée.
		var scroll := ScrollContainer.new()
		scroll.name = "BodyScroll"
		scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
		scroll.add_child(body)
		body.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		body.minimum_size_changed.connect(func() -> void:
			scroll.custom_minimum_size.y = minf(body.get_combined_minimum_size().y, max_body))
		outer.add_child(_rule("RuleHeader"))
		outer.add_child(scroll)
	elif body.get_child_count() > 0:
		outer.add_child(_rule("RuleHeader"))
		outer.add_child(body)
	var footer := _footer(spec, width)
	outer.add_child(_rule("RuleFooter"))
	outer.add_child(footer)
	panel.set_meta("ib_blocks", shown)
	panel.set_meta("ib_body_lines", lines)
	panel.set_meta("ib_detailed", detailed)
	RichTooltip.last_panel = weakref(panel)
	RichTooltip.last_bbcode = CodexText.format(RichTooltip.to_bbcode(spec), true)
	RichTooltip.last_spec = spec
	return panel


## Blocs de corps à afficher : [[nom, lignes BBCode ou données, nombre de lignes de corps]].
## Version courte : vedettes, effets, forces/faiblesses, conditions non remplies et avertissements.
## Version complète : en plus stats, capacités, prérequis remplis, `detail`, ambiance.
static func blocks_for(spec: Dictionary, detailed: bool) -> Array:
	var blocks: Array = []
	var headline: Array = spec.get("headline", [])
	if not headline.is_empty():
		blocks.append(["Headline", headline, 1])
	var effects := PackedStringArray()
	for effect in spec.get("effects", []):
		var sign := int(effect.get("sign", 0))
		var text := RichTooltip.effect_line(effect)
		if sign > 0:
			effects.append("[color=%s]▲[/color] %s" % [RichTooltip.GREEN, text])
		elif sign < 0:
			effects.append("[color=%s]▼[/color] %s" % [RichTooltip.RED, text])
		else:
			effects.append("%s %s" % [UNKNOWN, text])
	if not effects.is_empty():
		blocks.append(["Effects", effects, effects.size()])
	if detailed:
		var headline_stats: Array = []
		for item in headline:
			headline_stats.append(str(item.get("stat", "")))
		var stats: Array = []
		for stat in spec.get("stats", []):
			if not headline_stats.has(str(stat.get("key", ""))):
				stats.append(stat)
		if not stats.is_empty():
			blocks.append(["Stats", stats, ceili(stats.size() / 2.0)])
	var traits: Dictionary = spec.get("traits", {})
	var trait_lines := PackedStringArray()
	if not (traits.get("strengths", []) as Array).is_empty():
		trait_lines.append("[color=%s]▲ Forces : %s[/color]" % [RichTooltip.GREEN, ", ".join(PackedStringArray(traits["strengths"]))])
	if not (traits.get("weaknesses", []) as Array).is_empty():
		trait_lines.append("[color=%s]▼ Faiblesses : %s[/color]" % [RichTooltip.RED, ", ".join(PackedStringArray(traits["weaknesses"]))])
	if detailed and not (traits.get("abilities", []) as Array).is_empty():
		trait_lines.append("%s Capacités : %s" % [UNKNOWN, ", ".join(PackedStringArray(traits["abilities"]))])
	if not trait_lines.is_empty():
		blocks.append(["Traits", trait_lines, trait_lines.size()])
	var conditions := PackedStringArray()
	var warnings: Array = spec.get("warnings", [])
	var label := str(spec.get("requires_label", "Requiert"))
	for requirement in spec.get("requires", []):
		var met: Variant = requirement.get("met", null)
		if met == null:
			# État inconnu : utile seulement si l'action est refusée, ou dans la version complète.
			if detailed or not warnings.is_empty():
				conditions.append("[color=%s]%s[/color] %s : %s" % [RichTooltip.MUTED, UNKNOWN, label, requirement.get("text", "")])
		elif bool(met):
			if detailed:
				conditions.append("[color=%s]%s[/color] %s : %s" % [RichTooltip.GREEN, CHECK, label, requirement.get("text", "")])
		else:
			conditions.append("[color=%s]%s %s : %s[/color]" % [RichTooltip.RED, CROSS, label, requirement.get("text", "")])
	for warning in warnings:
		conditions.append("[color=%s]%s %s[/color]" % [RichTooltip.RED, WARNING, warning])
	if not conditions.is_empty():
		blocks.append(["Conditions", conditions, conditions.size()])
	if detailed:
		var detail := PackedStringArray()
		for line in spec.get("detail", []):
			if str(line) != "":
				detail.append(str(line))
		if not detail.is_empty():
			blocks.append(["Detail", detail, detail.size()])
		var flavour := str(spec.get("flavour", ""))
		if flavour != "":
			blocks.append(["Flavour", PackedStringArray(["[color=%s][i]%s[/i][/color]" % [RichTooltip.MUTED, flavour]]), 1])
	return blocks


## Style chargé depuis `data/ui/tooltip_style.json` (mis en cache).
static func style() -> Dictionary:
	if _cache.is_empty():
		_cache = FALLBACK.duplicate(true)
		var path := _data_dir().path_join(DATA_FILE)
		if not FileAccess.file_exists(path):
			path = MAP_PATHS_SCRIPT.project_root().path_join("data").path_join(DATA_FILE)
		var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(path)) if FileAccess.file_exists(path) else null
		if parsed is Dictionary:
			_cache.merge(parsed, true)
		else:
			push_warning("TooltipView : %s illisible, valeurs de repli." % DATA_FILE)
	return _cache


## Relit le style au prochain accès (tests).
static func reload() -> void:
	_cache = {}


## Couleur de catégorie de `kind` (`kind_colors`), encre par défaut.
static func kind_color(kind: String) -> Color:
	var hex := str((style().get("kind_colors", {}) as Dictionary).get(kind, ""))
	return Color.from_string(hex, RichTooltip.INK) if hex != "" else RichTooltip.INK


## Hauteur maximale d'une infobulle (part de l'écran visible, en unités d'interface).
static func max_height() -> float:
	return _screen_height() * float(style().get("max_height_screen_share", 0.7))


static func _max_body_height(style_data: Dictionary) -> float:
	# En-tête (icône + marges) et pied retranchés de la hauteur bornée.
	var header := float(style_data.get("header_icon_px", 48)) + 24.0
	var footer := float(UiType.size(UiType.CAPTION)) * 2.0 + 24.0
	return maxf(120.0, _screen_height() * float(style_data.get("max_height_screen_share", 0.7)) - header - footer)


static func _screen_height() -> float:
	var tree := Engine.get_main_loop() as SceneTree
	if tree != null and tree.root != null:
		return tree.root.get_visible_rect().size.y
	return 900.0


static func _data_dir() -> String:
	var tree := Engine.get_main_loop() as SceneTree
	if tree != null and tree.root != null:
		var map_paths := tree.root.get_node_or_null("MapPaths")
		if map_paths != null:
			return str(map_paths.get("data_dir"))
	return MAP_PATHS_SCRIPT.project_root().path_join("data")


# --- Blocs ---------------------------------------------------------------------------------


static func _header(spec: Dictionary, style_data: Dictionary) -> Control:
	var header := PanelContainer.new()
	header.name = "Header"
	var band := StyleBoxFlat.new()
	band.bg_color = Color(HudStyle.PARCHMENT_DARK, 0.35)
	band.border_color = kind_color(str(spec.get("kind", "")))
	band.border_width_left = 3
	band.set_content_margin_all(4)
	band.content_margin_left = 6
	header.add_theme_stylebox_override("panel", band)
	var row := HBoxContainer.new()
	row.add_theme_constant_override("separation", 8)
	header.add_child(row)
	var icon_px := float(style_data.get("header_icon_px", 48))
	var library := RichTooltip.icons()
	var icon_id := str(spec.get("icon", ""))
	if library != null and icon_id != "":
		var rect: TextureRect = library.call("make_rect", icon_id, icon_px, str(spec.get("icon_category", "")))
		if rect.texture != null:
			row.add_child(rect)
	var column := VBoxContainer.new()
	column.add_theme_constant_override("separation", 0)
	column.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	column.alignment = BoxContainer.ALIGNMENT_CENTER
	row.add_child(column)
	var title := _rich("Title", UiType.HEADING)
	title.add_theme_color_override("default_color", kind_color(str(spec.get("kind", ""))))
	title.text = "[b]%s[/b]" % RichTooltip.entity_name(str(spec.get("id", "")), str(spec.get("title", "")))
	column.add_child(title)
	var subtitle_text := str(spec.get("subtitle", ""))
	if subtitle_text != "":
		var subtitle := _rich("Subtitle", UiType.CAPTION)
		subtitle.add_theme_color_override("default_color", Color(RichTooltip.MUTED))
		subtitle.text = "[i]%s[/i]" % subtitle_text
		column.add_child(subtitle)
	return header


static func _block(name: String, data: Variant, spec: Dictionary, width: float) -> Control:
	match name:
		"Headline":
			return _headline(data as Array, spec)
		"Stats":
			return _stats(data as Array, width)
	var lines := PackedStringArray(data)
	if lines.is_empty():
		return null
	var label := _rich(name, UiType.BODY)
	label.custom_minimum_size = Vector2(width, 0)
	label.text = CodexText.format("\n".join(lines), true)
	return label


## Écussons des chiffres vedettes : icône, grand chiffre, libellé en légende.
static func _headline(items: Array, _spec: Dictionary) -> Control:
	var row := HBoxContainer.new()
	row.name = "Headline"
	row.add_theme_constant_override("separation", 12)
	var library := RichTooltip.icons()
	for item in items:
		var badge := HBoxContainer.new()
		badge.name = "Badge_%s" % str(item.get("key", ""))
		badge.add_theme_constant_override("separation", 6)
		badge.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		var icon_id := str(item.get("icon", ""))
		if library != null and icon_id != "" and bool(library.call("has_icon", icon_id)):
			badge.add_child(library.call("make_rect", icon_id, HEADLINE_ICON_PX, ""))
		var column := VBoxContainer.new()
		column.add_theme_constant_override("separation", -2)
		var value := _rich("Value", UiType.TITLE)
		value.autowrap_mode = TextServer.AUTOWRAP_OFF
		var sign := int(item.get("sign", 0))
		var color: String = RichTooltip.GREEN if sign > 0 else (RichTooltip.RED if sign < 0 else "")
		value.text = "[b]%s[/b]" % str(item.get("value", "")) if color == "" else "[b][color=%s]%s[/color][/b]" % [color, str(item.get("value", ""))]
		column.add_child(value)
		var caption := _label("Caption", str(item.get("label", "")), UiType.CAPTION, Color(RichTooltip.MUTED))
		column.add_child(caption)
		badge.add_child(column)
		row.add_child(badge)
	return row


## Grille 2 colonnes « libellé … valeur ».
static func _stats(stats: Array, width: float) -> Control:
	var grid := GridContainer.new()
	grid.name = "Stats"
	grid.columns = 2
	grid.add_theme_constant_override("h_separation", 16)
	grid.add_theme_constant_override("v_separation", 0)
	for stat in stats:
		var cell := HBoxContainer.new()
		cell.custom_minimum_size = Vector2((width - 16.0) / 2.0, 0)
		var name_label := _label("Label", str(stat.get("label", "")), UiType.BODY, RichTooltip.INK)
		name_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		cell.add_child(name_label)
		var value_label := _label("Value", str(stat.get("value", "")), UiType.BODY, RichTooltip.INK)
		value_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_RIGHT
		cell.add_child(value_label)
		grid.add_child(cell)
	return grid


## Pied : aide de touche à gauche (Caption atténué), coût / entretien / durée à droite.
static func _footer(spec: Dictionary, width: float) -> Control:
	var row := HBoxContainer.new()
	row.name = "Footer"
	row.add_theme_constant_override("separation", 8)
	var hint := _label("Hint", HINT, UiType.CAPTION, Color(RichTooltip.MUTED))
	hint.size_flags_vertical = Control.SIZE_SHRINK_END
	row.add_child(hint)
	var footer: Dictionary = spec.get("footer", {})
	var headline_keys: Array = []
	for item in spec.get("headline", []):
		headline_keys.append(str(item.get("key", "")))
	var parts := PackedStringArray()
	var icons := {"cost": "hud_treasury", "upkeep": "hud_income", "time": "hud_end_turn"}
	for key in ["cost", "upkeep", "time"]:
		var text := str(footer.get(key, ""))
		if text == "" or (key == "cost" and headline_keys.has("cost")):
			continue
		var icon := RichTooltip.icon_bbcode(icons[key], 14)
		parts.append("%s %s" % [icon, text] if icon != "" else "%s : %s" % [RichTooltip.FOOTER_LABELS[key], text])
	var costs := _rich("Costs", UiType.CAPTION)
	costs.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	costs.custom_minimum_size = Vector2(width * 0.6, 0)
	costs.text = "[right]%s[/right]" % "  ·  ".join(parts)
	costs.visible = not parts.is_empty()
	row.add_child(costs)
	return row


static func _rule(name: String) -> Control:
	var rule := HSeparator.new()
	rule.name = name
	var line := StyleBoxLine.new()
	line.color = Color(HudStyle.INK_SOFT, 0.45)
	line.thickness = 1
	rule.add_theme_stylebox_override("separator", line)
	rule.add_theme_constant_override("separation", 2)
	return rule


static func _rich(name: String, variation: String) -> RichTextLabel:
	var label := RichTextLabel.new()
	label.name = name
	label.bbcode_enabled = true
	label.fit_content = true
	label.scroll_active = false
	label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	label.add_theme_color_override("default_color", RichTooltip.INK)
	UiType.apply(label, variation)
	label.add_theme_font_size_override("bold_font_size", UiType.size(variation))
	label.add_theme_font_size_override("italics_font_size", UiType.size(variation))
	return label


static func _label(name: String, text: String, variation: String, color: Color) -> Label:
	var label := Label.new()
	label.name = name
	label.text = text
	UiType.apply(label, variation)
	label.add_theme_color_override("font_color", color)
	return label
