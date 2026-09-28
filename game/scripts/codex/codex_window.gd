class_name CodexWindow
extends PanelContainer

## Fenêtre Codex (H2, touche K) : onglets par grande famille, liste des fiches à gauche (non
## découvertes en grisé ; les ouvrir les découvre), fiche à droite (titre, catégorie, période,
## corps aux mots cliquables avec bulles, liberté prise par le jeu, « Voir aussi », sources),
## recherche, compteur des découvertes, historique précédent / suivant. Construite en code comme
## `ChronicleWindow` ; créée et affichée par `CodexBubbles` (`open_entry`, `window()`).

const INK := Color(0.22, 0.14, 0.07)
const FADED_INK := Color(0.40, 0.30, 0.18)
const RUBRIC := Color(0.55, 0.12, 0.10)
const UNREAD := Color(0.52, 0.46, 0.38)
const SIZE := Vector2(980, 640)
const ALL_TAB := "Toutes"
const HERB_CATEGORY := "plante"
const HERB_HEADER := "__herbier__"
## Miniature en tête de fiche : la sienne, sinon l'image de son `entity` (portraits en entier).
const ART_SIZE := Vector2(0, 220)
const OWN_ART := "res://assets/illustrations/%s.jpg"
const ENTITY_ART := ["res://assets/events/%s.jpg", "res://assets/illustrations/%s.jpg", "res://assets/portraits/%s.png"]

## Lot U11 : vue intégrée à `CodexHub` (onglet « Histoire ») : sans cadre, titre, recherche ni
## bouton de fermeture propres ; la recherche commune passe par `set_query`.
var embedded := false
var _header_title: Label
var _close_button: Button

var current_id: String = ""
var _history: Array = []
var _history_index: int = -1
var _visible_ids: Array = []

var _counter_label: Label
var _back_button: Button
var _forward_button: Button
var _search: LineEdit
var _tabs: TabBar
var _list: ItemList
var _title_label: Label
var _meta_label: Label
var _art: TextureRect
## H11 : « Voir dans l'encyclopédie » quand la fiche a une `entity` connue de l'Encyclopédie.
var encyclopedia_button: Button
var _body: RichTextLabel
## B1 : encadré « En jeu » (champ `gameplay` : comment le jeu modélise le sujet).
var gameplay_box: PanelContainer
var gameplay_label: RichTextLabel
var _anachronism_box: PanelContainer
var _anachronism: RichTextLabel
var _see_also: RichTextLabel
var _sources: Label
var _scroll: ScrollContainer


func _ready() -> void:
	theme = load("res://scenes/ui/parchment_theme.tres")
	mouse_filter = Control.MOUSE_FILTER_STOP
	custom_minimum_size = SIZE if not embedded else Vector2(SIZE.x, 560)
	# P2c : fenêtre seule (`CodexBubbles`, hors `CodexHub`) enregistrée dans `UiZones.Zone.MODAL`
	# par l'appelant (`CodexBubbles.window`), qui centre et assombrit le fond ; rien à faire ici.
	if embedded:
		add_theme_stylebox_override("panel", StyleBoxEmpty.new())
		size_flags_vertical = Control.SIZE_EXPAND_FILL
	var root := VBoxContainer.new()
	root.add_theme_constant_override("separation", 8)
	add_child(root)
	root.add_child(_build_header())

	_search = LineEdit.new()
	_search.placeholder_text = "Rechercher un nom, un lieu, un mot…"
	_search.clear_button_enabled = true
	_search.text_changed.connect(func(_text: String) -> void: _refresh_list())
	# D5 : texte d'aide lisible (encre passée sur parchemin).
	_search.add_theme_color_override("font_placeholder_color", FADED_INK)
	_search.visible = not embedded
	root.add_child(_search)

	_tabs = TabBar.new()
	_tabs.add_tab(ALL_TAB)
	# Libellés courts pour que les 9 onglets tiennent sans défilement à 980 px ; le nom complet
	# de la famille reste en infobulle.
	_tabs.clip_tabs = false
	for family in _families():
		var short_label := str(family[2]) if family.size() > 2 else str(family[0])
		_tabs.add_tab(short_label)
		_tabs.set_tab_tooltip(_tabs.tab_count - 1, str(family[0]))
	_tabs.tab_changed.connect(func(_tab: int) -> void: _refresh_list())
	CodexHub.style_tabs(_tabs, UiType.CAPTION, 9)  # D5 : onglets parchemin
	root.add_child(_tabs)

	var split := HBoxContainer.new()
	split.add_theme_constant_override("separation", 12)
	split.size_flags_vertical = Control.SIZE_EXPAND_FILL
	root.add_child(split)
	_list = ItemList.new()
	_list.custom_minimum_size = Vector2(270, 0)
	UiType.apply(_list, UiType.BODY)
	_list.item_selected.connect(func(index: int) -> void:
		if str(_list.get_item_metadata(index)) != "":
			navigate.call_deferred(str(_list.get_item_metadata(index))))
	split.add_child(_list)
	split.add_child(VSeparator.new())
	_scroll = ScrollContainer.new()
	_scroll.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	split.add_child(_scroll)
	_scroll.add_child(_build_page())
	_update_history_buttons()


func _build_header() -> Control:
	var header := HBoxContainer.new()
	header.add_theme_constant_override("separation", 8)
	var title := Label.new()
	title.text = "✠ Codex"
	UiType.apply(title, UiType.TITLE)
	title.add_theme_color_override("font_color", RUBRIC)
	title.visible = not embedded
	_header_title = title
	header.add_child(title)
	_counter_label = Label.new()
	UiType.apply(_counter_label, UiType.CAPTION)
	_counter_label.add_theme_color_override("font_color", FADED_INK)
	_counter_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_counter_label.size_flags_vertical = Control.SIZE_SHRINK_CENTER
	header.add_child(_counter_label)
	_back_button = _header_button("◀", "Fiche précédente", back)
	header.add_child(_back_button)
	_forward_button = _header_button("▶", "Fiche suivante", forward)
	header.add_child(_forward_button)
	_close_button = _header_button("×", "Fermer (Échap)", hide)
	_close_button.visible = not embedded
	header.add_child(_close_button)
	return header


func _header_button(text: String, tip: String, action: Callable) -> Button:
	var button := Button.new()
	button.text = text
	button.tooltip_text = tip
	button.pressed.connect(action)
	return button


func _build_page() -> Control:
	var page := VBoxContainer.new()
	page.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	page.add_theme_constant_override("separation", 8)
	_title_label = Label.new()
	UiType.apply(_title_label, UiType.TITLE)
	_title_label.add_theme_color_override("font_color", INK)
	_title_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	page.add_child(_title_label)
	_meta_label = Label.new()
	UiType.apply(_meta_label, UiType.CAPTION)
	_meta_label.add_theme_color_override("font_color", FADED_INK)
	page.add_child(_meta_label)
	_art = TextureRect.new()
	_art.custom_minimum_size = ART_SIZE
	_art.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
	_art.clip_contents = true
	_art.hide()
	page.add_child(_art)
	encyclopedia_button = Button.new()
	encyclopedia_button.text = "Voir la fiche de règles"
	encyclopedia_button.tooltip_text = "Onglet Règles : la fiche de jeu (touche L)"
	encyclopedia_button.size_flags_horizontal = Control.SIZE_SHRINK_BEGIN
	encyclopedia_button.hide()
	encyclopedia_button.pressed.connect(open_in_encyclopedia)
	page.add_child(encyclopedia_button)
	page.add_child(HSeparator.new())
	_body = _rich_text(UiType.BODY)
	page.add_child(_body)

	gameplay_box = PanelContainer.new()
	gameplay_box.name = "GameplayBox"
	var gameplay_style := StyleBoxFlat.new()
	gameplay_style.bg_color = Color(0.80, 0.84, 0.72, 1)  # vert de gris : règles, pas histoire
	gameplay_style.border_color = Color(0.25, 0.36, 0.20)
	gameplay_style.set_border_width_all(1)
	gameplay_style.border_width_left = 4
	gameplay_style.set_corner_radius_all(3)
	gameplay_style.set_content_margin_all(10)
	gameplay_box.add_theme_stylebox_override("panel", gameplay_style)
	gameplay_label = _rich_text(UiType.CAPTION)
	gameplay_box.add_child(gameplay_label)
	page.add_child(gameplay_box)

	_anachronism_box = PanelContainer.new()
	var style := StyleBoxFlat.new()
	style.bg_color = Color(0.88, 0.80, 0.62, 1)
	style.border_color = RUBRIC
	style.border_width_left = 4
	style.set_content_margin_all(10)
	_anachronism_box.add_theme_stylebox_override("panel", style)
	_anachronism = _rich_text(UiType.CAPTION)
	_anachronism_box.add_child(_anachronism)
	page.add_child(_anachronism_box)

	_see_also = _rich_text(UiType.BODY)
	page.add_child(_see_also)
	_sources = Label.new()
	UiType.apply(_sources, UiType.CAPTION)
	_sources.add_theme_color_override("font_color", FADED_INK)
	_sources.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	page.add_child(_sources)
	return page


func _rich_text(variation: String) -> RichTextLabel:
	var label := RichTextLabel.new()
	label.bbcode_enabled = true
	label.fit_content = true
	label.scroll_active = false
	label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	label.add_theme_color_override("default_color", INK)
	UiType.apply(label, variation)
	for key in ["bold_font_size", "italics_font_size", "bold_italics_font_size"]:
		label.add_theme_font_size_override(key, UiType.size(variation))
	var bubbles := _bubbles()
	if bubbles != null:
		bubbles.call("attach", label)
	return label


# --- API -------------------------------------------------------------------------------------


## Affiche la fenêtre (centrée) sur la fiche `id`, ou sur la dernière consultée si vide.
func open(id: String = "") -> void:
	show()
	# P2c : plus de centrage manuel ici — la fenêtre seule (non `embedded`) est enregistrée dans
	# `UiZones.Zone.MODAL` par l'appelant (`CodexBubbles.window`), qui la centre par ancrage.
	if id != "":
		navigate(id)
	elif current_id == "":
		_refresh_list()
		if not _visible_ids.is_empty():
			navigate(str(_visible_ids[0]))
	else:
		_refresh_list()


## Affiche la fiche `id` (la découvre) et l'ajoute à l'historique.
func navigate(id: String, record: bool = true) -> void:
	var codex := _store()
	if codex == null or not bool(codex.call("has_entry", id)):
		return
	if record and (current_id != id):
		_history.resize(_history_index + 1)
		_history.append(id)
		_history_index = _history.size() - 1
	codex.call("discover", id)
	current_id = id
	_show_entry(id)
	if not _visible_ids.has(id):
		_tabs.current_tab = 0
	_refresh_list()
	_update_history_buttons()


func back() -> void:
	if _history_index > 0:
		_history_index -= 1
		navigate(str(_history[_history_index]), false)


func forward() -> void:
	if _history_index < _history.size() - 1:
		_history_index += 1
		navigate(str(_history[_history_index]), false)


## Recherche commune (U11) : même effet que la saisie dans le champ de la fenêtre.
func set_query(text: String) -> void:
	if _search.text != text:
		_search.text = text
	# Recherche sur toutes les familles, pour ne rien cacher derrière un onglet.
	if text.strip_edges() != "" and _tabs.current_tab != 0:
		_tabs.current_tab = 0
	_refresh_list()


## Nombre de fiches (toutes familles) qui répondent à la recherche courante.
func match_count() -> int:
	var codex := _store()
	if codex == null:
		return 0
	var query := _search.text.strip_edges().to_lower()
	var count := 0
	for id in codex.call("ids_in_family", -1):
		if query == "" or _matches(codex.call("entry", id), query):
			count += 1
	return count


func counter_text() -> String:
	return _counter_label.text


func body_label() -> RichTextLabel:
	return _body


# --- Contenu ---------------------------------------------------------------------------------


func _show_entry(id: String) -> void:
	var codex := _store()
	var entry: Dictionary = codex.call("entry", id)
	_title_label.text = str(entry.get("title", id))
	var meta := PackedStringArray([str(codex.call("category_label", id))])
	var era := str(codex.call("era_label", id))
	if era != "":
		meta.append(era)
	_meta_label.text = " · ".join(meta)
	var art_path := art_path_of(id, str(entry.get("entity", "")))
	_art.texture = PortraitLoader.load_texture(art_path) if art_path != "" else null
	_art.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_CENTERED if art_path.ends_with(".png") else TextureRect.STRETCH_KEEP_ASPECT_COVERED
	_art.visible = _art.texture != null
	encyclopedia_button.visible = _encyclopedia() != null and not Encyclopedia.definition_of(str(entry.get("entity", ""))).is_empty()
	_body.text = CodexText.format(str(entry.get("body", entry.get("summary", ""))))
	var gameplay := str(entry.get("gameplay", ""))
	gameplay_box.visible = gameplay != ""
	gameplay_label.text = "[b]En jeu[/b]\n%s" % CodexText.format(gameplay)
	var anachronism := str(entry.get("anachronism", ""))
	_anachronism_box.visible = anachronism != ""
	_anachronism.text = "[b]Liberté prise par le jeu[/b]\n%s" % CodexText.format(anachronism)
	var links := PackedStringArray()
	for other in entry.get("see_also", []):
		if bool(codex.call("has_entry", str(other))):
			links.append(CodexText.link(str(other)))
	_see_also.visible = not links.is_empty()
	_see_also.text = "[b]Voir aussi :[/b] " + " · ".join(links)
	var sources: Array = entry.get("sources", [])
	_sources.text = "Sources (Wikipédia) : " + " ; ".join(PackedStringArray(sources)) if not sources.is_empty() else ""
	_scroll.scroll_vertical = 0


## Chemin de la miniature d'une fiche ("" si aucune) : la sienne, puis celle de son entité.
static func art_path_of(id: String, entity: String) -> String:
	var candidates := [OWN_ART % id]
	if entity != "":
		for pattern in ENTITY_ART:
			candidates.append(pattern % entity)
	for path in candidates:
		if ResourceLoader.exists(path) or FileAccess.file_exists(ProjectSettings.globalize_path(path)):
			return path
	return ""


func _refresh_list() -> void:
	var codex := _store()
	if codex == null or _list == null:
		return
	var ids: Array = codex.call("ids_in_family", _tabs.current_tab - 1)
	var query := _search.text.strip_edges().to_lower()
	_visible_ids.clear()
	_list.clear()
	# H9 : dans un onglet mêlant fiches et plantes (Médecine et herbier), les plantes forment
	# une sous-section « Herbier » en fin de liste.
	var plants: Array = ids.filter(func(id: String) -> bool: return str(codex.call("entry", id).get("category", "")) == HERB_CATEGORY)
	var herb_header := _tabs.current_tab > 0 and not plants.is_empty() and plants.size() < ids.size()
	if herb_header:
		ids = ids.filter(func(id: String) -> bool: return not plants.has(id)) + [HERB_HEADER] + plants
	for id in ids:
		if id == HERB_HEADER:
			var header := _list.add_item("— Herbier —", null, false)
			_list.set_item_custom_fg_color(header, RUBRIC)
			_list.set_item_metadata(header, "")
			continue
		if query != "" and not _matches(codex.call("entry", id), query):
			continue
		_visible_ids.append(id)
		var discovered := bool(codex.call("is_discovered", id))
		var index := _list.add_item(("" if discovered else "✧ ") + str(codex.call("title", id)))  # D5 : à découvrir marqué
		_list.set_item_metadata(index, id)
		_list.set_item_tooltip(index, "" if discovered else "À découvrir…")
		_list.set_item_custom_fg_color(index, INK if discovered else UNREAD)
		if id == current_id:
			_list.select(index)
			_list.ensure_current_is_visible()
	_counter_label.text = "%d / %d découvertes" % [int(codex.call("discovered_count")), int(codex.call("total_count"))]


static func _matches(entry: Dictionary, query: String) -> bool:
	var haystack := "%s %s %s" % [entry.get("title", ""), " ".join(PackedStringArray(entry.get("aliases", []))), entry.get("summary", "")]
	return CodexText.plain(haystack).to_lower().contains(query)


func _update_history_buttons() -> void:
	_back_button.disabled = _history_index <= 0
	_forward_button.disabled = _history_index >= _history.size() - 1


func _families() -> Array:
	var codex := _store()
	return codex.call("families") if codex != null else []


## H11 : ferme le Codex et ouvre l'Encyclopédie sur l'entité de la fiche courante.
func open_in_encyclopedia() -> bool:
	var codex := _store()
	var entity := str((codex.call("entry", current_id) as Dictionary).get("entity", "")) if codex != null else ""
	var encyclopedia := _encyclopedia()
	if encyclopedia == null or entity == "":
		return false
	hide()
	encyclopedia.call("open_window", entity)
	return true


func _encyclopedia() -> Node:
	return get_tree().get_first_node_in_group("encyclopedia") if is_inside_tree() else null


func _store() -> Node:
	return CodexText.store()


func _bubbles() -> Node:
	return get_node_or_null("/root/CodexBubbles")
