class_name SeasonReport
extends PanelContainer

## F3 — rapport de saison : fenêtre parchemin ouverte en fin de tour, qui liste les
## événements majeurs du journal de la simulation (`end_turn`). Lot U5 (audit A3, T1) : cinq
## rubriques dans l'ordre d'urgence — « Vos terres » (pertes et prises en rouge, en premier),
## « Trésor », « Armées », « Constructions et recherches », « Le monde » (nouvelles étrangères
## filtrées par intérêt : voisins, alliés, ennemis, grandes puissances). Chaque ligne liée à un
## lieu, une armée, une technologie ou au trésor porte un bouton d'action (`entry_selected`).
## « Ne plus afficher » désactive le rapport (réglage `interface/season_report`).
## Aucun calcul de jeu : tri et mise en forme des événements déjà produits par `core/`.

signal entry_selected(entry: Dictionary)
signal closed
signal disable_requested

## Rubriques du rapport, dans l'ordre d'affichage.
const SECTIONS := [
	{"id": "lands", "title": "Vos terres", "glyph": "⚑"},
	{"id": "treasury", "title": "Trésor", "glyph": "₶"},
	{"id": "armies", "title": "Armées", "glyph": "⚔"},
	{"id": "works", "title": "Constructions et recherches", "glyph": "⚒"},
	{"id": "world", "title": "Le monde", "glyph": "✉"},
]
## Genres rangés par rubrique quand ils concernent le joueur (le reste va à « Le monde »).
const SECTION_KINDS := {
	"lands": ["province_captured", "siege_started", "siege_lifted", "raid", "revolt", "plague", "famine",
		"death", "succession", "no_heir", "birth", "marriage", "regency", "table", "medicine", "chivalry",
		"agent", "excommunication", "heresy", "vassal_rebellion", "victory", "defeat", "campaign_ended",
		"edict", "mission", "crusade"],  # C4 : édits régionaux ; NT3 : missions ; JR3 : croisade
	# « income » (revenus bruts) : redondant avec la ligne de synthèse du trésor, laissé au journal.
	"treasury": ["bankruptcy", "coinage", "ransom", "trade"],  # C5 : accords et routes coupées
	"armies": ["battle", "army_destroyed", "general_captured", "recruited", "attrition"],
	"works": ["building_completed", "technology_researched"],
}
## Genres d'une autre faction qui peuvent figurer dans « Le monde » (après filtre d'intérêt).
const WORLD_NEWS_KINDS := [
	"war_declared", "peace_signed", "alliance_formed", "alliance_broken", "vassalage", "vassal_rebellion",
	"faction_destroyed", "schism", "excommunication", "succession", "death", "province_captured",
	"battle", "revolt", "plague", "chronicle", "victory", "defeat", "campaign_ended", "general_captured"]
## Genres rapportés même quand ils ne concernent pas le joueur et sans filtre fourni.
const WORLD_KINDS := ["war_declared", "peace_signed", "faction_destroyed", "schism", "chronicle", "victory", "defeat", "campaign_ended", "succession", "excommunication"]
## H9 / H11 : glyphe, libellé et encre par genre, pour le journal et les alertes.
const KIND_STYLES := {
	"table": {"glyph": "♨", "label": "La Table", "color": "#7a4a10"},
	"medicine": {"glyph": "✚", "label": "Médecine", "color": "#2a6a4a"},
	# H11
	"coinage": {"glyph": "¤", "label": "Monnaie", "color": "#8a6a10"},
	"ransom": {"glyph": "⚖", "label": "Rançon", "color": "#7a2a1a"},
	"chivalry": {"glyph": "⚜", "label": "Chevalerie", "color": "#2a3a7a"},
	# C6
	"agent": {"glyph": "✦", "label": "Agents", "color": "#4a2a6a"},
	# C5 : accords commerciaux, routes coupées par la guerre, un siège ou un blocus.
	"trade": {"glyph": "⚓", "label": "Commerce", "color": "#1a5a6a"},
	# NT3 : missions obtenues, réussies, échouées.
	"mission": {"glyph": "✠", "label": "Mission", "color": "#5a3a10"},
	# JR3 : ferveur, passage prêché, contingents, débandade, cité du vœu prise ou perdue.
	"crusade": {"glyph": "✠", "label": "Croisade", "color": RichTooltip.RED},
}
## Ton d'une ligne (`_tone`) : perte (rouge, en tête), prise (vert, juste après), neutre.
const TONE_LOSS := "loss"
const TONE_GAIN := "gain"
const MAX_ENTRIES_PER_GROUP := 12
const MAX_LIST_HEIGHT := 460.0
## AR1 : taille de la vignette enluminée en tête du rapport.
const VIGNETTE_SIZE := Vector2(536, 170)

var title_label: Label
var scroll: ScrollContainer
var list_box: VBoxContainer
var groups: Array = []  # [{id, title, glyph, entries: [event + _tone]}]
var _title: String = ""
var vignette: TextureRect
var vignette_caption: Label
## Identifiant de la vignette affichée (tests, captures).
var vignette_id: String = ""
## Q5 : image du dernier remplissage (les panneaux ouverts par la fin de tour dans cette image
## restent sous le rapport ; ceux que le joueur ouvre ensuite le referment).
var filled_frame := 0


func _ready() -> void:
	theme = load("res://scenes/ui/parchment_theme.tres")
	process_mode = Node.PROCESS_MODE_ALWAYS
	set_anchors_and_offsets_preset(Control.PRESET_TOP_LEFT)
	position = Vector2(20, 60)
	custom_minimum_size = Vector2(560, 0)
	var box := UiBuild.vbox(6)
	add_child(box)
	var header := HBoxContainer.new()
	box.add_child(header)
	title_label = Label.new()
	UiType.apply(title_label, UiType.HEADING)
	title_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	header.add_child(title_label)
	var close_button := UiBuild.button("×")
	RichTooltip.attach_plain(close_button, "close_escape")
	close_button.pressed.connect(close)
	header.add_child(close_button)
	box.add_child(HSeparator.new())
	# AR1 : vignette enluminée du fait le plus marquant de la saison (genre prioritaire).
	vignette = TextureRect.new()
	vignette.name = "Vignette"
	vignette.custom_minimum_size = VIGNETTE_SIZE
	vignette.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
	vignette.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_COVERED
	vignette.clip_contents = true
	vignette.hide()
	box.add_child(vignette)
	vignette_caption = Label.new()
	UiType.apply(vignette_caption, UiType.CAPTION)
	vignette_caption.add_theme_color_override("font_color", HudStyle.RUBRIC)
	vignette_caption.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	vignette_caption.hide()
	box.add_child(vignette_caption)
	scroll = ScrollContainer.new()
	scroll.size_flags_vertical = Control.SIZE_EXPAND_FILL
	scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	box.add_child(scroll)
	list_box = UiBuild.vbox(3)
	list_box.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	scroll.add_child(list_box)
	var footer := UiBuild.hbox(8)
	footer.alignment = BoxContainer.ALIGNMENT_END
	box.add_child(footer)
	var disable := UiBuild.button("Ne plus afficher")
	RichTooltip.attach_plain(disable, "season_report_disable")
	disable.pressed.connect(func() -> void:
		disable_requested.emit()
		close())
	footer.add_child(disable)
	var ok := UiBuild.button("Continuer", close, footer)
	hide()


func close() -> void:
	hide()
	closed.emit()


## Rubrique d'un événement qui concerne le joueur (« lands », « treasury »…), « world » sinon.
static func section_of(kind: String) -> String:
	for section in SECTION_KINDS:
		if kind in SECTION_KINDS[section]:
			return section
	return "world"


## JR5 : vrai pour une nouvelle publique (champ `public` du cœur : délivrance et perte de la cité
## du vœu, appel à défendre une place sainte), montrée à tous les joueurs quel que soit le filtre.
static func is_public(event: Dictionary) -> bool:
	return bool(event.get("public", false))


## JR3 : vrai pour une nouvelle de croisade qui ne regarde que la faction croisée (passage,
## contingents, débandade) et que `player` n'a donc pas à lire.
static func is_private_crusade(event: Dictionary, player: String) -> bool:
	if str(event.get("kind", "")) != "crusade" or is_public(event):
		return false
	return player != "" and str(event.get("faction", "")) != player


## Ton d'un événement pour le joueur `player` : perte d'une place, prise, ou neutre.
## `province_owner(province_id) -> String` (facultatif) désigne le propriétaire actuel.
static func tone_of(event: Dictionary, player: String, province_owner: Callable = Callable()) -> String:
	if player == "":
		return ""
	var kind := str(event.get("kind", ""))
	var faction := str(event.get("faction", ""))
	match kind:
		"province_captured":
			if faction == player:
				return TONE_GAIN
			return TONE_LOSS
		"siege_started", "raid":
			return TONE_LOSS if faction != player else ""
		"revolt", "army_destroyed", "bankruptcy", "defeat", "vassal_rebellion", "famine", "plague":
			return TONE_LOSS
		"siege_lifted", "victory":
			return TONE_GAIN if faction == player or kind == "victory" else ""
		"crusade":  # JR5 : la cité du vœu délivrée (prise) ou perdue (perte) par le joueur
			if faction != player:
				return ""
			if bool(event.get("loss", false)):
				return TONE_LOSS
			return TONE_GAIN if is_public(event) else ""
	return ""


## Regroupe les événements en rubriques. `is_relevant(event) -> bool` décide si un événement
## concerne le joueur (rubriques du royaume) ; les autres vont à « Le monde » s'ils passent
## `keeps_world(event) -> bool` (filtre d'intérêt ; par défaut les nouvelles de `WORLD_KINDS`).
## `player` : faction du joueur, pour le ton des lignes (pertes et prises).
static func build_groups(events: Array, is_relevant: Callable, keeps_world: Callable = Callable(), player: String = "") -> Array:
	var by_section := {}
	for event in events:
		if not (event is Dictionary) or str(event.get("text_fr", "")) == "":
			continue
		var kind := str(event.get("kind", ""))
		var section := ""
		if kind == "income" or is_private_crusade(event, player):
			continue
		if is_relevant.call(event):
			section = section_of(kind)
			if section == "world" and not (kind in WORLD_NEWS_KINDS or kind in WORLD_KINDS or kind.begins_with("diplom") or kind == "embargo"):
				continue
		elif is_public(event):  # JR5 : lue par tous, quel que soit le filtre d'intérêt
			section = "world"
		elif keeps_world.is_valid():
			if kind in WORLD_NEWS_KINDS and keeps_world.call(event):
				section = "world"
		elif kind in WORLD_KINDS:
			section = "world"
		if section == "":
			continue
		var entry: Dictionary = (event as Dictionary).duplicate()
		entry["_tone"] = tone_of(event, player) if is_relevant.call(event) else ""
		if not by_section.has(section):
			by_section[section] = []
		(by_section[section] as Array).append(entry)
	var result: Array = []
	for spec in SECTIONS:
		if not by_section.has(spec["id"]):
			continue
		result.append({"id": spec["id"], "title": spec["title"], "glyph": spec["glyph"], "entries": sort_entries(by_section[spec["id"]])})
	return result


## Pertes d'abord, puis prises, puis le reste (ordre du journal conservé à ton égal).
static func sort_entries(entries: Array) -> Array:
	var losses: Array = []
	var gains: Array = []
	var rest: Array = []
	for entry in entries:
		match str(entry.get("_tone", "")):
			TONE_LOSS:
				losses.append(entry)
			TONE_GAIN:
				gains.append(entry)
			_:
				rest.append(entry)
	return losses + gains + rest


## Ajoute des lignes de synthèse (`{text_fr, kind, _tone}`) en tête de la rubrique `section_id`
## (créée à sa place si absente) : par ex. le bilan du trésor.
static func with_summary(report_groups: Array, section_id: String, lines: Array) -> Array:
	if lines.is_empty():
		return report_groups
	var result: Array = []
	var inserted := false
	for spec in SECTIONS:
		var existing: Dictionary = {}
		for group in report_groups:
			if str(group.get("id", "")) == spec["id"]:
				existing = group
		if spec["id"] == section_id:
			var entries: Array = lines.duplicate()
			if not existing.is_empty():
				entries.append_array(existing["entries"])
			result.append({"id": spec["id"], "title": spec["title"], "glyph": spec["glyph"], "entries": entries})
			inserted = true
		elif not existing.is_empty():
			result.append(existing)
	return result if inserted else report_groups


static func entry_count(report_groups: Array) -> int:
	var total := 0
	for group in report_groups:
		total += (group["entries"] as Array).size()
	return total


## Vrai si le rapport contient autre chose que des lignes de synthèse.
static func has_news(report_groups: Array) -> bool:
	for group in report_groups:
		for entry in group["entries"]:
			if str(entry.get("kind", "")) != "summary":
				return true
	return false


## Affiche le rapport ; renvoie faux (et reste fermé) s'il n'y a rien de notable.
func show_report(date_label: String, report_groups: Array) -> bool:
	filled_frame = Engine.get_process_frames()
	groups = report_groups
	_title = date_label
	_render()
	if not has_news(report_groups):
		hide()
		return false
	show()
	_fit_height.call_deferred()
	return true


## Fusionne des événements survenus après l'affichage initiale (batailles résolues via le
## dialogue d'avant-bataille, dont le résultat n'est connu qu'après la fermeture de ce
## dialogue) dans les rubriques déjà construites, et rouvre le rapport s'il avait été fermé.
## Mêmes paramètres que `build_groups`. `date_label` : titre du rapport s'il n'en a pas encore.
func add_events(new_events: Array, is_relevant: Callable, keeps_world: Callable = Callable(), player: String = "", date_label: String = "") -> bool:
	filled_frame = Engine.get_process_frames()
	var new_groups := build_groups(new_events, is_relevant, keeps_world, player)
	if new_groups.is_empty():
		return false
	groups = merge_groups(groups, new_groups)
	if _title == "":  # Q1 : bataille livrée avant la première fin de tour (titre vide)
		_title = date_label
	_render()
	show()
	_fit_height.call_deferred()
	return true


## Fusionne deux listes de rubriques (même format que `build_groups`) dans l'ordre de
## `SECTIONS`, les nouvelles entrées à la suite (pertes et prises remises en tête).
static func merge_groups(existing: Array, additional: Array) -> Array:
	var by_title: Dictionary = {}
	var order: Array = []
	for group in existing + additional:
		var title := str(group["title"])
		if not by_title.has(title):
			by_title[title] = {"id": group.get("id", ""), "title": title, "glyph": group["glyph"], "entries": []}
			order.append(title)
		(by_title[title]["entries"] as Array).append_array(group["entries"])
	var result: Array = []
	for spec in SECTIONS:
		if by_title.has(spec["title"]):
			var group: Dictionary = by_title[spec["title"]]
			group["entries"] = sort_entries(group["entries"])
			result.append(group)
			order.erase(spec["title"])
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
	_update_vignette()
	for group in groups:
		var heading := UiBuild.label("%s  %s" % [group["glyph"], group["title"]])
		UiType.apply(heading, UiType.HEADING)
		heading.add_theme_color_override("font_color", HudStyle.RUBRIC)
		list_box.add_child(heading)
		var entries: Array = group["entries"]
		for index in mini(entries.size(), MAX_ENTRIES_PER_GROUP):
			list_box.add_child(_entry_row(entries[index]))
		if entries.size() > MAX_ENTRIES_PER_GROUP:
			var more := Label.new()
			var hidden := entries.size() - MAX_ENTRIES_PER_GROUP
			more.text = "… et %d autre%s (voir le journal)" % [hidden, "s" if hidden > 1 else ""]
			UiType.apply(more, UiType.CAPTION)
			list_box.add_child(more)


## AR1 : vignette du genre d'événement le plus prioritaire (`data/ui/illustrations.json`).
func _update_vignette() -> void:
	var entries: Array = []
	for group in groups:
		entries.append_array(group["entries"])
	var chosen := ArtPlates.vignette_for_events(entries)
	vignette_id = str(chosen.get("id", ""))
	var texture := ArtPlates.texture(chosen) if not chosen.is_empty() else null
	vignette.texture = texture
	vignette.visible = texture != null
	vignette_caption.text = str(chosen.get("title", ""))
	vignette_caption.visible = texture != null


## Hauteur ajustée au contenu (au plus `MAX_LIST_HEIGHT`, défilement au-delà).
func _fit_height() -> void:
	var list_height := minf(list_box.get_combined_minimum_size().y, MAX_LIST_HEIGHT)
	if is_inside_tree():
		# Q3: with the AR1 vignette the report overflowed a 720p window and hid « Continuer ».
		scroll.custom_minimum_size.y = 0.0
		var room := get_viewport_rect().size.y - position.y - get_combined_minimum_size().y - 12.0
		list_height = minf(list_height, maxf(room, 80.0))
	scroll.custom_minimum_size.y = list_height
	reset_size()


## Libellé du bouton d'action d'une ligne, vide si aucune action.
static func action_label(event: Dictionary) -> String:
	var kind := str(event.get("kind", ""))
	if kind == "technology_researched":
		return "Technologies"
	if kind == "summary":
		return "Finances" if str(event.get("action", "")) == "faction" else ""
	if kind == "building_completed" and str(event.get("province", "")) != "":
		return "Ville"
	if str(event.get("army", "")) != "" or str(event.get("province", "")) != "":
		return "Voir ⌖"
	return ""


func _entry_row(event: Dictionary) -> Control:
	var tone := str(event.get("_tone", ""))
	var row := UiBuild.hbox(6)
	var mark := UiBuild.label("▼" if tone == TONE_LOSS else ("▲" if tone == TONE_GAIN else "•"))
	UiType.apply(mark, UiType.CAPTION)
	mark.custom_minimum_size = Vector2(16, 0)
	row.add_child(mark)
	var label := UiBuild.label(str(event.get("text_fr", "")), 0, null, true, 380)
	label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	UiType.apply(label, UiType.BODY)
	var ink := HudStyle.RUBRIC if tone == TONE_LOSS else (Money.GAIN_COLOR if tone == TONE_GAIN else HudStyle.INK)
	label.add_theme_color_override("font_color", ink)
	mark.add_theme_color_override("font_color", ink)
	row.add_child(label)
	var action := action_label(event)
	if action != "":
		var button := UiBuild.button(action)
		RichTooltip.attach_plain(button, "season_event_action", {"title": "Aller voir" if action.begins_with("Voir") else "Ouvrir : %s" % action.to_lower()})
		UiType.apply(button, UiType.CAPTION)
		button.size_flags_vertical = Control.SIZE_SHRINK_CENTER
		button.pressed.connect(func() -> void: entry_selected.emit(event))
		row.add_child(button)
	return row


func line_count() -> int:
	return list_box.get_child_count()
