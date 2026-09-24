class_name SeasonReport
extends PanelContainer

## F3 — rapport de saison : fenêtre parchemin ouverte en fin de tour, qui liste les
## événements majeurs du journal de la simulation (`end_turn`) groupés par rubrique. Chaque
## ligne liée à une province ou une armée est cliquable (`entry_selected` : la carte y porte
## la caméra). « Ne plus afficher » désactive le rapport (réglage `interface/season_report`).
## Aucun calcul de jeu : tri et mise en forme des événements déjà produits par `core/`.

signal entry_selected(entry: Dictionary)
signal closed
signal disable_requested

## Rubriques du rapport : titre, glyphe, genres d'événements (`EventKind` en snake_case).
const GROUPS := [
	{"title": "Batailles et sièges", "glyph": "⚔", "kinds": ["battle", "siege_started", "siege_lifted", "province_captured", "raid", "army_destroyed", "general_captured"]},
	{"title": "Diplomatie et Église", "glyph": "✉", "kinds": ["war_declared", "peace_signed", "alliance_formed", "alliance_broken", "vassalage", "vassal_rebellion", "embargo", "diplomatic_offer", "diplomacy", "excommunication", "schism", "heresy"]},
	{"title": "Cour et dynasties", "glyph": "♔", "kinds": ["death", "succession", "no_heir", "birth", "marriage", "regency", "faction_destroyed"]},
	{"title": "Royaume", "glyph": "⚒", "kinds": ["building_completed", "technology_researched", "revolt", "plague", "famine", "bankruptcy"]},
	# H9 : régimes revenus au défaut, Carême ; blessés soignés, épidémies contenues.
	{"title": "Table et santé", "glyph": "✚", "kinds": ["table", "medicine"]},
	# H11 : mutations monétaires, rançons et échéances, ordres de chevalerie.
	{"title": "Monnaie, rançons et chevalerie", "glyph": "¤", "kinds": ["coinage", "ransom", "chivalry"]},
	{"title": "Chronique", "glyph": "§", "kinds": ["chronicle", "victory", "defeat", "campaign_ended"]},
]
## Genres rapportés même quand ils ne concernent pas le joueur (nouvelles du monde).
const WORLD_KINDS := ["war_declared", "peace_signed", "faction_destroyed", "schism", "chronicle", "victory", "defeat", "campaign_ended", "succession", "excommunication"]
## H9 / H11 : glyphe, libellé et encre par genre, pour le journal et les alertes.
const KIND_STYLES := {
	"table": {"glyph": "♨", "label": "La Table", "color": "#7a4a10"},
	"medicine": {"glyph": "✚", "label": "Médecine", "color": "#2a6a4a"},
	# H11
	"coinage": {"glyph": "¤", "label": "Monnaie", "color": "#8a6a10"},
	"ransom": {"glyph": "⚖", "label": "Rançon", "color": "#7a2a1a"},
	"chivalry": {"glyph": "⚜", "label": "Chevalerie", "color": "#2a3a7a"},
}
const MAX_ENTRIES_PER_GROUP := 12
const MAX_LIST_HEIGHT := 440.0

var title_label: Label
var scroll: ScrollContainer
var list_box: VBoxContainer
var groups: Array = []  # [{title, glyph, entries: [event]}]
var _title: String = ""


func _ready() -> void:
	theme = load("res://scenes/ui/parchment_theme.tres")
	process_mode = Node.PROCESS_MODE_ALWAYS
	set_anchors_and_offsets_preset(Control.PRESET_TOP_LEFT)
	position = Vector2(20, 60)
	custom_minimum_size = Vector2(520, 0)
	var box := VBoxContainer.new()
	box.add_theme_constant_override("separation", 6)
	add_child(box)
	var header := HBoxContainer.new()
	box.add_child(header)
	title_label = Label.new()
	title_label.add_theme_font_size_override("font_size", 22)
	title_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	header.add_child(title_label)
	var close_button := Button.new()
	close_button.text = "×"
	close_button.tooltip_text = "Fermer (Échap)"
	close_button.pressed.connect(close)
	header.add_child(close_button)
	box.add_child(HSeparator.new())
	scroll = ScrollContainer.new()
	scroll.size_flags_vertical = Control.SIZE_EXPAND_FILL
	scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	box.add_child(scroll)
	list_box = VBoxContainer.new()
	list_box.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	list_box.add_theme_constant_override("separation", 2)
	scroll.add_child(list_box)
	var footer := HBoxContainer.new()
	footer.alignment = BoxContainer.ALIGNMENT_END
	footer.add_theme_constant_override("separation", 8)
	box.add_child(footer)
	var disable := Button.new()
	disable.text = "Ne plus afficher"
	disable.tooltip_text = "Réactivable dans Réglages → Carte."
	disable.pressed.connect(func() -> void:
		disable_requested.emit()
		close())
	footer.add_child(disable)
	var ok := Button.new()
	ok.text = "Continuer"
	ok.pressed.connect(close)
	footer.add_child(ok)
	hide()


func close() -> void:
	hide()
	closed.emit()


## Regroupe les événements pertinents : `is_relevant(event) -> bool` décide si un événement
## concerne le joueur ; les nouvelles du monde (`WORLD_KINDS`) passent toujours.
static func build_groups(events: Array, is_relevant: Callable) -> Array:
	var result: Array = []
	for spec in GROUPS:
		var entries: Array = []
		for event in events:
			if not (event is Dictionary):
				continue
			var kind := str(event.get("kind", ""))
			if not (kind in spec["kinds"]) or str(event.get("text_fr", "")) == "":
				continue
			if kind in WORLD_KINDS or is_relevant.call(event):
				entries.append(event)
		if not entries.is_empty():
			result.append({"title": spec["title"], "glyph": spec["glyph"], "entries": entries})
	return result


static func entry_count(report_groups: Array) -> int:
	var total := 0
	for group in report_groups:
		total += (group["entries"] as Array).size()
	return total


## Affiche le rapport ; renvoie faux (et reste fermé) s'il n'y a rien de notable.
func show_report(date_label: String, report_groups: Array) -> bool:
	groups = report_groups
	_title = date_label
	_render()
	if report_groups.is_empty():
		hide()
		return false
	show()
	_fit_height.call_deferred()
	return true


## Fusionne des événements survenus après l'affichage initiale (batailles résolues via le
## dialogue d'avant-bataille, dont le résultat n'est connu qu'après la fermeture de ce
## dialogue) dans les rubriques déjà construites, et rouvre le rapport s'il avait été fermé.
## `is_relevant` : voir `build_groups`.
func add_events(new_events: Array, is_relevant: Callable) -> bool:
	var new_groups := build_groups(new_events, is_relevant)
	if new_groups.is_empty():
		return false
	groups = merge_groups(groups, new_groups)
	_render()
	show()
	_fit_height.call_deferred()
	return true


## Fusionne deux listes de rubriques (même format que `build_groups`) par titre, en
## conservant l'ordre de `GROUPS` et en ajoutant les nouvelles entrées à la suite.
static func merge_groups(existing: Array, additional: Array) -> Array:
	var by_title: Dictionary = {}
	var order: Array = []
	for group in existing:
		by_title[group["title"]] = {"title": group["title"], "glyph": group["glyph"], "entries": (group["entries"] as Array).duplicate()}
		order.append(group["title"])
	for group in additional:
		if by_title.has(group["title"]):
			(by_title[group["title"]]["entries"] as Array).append_array(group["entries"])
		else:
			by_title[group["title"]] = {"title": group["title"], "glyph": group["glyph"], "entries": (group["entries"] as Array).duplicate()}
			order.append(group["title"])
	var result: Array = []
	for title in order:
		result.append(by_title[title])
	return result


func _render() -> void:
	for child in list_box.get_children():
		list_box.remove_child(child)
		child.queue_free()
	if groups.is_empty():
		return
	title_label.text = "Rapport de saison — %s" % _title
	for group in groups:
		var heading := Label.new()
		heading.text = "%s  %s" % [group["glyph"], group["title"]]
		heading.add_theme_font_size_override("font_size", 18)
		heading.add_theme_color_override("font_color", Color(0.45, 0.12, 0.08))
		list_box.add_child(heading)
		var entries: Array = group["entries"]
		for index in mini(entries.size(), MAX_ENTRIES_PER_GROUP):
			list_box.add_child(_entry_row(entries[index]))
		if entries.size() > MAX_ENTRIES_PER_GROUP:
			var more := Label.new()
			more.text = "… et %d autres (voir le journal)" % (entries.size() - MAX_ENTRIES_PER_GROUP)
			more.add_theme_font_size_override("font_size", 13)
			list_box.add_child(more)


## Hauteur ajustée au contenu (au plus `MAX_LIST_HEIGHT`, défilement au-delà).
func _fit_height() -> void:
	scroll.custom_minimum_size.y = minf(list_box.get_combined_minimum_size().y, MAX_LIST_HEIGHT)
	reset_size()


func _entry_row(event: Dictionary) -> Control:
	var text := str(event.get("text_fr", ""))
	var target := str(event.get("province", "")) != "" or str(event.get("army", "")) != ""
	if not target:
		var label := Label.new()
		label.text = "•  " + text
		label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		label.add_theme_font_size_override("font_size", 15)
		return label
	var button := Button.new()
	button.flat = true
	button.text = "•  %s  ⌖" % text
	button.alignment = HORIZONTAL_ALIGNMENT_LEFT
	button.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	button.tooltip_text = "Voir sur la carte"
	button.add_theme_font_size_override("font_size", 15)
	button.custom_minimum_size = Vector2(460, 0)
	button.pressed.connect(func() -> void: entry_selected.emit(event))
	return button


func line_count() -> int:
	return list_box.get_child_count()
