class_name CreditsScreen
extends Control

## F3 — écran de crédits : fond illustré voilé, parchemin défilant lentement. Le texte vient
## de `CREDITS.md` à la racine du dépôt (ou à côté de `data/` dans le jeu exporté) ; sans
## fichier, un texte intégré. Markdown simple converti en BBCode (titres, listes, gras,
## italique, liens réduits à leur texte). Molette ou glisser : défilement manuel.
## Échap ou « Retour » : signal `closed`.
## Lot P2e (ADR 0097, bible DA § 12.1/12.2) : `self` reste un `Control` plein écran, seul hôte du
## fond illustré voilé (`MenuBackground`, propre à cet écran — la zone `MODAL` ne fournit qu'un
## voile uni) ; le cadre du parchemin (`_panel`, ex-`CenterContainer` + `PanelContainer`) rejoint
## lui la zone `MODAL` de `UiLayout`. Tailles par `UiType` (titre, bouton) et par la même échelle
## dans le BBCode généré (`markdown_to_bbcode`) ; ouverture et fermeture du cadre par `UiMotion`.

signal closed

const SCROLL_SPEED := 22.0  # pixels par seconde
const FALLBACK_TEXT := """# Cent Ans

Un jeu de grande stratégie sur la guerre de Cent Ans (1337-1453).

## Conception et développement
- Benjamin Navet
- Développé avec l'aide d'agents Claude (Anthropic)

## Technologies
- Godot Engine 4 (licence MIT)
- Rust et godot-rust (gdext)
- Blender, Python, Pillow, numpy

## Données géographiques
- ETOPO 2022 (NOAA) pour le relief et les fonds marins
- Copernicus DEM GLO-90 (© DLR e.V. 2010-2014 et © Airbus Defence and Space GmbH 2014-2018,
  fourni dans le cadre de COPERNICUS par l'Union européenne et l'ESA) pour le relief fin
- KK10 (Kaplan et al. 2011, PANGAEA, CC BY 3.0) pour le défrichement vers 1340
- Natural Earth pour les côtes, les rivières et les lacs

## Assets
- Écus, sons, musiques, modèles et illustration du menu générés de façon procédurale
- Portraits, miniatures de chronique et illustrations de l'encyclopédie générés par IA
  (openai/gpt-5-image-mini via OpenRouter)

## Sources historiques
Voir `docs/design/` et les champs `sources` des fichiers de `data/`.

*Merci d'avoir joué.*"""

var text_label: RichTextLabel
var scroll: ScrollContainer
var source_path: String = ""
var _auto_scroll := true
var _scroll_position := 0.0
var _panel: PanelContainer


func _ready() -> void:
	set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	var background := MenuBackground.new()
	background.dim = 0.45
	add_child(background)
	_panel = PanelContainer.new()
	_panel.theme = load("res://scenes/ui/parchment_theme.tres")
	_panel.mouse_filter = Control.MOUSE_FILTER_STOP
	_panel.custom_minimum_size = Vector2(760, 620)
	add_child(_panel)
	var box := UiBuild.vbox(8, _panel)
	var title := UiBuild.label("Crédits")
	UiType.apply(title, UiType.TITLE)
	title.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	box.add_child(title)
	scroll = ScrollContainer.new()
	scroll.size_flags_vertical = Control.SIZE_EXPAND_FILL
	scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	box.add_child(scroll)
	text_label = RichTextLabel.new()
	text_label.bbcode_enabled = true
	text_label.fit_content = true
	text_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	UiType.apply(text_label, UiType.BODY)
	text_label.add_theme_font_size_override("bold_font_size", UiType.size(UiType.BODY))
	text_label.add_theme_color_override("default_color", Color(0.22, 0.14, 0.07))
	scroll.add_child(text_label)
	scroll.gui_input.connect(func(event: InputEvent) -> void:
		if event is InputEventMouseButton or event is InputEventScreenDrag:
			_auto_scroll = false)
	var back := UiBuild.button("Retour")
	UiType.apply(back, UiType.HEADING)
	back.size_flags_horizontal = Control.SIZE_SHRINK_CENTER
	back.pressed.connect(close)
	box.add_child(back)
	var parts := split_title(load_credits())
	title.text = parts[0]
	text_label.text = markdown_to_bbcode(parts[1])
	UiZones.put(UiZones.Zone.MODAL, _panel)
	UiMotion.fade_in(_panel)


func _process(delta: float) -> void:
	if not visible or not _auto_scroll:
		return
	_scroll_position += SCROLL_SPEED * delta
	scroll.scroll_vertical = int(_scroll_position)


func _unhandled_input(event: InputEvent) -> void:
	if visible and event.is_action_pressed("ui_cancel"):
		close()
		get_viewport().set_input_as_handled()


func close() -> void:
	closed.emit()
	UiMotion.fade_out(_panel, UiMotion.DURATION, true)
	queue_free()


## Contenu de `CREDITS.md` (dépôt, puis jeu exporté), texte intégré à défaut.
func load_credits() -> String:
	var candidates: Array[String] = [ProjectSettings.globalize_path("res://").path_join("../CREDITS.md").simplify_path()]
	candidates.append(DataFile.data_dir().path_join("../CREDITS.md").simplify_path())
	candidates.append("res://CREDITS.md")
	for candidate in candidates:
		if FileAccess.file_exists(candidate):
			source_path = candidate
			return FileAccess.get_file_as_string(candidate)
	source_path = ""
	return FALLBACK_TEXT


## Markdown simple → BBCode centré (audit A3 M4) : paragraphes recollés, tableaux en lignes
## « [b]Auteur[/b] — colonnes », parenthèses purement techniques (« (`tools/`) », « (outil `…`) »)
## retirées, code restant en italique.
static func markdown_to_bbcode(markdown: String) -> String:
	var bold := RegEx.create_from_string("\\*\\*(.+?)\\*\\*")
	var italic := RegEx.create_from_string("(?<![*\\w])[*_](.+?)[*_](?![*\\w])")
	var link := RegEx.create_from_string("\\[([^\\]]+)\\]\\([^)]+\\)")
	var code := RegEx.create_from_string("`([^`]+)`")
	var technical := RegEx.create_from_string("\\s*\\((?:outils?\\s+)?`[^`]*`\\)")
	var blocks := PackedStringArray()
	var paragraph := PackedStringArray()
	var table_rows: Array[PackedStringArray] = []
	for raw_line in (markdown + "\n").split("\n"):
		var stripped := raw_line.strip_edges()
		if stripped.begins_with("|"):
			var cells := PackedStringArray()
			for cell in stripped.trim_prefix("|").trim_suffix("|").split("|"):
				cells.append(cell.strip_edges())
			if not cells.is_empty() and not cells[0].begins_with("---"):
				table_rows.append(cells)
			continue
		if not table_rows.is_empty():
			# La première ligne est l'en-tête : les autres deviennent « Auteur — valeurs ».
			for index in range(1, table_rows.size()):
				var row: PackedStringArray = table_rows[index]
				var rest := row.slice(1)
				blocks.append("[b]%s[/b] — [font_size=%d]%s[/font_size]" % [row[0], UiType.size(UiType.CAPTION), " — ".join(rest)])
			table_rows.clear()
		var is_item := stripped.begins_with("- ") or stripped.begins_with("* ")
		if stripped == "" or stripped.begins_with("#") or is_item:
			if not paragraph.is_empty():
				blocks.append(" ".join(paragraph))
				paragraph.clear()
		if stripped == "":
			continue
		if stripped.begins_with("#"):
			blocks.append(stripped)
		else:
			paragraph.append(stripped.substr(2) if is_item else stripped)
	var lines := PackedStringArray()
	for block in blocks:
		var line := technical.sub(block, "", true)
		line = link.sub(line, "$1", true)
		if not line.begins_with("[b]"):
			line = line.replace("[", "[lb]")
		line = bold.sub(line, "[b]$1[/b]", true)
		line = italic.sub(line, "[i]$1[/i]", true)
		line = code.sub(line, "[i]$1[/i]", true)
		if line.begins_with("# "):
			line = "[font_size=%d][b]%s[/b][/font_size]" % [UiType.size(UiType.TITLE), line.trim_prefix("# ")]
		elif line.begins_with("## "):
			line = "\n[font_size=%d][b]%s[/b][/font_size]" % [UiType.size(UiType.HEADING), line.trim_prefix("## ")]
		elif line.begins_with("### "):
			line = "[font_size=%d][b]%s[/b][/font_size]" % [UiType.size(UiType.HEADING), line.trim_prefix("### ")]
		lines.append(line)
	return "[center]%s[/center]" % "\n".join(lines)


## Sépare le titre de premier niveau (« # Crédits — Cent Ans ») du reste : il remplace le titre
## fixe de l'écran au lieu d'être répété dessous (audit A3 M4).
static func split_title(markdown: String) -> PackedStringArray:
	var lines := markdown.split("\n")
	for index in lines.size():
		var line := lines[index].strip_edges()
		if line == "":
			continue
		if line.begins_with("# "):
			return PackedStringArray([line.trim_prefix("# "), "\n".join(lines.slice(index + 1))])
		break
	return PackedStringArray(["Crédits", markdown])
