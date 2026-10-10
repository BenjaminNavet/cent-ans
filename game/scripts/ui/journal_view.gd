class_name JournalView
extends RefCounted

## Journal des événements de la carte de campagne (extrait de `MapUI`) : filtre d'intérêt et de
## pertinence, mise en forme BBCode, onglet « Monde » (nouvelles lointaines, replié), titre et
## bascule. `MapUI` garde les points d'entrée publics et délègue ici.

const MAX_LOG_LINES := 200
const RELEVANCE_FAR := "far"
const RELEVANCE_KEY := "relevance"

## Événements d'une autre faction sans intérêt pour le joueur (gestion interne).
const FOREIGN_MINOR_KINDS := [
	"income", "bankruptcy", "attrition", "recruited", "building_completed", "trait_acquired",
	"skill_learned", "appointment", "technology_researched", "regency", "birth", "raid",
]

# Styles de ligne : `color`, `bold`, `glyph` (repli Unicode), `icon` (glyphe à l'encre, « » = aucun).
const _WAR := {"color": "#8b1a1a", "bold": true, "glyph": "⚔", "icon": "glyph_swords"}
const _OFFICE := {"color": "#4a3a10", "bold": false, "glyph": "⚑", "icon": "glyph_flag"}
const _ALLIANCE := {"color": "#1a3a8b", "bold": true, "glyph": "⚜", "icon": "glyph_fleur"}
const _MAIL := {"color": "#4a3a10", "bold": false, "glyph": "✉", "icon": "glyph_letter"}
const _FAITH := {"color": "#5a2a6a", "bold": true, "glyph": "✠", "icon": "glyph_cross"}
const _SKILL := {"color": "#2a5a7a", "bold": false, "glyph": "★", "icon": "glyph_star"}
const _SUCCESSION := {"color": "#7a5a10", "bold": true, "glyph": "♔", "icon": ""}

## Événement (`kind`) → style de sa ligne ; les genres absents passent par `SeasonReport.KIND_STYLES`
## puis restent en texte brut.
const JOURNAL_STYLES := {
	"battle": _WAR, "siege_started": _WAR, "province_captured": _WAR, "war_declared": _WAR,
	"revolt": {"color": "#a1121a", "bold": true, "glyph": "⚑", "icon": "glyph_flag"},
	"plague": {"color": "#4a6b2a", "bold": true, "glyph": "☠", "icon": ""},
	"famine": {"color": "#8a5a10", "bold": true, "glyph": "⚠", "icon": ""},
	"building_completed": {"color": "#1a5c8b", "bold": false, "glyph": "⚒", "icon": "glyph_hammer"},
	"birth": {"color": "#2a7a4a", "bold": false, "glyph": "✚", "icon": ""},
	"marriage": {"color": "#8a3a8a", "bold": true, "glyph": "♥", "icon": ""},
	"death": {"color": "#3a3a3a", "bold": true, "glyph": "✝", "icon": ""},
	"succession": _SUCCESSION,
	"regency": {"color": "#7a5a10", "bold": false, "glyph": "⚖", "icon": ""},
	"trait_acquired": {"color": "#2a5a7a", "bold": false, "glyph": "✦", "icon": ""},
	"skill_learned": _SKILL,
	"appointment": _OFFICE,
	"peace_signed": {"color": "#2a6a2a", "bold": true, "glyph": "☮", "icon": ""},
	"alliance_formed": _ALLIANCE, "vassalage": _ALLIANCE,
	"alliance_broken": {"color": "#a1121a", "bold": true, "glyph": "⚡", "icon": ""},
	"vassal_rebellion": {"color": "#a1121a", "bold": true, "glyph": "⚡", "icon": ""},
	"embargo": _MAIL, "diplomatic_offer": _MAIL, "diplomacy": _MAIL,
	"trade": {"color": "#4a3a10", "bold": false, "glyph": "⚓", "icon": ""},  # C5
	"excommunication": _FAITH, "schism": _FAITH, "heresy": _FAITH,
	"chronicle": {"color": "#7a3b0c", "bold": true, "glyph": "§", "icon": ""},  # M10
	"technology_researched": {"color": "#5a2a8a", "bold": true, "glyph": "⚙", "icon": ""},
	"income": {"color": "#4a3a10", "bold": false, "glyph": "", "icon": ""},
}

## WR turn : filtre par genre (guerre, diplomatie, économie…), défini dans `data/ui/journal_genres.json`.
## Purement visuel : les lignes restent toutes gardées ; le choix est retenu pour la session.
const GENRES_PATH := "ui/journal_genres.json"
static var _genres_lookup := JsonLookup.new(GENRES_PATH)
static var remembered_genre := "all"

## Faction du joueur et nom court d'une faction (`Callable(id) -> String`) : le journal
## masque les événements courants des autres factions et nomme la faction des autres.
var player_faction: String = ""
var faction_name: Callable = Callable()

## Filtre d'intérêt des lettres et du bandeau (voisins, alliés, ennemis, grandes
## puissances), recalculé en fin de tour par `HudController.update_interest` ; nul = tout passe.
var interest: NewsInterest = null

## A6-L6 (U6/U7) : pertinence d'un événement pour le joueur, calculée par le cœur
## (`CampaignSim.classify_news` : sa faction, suzerain/vassaux, alliés, ennemis en guerre,
## provinces voisines). Clés : `player`, `related`, `neighbor`, `far` (« Monde »). Le classeur
## est posé par `HudController.update_interest` ; nul (maquette, ancien cœur) = repli sur
## `interest`.
var classifier: Callable = Callable()

var lines: PackedStringArray = PackedStringArray()
var world_lines: PackedStringArray = PackedStringArray()
## Genre de chaque ligne de `lines` / `world_lines` (même indice ; "" = ligne de date ou de repli).
var line_genres: PackedStringArray = PackedStringArray()
var world_genres: PackedStringArray = PackedStringArray()
var genre_filter := "all"
var genre_bar: HBoxContainer
var world_open := false
var world_toggle: Button
var world_scroll: ScrollContainer
var world_text: RichTextLabel

var _title: Label
var _toggle: Button
var _scroll: ScrollContainer
var _text: RichTextLabel
var _letters: NewsLetters
var _relayout: Callable  # rappelé quand la hauteur du journal change (`MapUI.queue_layout`)


func _init(title: Label, toggle: Button, scroll: ScrollContainer, text: RichTextLabel,
		letters: NewsLetters, relayout: Callable) -> void:
	_title = title
	_toggle = toggle
	_scroll = scroll
	_text = text
	_letters = letters
	_relayout = relayout
	_toggle.pressed.connect(func() -> void: set_expanded(not _scroll.visible))
	genre_filter = remembered_genre
	_build_genre_bar()
	_build_world_tab()


## Genre de filtre d'un genre d'événement (`kind`) : identifiant de `journal_genres.json`.
static func genre_of(kind: String) -> String:
	var config := _genres_lookup.data()
	for genre: Dictionary in config.get("genres", []):
		if kind in genre.get("kinds", []):
			return str(genre.get("id", ""))
	return str(config.get("other", "other"))


## Genres proposés par le filtre : `[{id, label}]`, « Tout » en tête, « Autre » en queue si utilisé.
static func genre_choices() -> Array:
	var config := _genres_lookup.data()
	var choices: Array = [{"id": "all", "label": str(config.get("all_label", "Tout"))}]
	for genre: Dictionary in config.get("genres", []):
		choices.append({"id": str(genre.get("id", "")), "label": str(genre.get("label", ""))})
	return choices


## Choisit le genre affiché (`all` = tout) et le retient pour la session.
func set_genre_filter(genre_id: String) -> void:
	genre_filter = genre_id
	remembered_genre = genre_id
	_sync_genre_bar()
	_render()
	_relayout.call()


## Lignes visibles sous le filtre : une date n'est gardée que si une ligne du genre la suit.
static func filter_lines(source: PackedStringArray, genres: PackedStringArray, genre_id: String) -> PackedStringArray:
	if genre_id == "all":
		return source
	var out := PackedStringArray()
	var pending_header := ""
	for i in source.size():
		var genre := genres[i] if i < genres.size() else ""
		if genre == "":
			pending_header = source[i]
		elif genre == genre_id:
			if pending_header != "":
				out.append(pending_header)
				pending_header = ""
			out.append(source[i])
	return out


func _build_genre_bar() -> void:
	var box := _scroll.get_parent()
	genre_bar = HBoxContainer.new()
	genre_bar.name = "GenreBar"
	genre_bar.add_theme_constant_override("separation", 2)
	genre_bar.visible = false
	for choice: Dictionary in genre_choices():
		var button := Button.new()
		button.name = "Genre_%s" % choice["id"]
		button.text = str(choice["label"])
		button.toggle_mode = true
		button.flat = true
		button.focus_mode = Control.FOCUS_NONE
		button.add_theme_font_size_override("font_size", 11)
		button.pressed.connect(set_genre_filter.bind(str(choice["id"])))
		genre_bar.add_child(button)
	box.add_child(genre_bar)
	box.move_child(genre_bar, _scroll.get_index())
	_sync_genre_bar()


func _sync_genre_bar() -> void:
	for child in genre_bar.get_children():
		if child is Button:
			(child as Button).button_pressed = child.name == "Genre_%s" % genre_filter


## Marque chaque événement de sa pertinence (un seul appel au cœur pour tout le lot).
func ensure_relevance(events: Array) -> void:
	if not classifier.is_valid() or events.is_empty():
		return
	var pending: Array = []
	for event in events:
		if event is Dictionary and not (event as Dictionary).has(RELEVANCE_KEY):
			pending.append(event)
	if pending.is_empty():
		return
	var classes: PackedStringArray = classifier.call(pending)
	for i in mini(pending.size(), classes.size()):
		(pending[i] as Dictionary)[RELEVANCE_KEY] = classes[i]


## Pertinence d'un événement (`player`, `related`, `neighbor`, `far`) ; `player` quand elle est
## inconnue (aucun classeur) pour ne rien cacher par erreur.
func relevance_of(event: Dictionary) -> String:
	if not event.has(RELEVANCE_KEY):
		if not classifier.is_valid():
			return "player" if interest == null or interest.keeps(event) else RELEVANCE_FAR
		ensure_relevance([event])
	return str(event.get(RELEVANCE_KEY, "player"))


static func relevance_label(relevance: String) -> String:
	match relevance:
		"player":
			return "Votre royaume"
		"related":
			return "Allié, ennemi ou vassal"
		"neighbor":
			return "Voisin"
	return "Nouvelle lointaine"


## Vrai si la nouvelle mérite une lettre ou le bandeau du haut (le journal garde tout).
func keeps_news(event: Dictionary) -> bool:
	if SeasonReport.is_public(event):  # Nouvelle publique (champ `public`), lue par tous
		return true
	var mode := interest.mode if interest != null else NewsInterest.MODE_INTEREST
	if mode == NewsInterest.MODE_ALL:
		return true
	var relevance := relevance_of(event)
	if mode == NewsInterest.MODE_OWN:
		return relevance == "player"
	return relevance != RELEVANCE_FAR


## Vrai si l'événement doit figurer au journal du joueur.
func keeps(event: Dictionary) -> bool:
	var faction := str(event.get("faction", ""))
	if faction == "" or player_faction == "" or faction == player_faction:
		return true
	if SeasonReport.is_private_crusade(event, player_faction):  # JR3
		return false
	return not FOREIGN_MINOR_KINDS.has(str(event.get("kind", "")))


## Texte de l'événement, préfixé du nom de sa faction s'il ne la nomme pas déjà.
func text_of(event: Dictionary) -> String:
	var text := str(event.get("text_fr", event.get("text", "")))
	var faction := str(event.get("faction", ""))
	if text == "" or faction == "" or faction == player_faction or not faction_name.is_valid():
		return text
	var name := str(faction_name.call(faction))
	if name == "" or text.contains(name):
		return text
	return "%s — %s" % [name, text]


## Ajoute les événements d'un tour en tête du journal (plus récents en haut). A6-L6 (U6/U7) :
## ce qui ne concerne pas le joueur (calcul du cœur, `classify_news`) va dans l'onglet « Monde »,
## replié, et ne pousse aucune lettre.
func add_events(events: Array, date_text: String) -> void:
	ensure_relevance(events)
	var new_lines := PackedStringArray()
	var new_world := PackedStringArray()
	var new_genres := PackedStringArray()
	var new_world_genres := PackedStringArray()
	var new_news: Array = []  # lettres du tour, poussées en un seul lot (une reconstruction, un son)
	for event in events:
		if not keeps(event):
			continue
		var text: String = text_of(event)
		var is_world := relevance_of(event) == RELEVANCE_FAR and not SeasonReport.is_public(event)
		var news := NewsLetters.news_from_event(event)  # Lettre scellée (trace persistante)
		if not news.is_empty() and keeps_news(event):  # Filtre d'intérêt (« Toute l'Europe » garde tout)
			news["interest"] = relevance_label(relevance_of(event))
			new_news.append(news)
		if text == "":
			continue
		var line := format_line(event, CodexText.format(text, true))  # Liens du Codex
		var genre := genre_of(str(event.get("kind", "")))
		if is_world:
			new_world.append(line)
			new_world_genres.append(genre)
		else:
			new_lines.append(line)
			new_genres.append(genre)
	_letters.push_news_batch(new_news)
	if new_lines.is_empty():
		new_lines.append("[i]Rien à signaler.[/i]")
		new_genres.append("")
	lines = _with_header(new_lines, date_text, lines)
	line_genres = _with_header_genres(new_genres, line_genres)
	if not new_world.is_empty():
		world_lines = _with_header(new_world, date_text, world_lines)
		world_genres = _with_header_genres(new_world_genres, world_genres)
	_render()
	_title.text = "Journal (%d)" % new_lines.size()


## Genres parallèles à `_with_header` (la ligne de date a le genre "").
static func _with_header_genres(new_genres: PackedStringArray, previous: PackedStringArray) -> PackedStringArray:
	var block := PackedStringArray([""])
	block.append_array(new_genres)
	block.append_array(previous)
	return block.slice(0, mini(block.size(), MAX_LOG_LINES))


## Bloc daté `new_lines` placé en tête de `previous`, borné à `MAX_LOG_LINES`.
static func _with_header(new_lines: PackedStringArray, date_text: String, previous: PackedStringArray) -> PackedStringArray:
	var block := PackedStringArray(["[b]— %s —[/b]" % date_text])
	block.append_array(new_lines)
	block.append_array(previous)
	return block.slice(0, mini(block.size(), MAX_LOG_LINES))


## Ligne mise en forme (BBCode) d'un événement du journal (`JOURNAL_STYLES`).
static func format_line(event: Dictionary, text: String) -> String:
	var kind := str(event.get("kind", ""))
	var style: Dictionary = JOURNAL_STYLES.get(kind, {})
	if style.is_empty() and SeasonReport.KIND_STYLES.has(kind):  # Table, médecine, monnaie, rançon, chevalerie
		var season: Dictionary = SeasonReport.KIND_STYLES[kind]
		return "[color=%s]%s %s[/color]" % [season["color"], InkGlyph.bbcode(str(season.get("icon", "")), str(season["glyph"])), text]
	if style.is_empty():
		return text
	var glyph := str(style["glyph"])
	var body := text
	if glyph != "":
		body = "%s %s" % [InkGlyph.bbcode(str(style["icon"]), glyph), text]
	if bool(style["bold"]):
		body = "[b]%s[/b]" % body
	return "[color=%s]%s[/color]" % [style["color"], body]


func clear() -> void:
	lines = PackedStringArray()
	world_lines = PackedStringArray()
	line_genres = PackedStringArray()
	world_genres = PackedStringArray()
	_render_world()
	_letters.clear()
	_text.text = "[i]Aucun événement pour l’instant.[/i]"
	_title.text = "Journal"


func _render() -> void:
	var shown := filter_lines(lines, line_genres, genre_filter)
	if shown.is_empty() and not lines.is_empty():
		_text.text = "[i]Rien de ce genre.[/i]"
	else:
		_text.text = "\n".join(shown)
	genre_bar.visible = _scroll.visible
	_scroll.scroll_vertical = 0
	_render_world()


## Onglet « Monde » : bouton de dépli (visible journal déplié, s'il y a des nouvelles lointaines).
func _build_world_tab() -> void:
	var box := _scroll.get_parent()
	world_toggle = Button.new()
	world_toggle.name = "WorldToggle"
	world_toggle.flat = true
	world_toggle.alignment = HORIZONTAL_ALIGNMENT_LEFT
	world_toggle.focus_mode = Control.FOCUS_NONE
	world_toggle.pressed.connect(func() -> void:
		world_open = not world_open
		_render_world()
		_relayout.call())
	box.add_child(world_toggle)
	world_scroll = ScrollContainer.new()
	world_scroll.name = "WorldScroll"
	world_scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	world_scroll.custom_minimum_size = Vector2(0, 120)
	world_text = RichTextLabel.new()
	world_text.bbcode_enabled = true
	world_text.fit_content = true
	world_text.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	world_text.add_theme_color_override("default_color", Color(0.30, 0.24, 0.16))
	world_scroll.add_child(world_text)
	box.add_child(world_scroll)
	_render_world()


func _render_world() -> void:
	if world_toggle == null:
		return
	var has_world := not world_lines.is_empty()
	var expanded := _scroll.visible
	world_toggle.visible = expanded and has_world
	world_toggle.text = "%s Monde (%d)" % ["▾" if world_open else "▸", world_line_count()]
	world_scroll.visible = expanded and has_world and world_open
	world_text.text = "\n".join(filter_lines(world_lines, world_genres, genre_filter))


## Nombre de nouvelles lointaines gardées (sans les lignes de date).
func world_line_count() -> int:
	var count := 0
	for line in world_lines:
		if not line.begins_with("[b]—"):
			count += 1
	return count


## Journal déplié ou replié (replié par défaut : le sceau et le bandeau occupent le bas).
func set_expanded(expanded: bool) -> void:
	_scroll.visible = expanded
	_toggle.text = "Replier" if expanded else "Déplier"
	genre_bar.visible = expanded
	_render_world()
	_relayout.call()


func line_count() -> int:
	return lines.size()
