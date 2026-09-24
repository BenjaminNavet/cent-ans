class_name Encyclopedia
extends Control

## F8 — encyclopédie (touche L ou Menu → Encyclopédie) : fenêtre parchemin à onglets (Unités,
## Bâtiments, Technologies, Ressources, Traits, Compétences, Factions, Religion, Mécaniques),
## liste filtrée par la recherche, fiche détaillée avec icônes (`IconLibrary`) et liens
## internes (clic sur un prérequis, un déblocage, une religion… ouvre sa fiche).
##
## Tout vient de `data/` (`GameCatalog.definitions`, mêmes JSON que `GameDataStore`) :
## aucune règle ici, seulement de la mise en forme. Les textes de l'onglet « Mécaniques »
## décrivent les règles de `core/` sans les calculer ; ils citent les valeurs des données
## (objectifs, lois de succession, échéances) quand elles existent.
##
## Autoloads par `/root/...` (le smoke `--script` est compilé avant leur enregistrement).

signal closed

const THEME_PATH := "res://scenes/ui/parchment_theme.tres"
const MUTED := "#6b5a40"
const LINK := "#1a3a8b"
## Miniatures facultatives des unités, bâtiments et technologies (OpenRouter,
## `cent-ans assets illustrations`) : `<dossier><identifiant>.jpg`, en tête de fiche.
const ILLUSTRATIONS_DIR := "res://assets/illustrations/"
const ILLUSTRATION_WIDTH := 660

## Onglets : identifiant, libellé, dossier de `data/` (vide : entrées construites ici),
## préfixe des identifiants, icône de l'onglet.
const TABS := [
	{"id": "units", "label": "Unités", "dir": "unit_types", "prefix": "unit_", "icon": "cat_unit"},
	{"id": "buildings", "label": "Bâtiments", "dir": "buildings", "prefix": "bld_", "icon": "cat_building"},
	{"id": "technologies", "label": "Technologies", "dir": "technologies", "prefix": "tech_", "icon": "cat_technology"},
	{"id": "resources", "label": "Ressources", "dir": "resources", "prefix": "res_", "icon": "cat_resource"},
	{"id": "traits", "label": "Traits", "dir": "traits", "prefix": "trait_", "icon": "cat_trait"},
	{"id": "skills", "label": "Compétences", "dir": "skills", "prefix": "skill_", "icon": "cat_skill"},
	{"id": "factions", "label": "Factions", "dir": "factions", "prefix": "fac_", "icon": "hud_diplomacy"},
	{"id": "religions", "label": "Religion", "dir": "religions", "prefix": "rel_", "icon": "bld_parish_church"},
	{"id": "mechanics", "label": "Mécaniques", "dir": "", "prefix": "mech_", "icon": "hud_menu"},
]

const GOVERNMENT_LABELS := {
	"kingdom": "royaume", "duchy": "duché", "county": "comté", "republic": "république",
	"empire": "empire", "theocracy": "théocratie", "confederation": "confédération",
	"lordship": "seigneurie",
}
const SUCCESSION_LABELS := {
	"salic": "loi salique (héritiers mâles par les mâles)",
	"male_preference_primogeniture": "primogéniture à préférence masculine",
	"cognatic_primogeniture": "primogéniture cognatique", "elective": "élective",
}
const RELIGION_KIND_LABELS := {"church": "Église", "heresy": "hérésie", "obedience": "obédience", "other_faith": "autre foi"}

## Onglet « Mécaniques » : fiches explicatives (texte d'interface ; les règles vivent dans
## `core/`). `extra` : fiche complétée par des valeurs des données (voir `_mechanic_extra`).
const MECHANICS := [
	{"id": "mech_economy", "name": "Économie", "icon": "hud_treasury", "text": "Chaque saison, les provinces rapportent l'impôt (selon la population de chaque classe, sa richesse et le taux d'imposition choisi dans le panneau de faction) et le commerce (bâtiments de commerce, ressources). Cet impôt est réparti entre les colonies de la province au prorata de leur poids, une colonie assiégée n'en touchant rien ; tenir toutes les colonies d'une province ajoute un bonus de province complète. On en retire l'entretien des armées, des garnisons et des bâtiments (payé en partie par la couronne selon le type de colonie) et les frais de cour et d'administration, qui croissent avec la taille du royaume et quand le trésor dort. En dette, les troupes perdent du moral et se débandent : licenciez ou baissez les dépenses.\n\nLes quatre classes (paysans, bourgeois, clergé, noblesse) ont chacune un mécontentement, une santé, une richesse et une satisfaction en biens. Un fardeau fiscal trop lourd, la dévastation, l'occupation étrangère ou une religion différente attisent le mécontentement ; au-delà du seuil de révolte, la province se soulève.", "extra": "classes"},
	{"id": "mech_morale", "name": "Moral", "icon": "gauge_morale", "text": "Le moral d'une unité baisse sous le tir, en mêlée, quand elle est prise de flanc ou de dos, quand ses voisines fuient ou que le général tombe. Au plus bas, elle rompt et fuit ; elle peut se rallier loin de l'ennemi. En campagne, la dette, la famine et les défaites entament le moral des armées ; le repos en territoire ami le rétablit. Certains traits (chevaleresque…) et compétences du général le relèvent."},
	{"id": "mech_supply", "name": "Ravitaillement", "icon": "gauge_supply", "text": "Chaque armée a des vivres. Ils baissent en territoire hostile ou dévasté, surtout l'hiver, et remontent en territoire ami. Sans vivres, l'armée perd des hommes (attrition). Les bâtiments et les compétences de logistique améliorent le ravitaillement ; la posture « Chevauchée » pille le pays pour vivre sur l'ennemi, au prix de la dévastation.\n\nTraverser la mer se fait entre deux ports ; débarquer en terre ennemie épuise le mouvement et coûte des hommes."},
	{"id": "mech_sieges", "name": "Sièges", "icon": "bld_stone_walls", "text": "Chaque province contient plusieurs colonies prenables séparément (cité, villes, châteaux, abbayes, villages) : une armée en posture « Siège » sur une colonie ennemie fortifiée l'assiège. La place a des vivres : quand ils s'épuisent, elle capitule. Les engins de siège ouvrent une brèche. Le panneau d'armée affiche vivres, brèche et chances d'assaut ; « Donner l'assaut » lance une bataille de siège (3D ou automatique). Les fortifications (palissade, murailles de pierre, château fort, bastion) allongent le siège et renforcent la garnison ; un village sans garnison est pris dès qu'une armée ennemie y entre, sans siège.\n\nEn bataille de siège : échelles, tours de siège et bélier contre la porte ; la victoire revient à qui met la garnison en déroute ou tient la place centrale.\n\nUne armée battue se replie vers la colonie amie la plus proche, sinon une colonie neutre en perdant des traînards, sinon c'est la débandade (lourdes pertes, dispersion possible)."},
	{"id": "mech_battles", "name": "Batailles", "icon": "hud_army", "text": "Quand deux armées ennemies se rencontrent, choisissez « Livrer bataille » (bataille 3D en temps réel) ou la résolution automatique. En 3D : régiments en ligne, colonne, schiltron ou coin ; flancs et arrières vulnérables ; piques contre cavalerie ; pieux des archers ; la pluie gêne arcs et arbalètes, le brouillard réduit la portée. La mort du général fait chuter le moral. Les armées alliées présentes dans la province se joignent à la bataille."},
	{"id": "mech_diplomacy", "name": "Diplomatie", "icon": "hud_diplomacy", "text": "L'attitude de chaque puissance envers vous est calculée (liens, guerres, religion, prétentions, réputation) et le panneau en donne les raisons. Déclarer une guerre sans casus belli ou rompre une trêve coûte en réputation. La paix se négocie selon le score de guerre (cessions, tribut, trêve). Alliances, appels aux armes, embargos, vassalité, mariages entre dynasties et unions personnelles complètent le jeu.\n\nReligion : la faveur pontificale se gagne par la piété et les dons ; l'excommunication isole. Le Grand Schisme (1378-1417) oblige à choisir une obédience.", "extra": "relations"},
	{"id": "mech_succession", "name": "Succession", "icon": "hud_court", "text": "Les personnages vieillissent, se marient, ont des enfants et meurent. À la mort du souverain, l'héritier est désigné par la loi de succession du royaume ; un héritier mineur règne sous régence. Une faction sans héritier voit une nouvelle maison (ou un élu) prendre le pouvoir. Les prétentions dynastiques issues des mariages peuvent donner un casus belli, voire une union personnelle.", "extra": "succession"},
	{"id": "mech_characters", "name": "Personnages", "icon": "hud_governor", "text": "Les personnages gagnent de l'expérience (batailles, gouvernance) et des points de compétence à dépenser dans trois branches : Commandement, Gouvernance, Cour. Leurs traits (personnalité, physique, martial, gouvernance, acquis) modifient batailles, provinces et diplomatie. Nommez des généraux à la tête des armées et des gouverneurs dans les provinces (fiche personnage)."},
	{"id": "mech_technology", "name": "Recherche", "icon": "hud_research", "text": "Les points de recherche viennent d'une base, des bâtiments savants, des technologies et de la gouvernance du souverain. Une technologie en avance de plus de vingt ans sur sa date historique coûte davantage. Le surplus d'une technologie achevée est reporté sur la suivante ; sans recherche choisie, les points de la saison sont perdus."},
	{"id": "mech_chronicle", "name": "Chronique", "icon": "hud_chronicle", "text": "Des événements historiques datés et conditionnels (L'Écluse, Crécy, la Peste noire, la Jacquerie, Azincourt, Jeanne d'Arc…) et des événements aléatoires demandent une décision. Vous avez deux saisons pour choisir, sinon le conseil tranche. Certains choix déclenchent des suites plusieurs saisons plus tard."},
	{"id": "mech_victory", "name": "Victoire", "icon": "hud_end_turn", "text": "Chaque faction jouable a des objectifs historiques à remplir avant une échéance (touche O). Les remplir tous donne la victoire ; perdre toutes ses terres, la défaite ; à l'échéance, la campagne s'achève sur un score.", "extra": "victory"},
]

var tab_bar: TabBar
var search_field: LineEdit
var entry_list: ItemList
var fiche: RichTextLabel
var title_label: Label
var count_label: Label
## H11 : « Fiche historique » (Codex) de l'entité affichée, masqué si elle n'en a pas.
var codex_button: Button

var current_tab: int = 0
var current_entry: String = ""
var query: String = ""
## tab id → Array[{id, name, icon, search}] (trié par nom).
var _entries: Dictionary = {}
var _visible_ids: PackedStringArray = PackedStringArray()
var _history: PackedStringArray = PackedStringArray()


func _ready() -> void:
	name = "Encyclopedia"
	add_to_group("encyclopedia")  # H11 : retrouvée par la fenêtre Codex
	set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	if ResourceLoader.exists(THEME_PATH):
		theme = load(THEME_PATH)
	var center := CenterContainer.new()
	center.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	center.mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_child(center)
	var panel := PanelContainer.new()
	panel.name = "Window"
	panel.custom_minimum_size = Vector2(1060, 680)
	panel.mouse_filter = Control.MOUSE_FILTER_STOP
	center.add_child(panel)
	var box := VBoxContainer.new()
	box.add_theme_constant_override("separation", 8)
	panel.add_child(box)
	var header := HBoxContainer.new()
	header.add_theme_constant_override("separation", 10)
	box.add_child(header)
	title_label = Label.new()
	title_label.text = "Encyclopédie"
	title_label.add_theme_font_size_override("font_size", 24)
	header.add_child(title_label)
	var spacer := Control.new()
	spacer.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	header.add_child(spacer)
	codex_button = Button.new()
	codex_button.name = "CodexButton"
	codex_button.text = "✠ Fiche historique"
	codex_button.tooltip_text = "Ouvrir la fiche du Codex (K)"
	codex_button.hide()
	codex_button.pressed.connect(open_codex_entry)
	header.add_child(codex_button)
	var back := Button.new()
	back.name = "BackButton"
	back.text = "← Retour"
	back.tooltip_text = "Fiche précédente"
	back.pressed.connect(go_back)
	header.add_child(back)
	search_field = LineEdit.new()
	search_field.name = "Search"
	search_field.placeholder_text = "Rechercher…"
	search_field.clear_button_enabled = true
	search_field.custom_minimum_size = Vector2(260, 0)
	search_field.add_theme_color_override("font_placeholder_color", Color(0.45, 0.36, 0.25))
	search_field.add_theme_color_override("font_color", Color(0.22, 0.14, 0.07))
	search_field.text_changed.connect(set_query)
	header.add_child(search_field)
	var close := Button.new()
	close.name = "CloseButton"
	close.text = "×"
	close.tooltip_text = "Fermer (Échap ou K)"
	close.pressed.connect(close_window)
	header.add_child(close)
	tab_bar = TabBar.new()
	tab_bar.name = "Tabs"
	tab_bar.clip_tabs = false
	tab_bar.add_theme_constant_override("icon_max_width", 18)
	var library := _icons()
	for tab in TABS:
		tab_bar.add_tab(str(tab["label"]))
		if library != null:
			tab_bar.set_tab_icon(tab_bar.tab_count - 1, library.call("get_icon", str(tab["icon"])))
	tab_bar.tab_changed.connect(func(index: int) -> void: select_tab(index))
	box.add_child(tab_bar)
	var split := HSplitContainer.new()
	split.size_flags_vertical = Control.SIZE_EXPAND_FILL
	split.split_offset = 0
	box.add_child(split)
	var left := VBoxContainer.new()
	left.custom_minimum_size = Vector2(300, 0)
	split.add_child(left)
	count_label = Label.new()
	count_label.add_theme_font_size_override("font_size", 13)
	count_label.add_theme_color_override("font_color", Color(0.42, 0.35, 0.25))
	left.add_child(count_label)
	entry_list = ItemList.new()
	entry_list.name = "Entries"
	entry_list.size_flags_vertical = Control.SIZE_EXPAND_FILL
	entry_list.fixed_icon_size = Vector2i(22, 22)
	entry_list.add_theme_font_size_override("font_size", 15)
	entry_list.add_theme_color_override("font_selected_color", Color(0.98, 0.94, 0.84))
	var selected := StyleBoxFlat.new()
	selected.bg_color = Color(0.45, 0.28, 0.12)
	entry_list.add_theme_stylebox_override("selected", selected)
	entry_list.add_theme_stylebox_override("selected_focus", selected)
	entry_list.item_selected.connect(func(index: int) -> void:
		if index >= 0 and index < _visible_ids.size():
			open_entry(_visible_ids[index]))
	left.add_child(entry_list)
	fiche = RichTextLabel.new()
	fiche.name = "Fiche"
	fiche.bbcode_enabled = true
	fiche.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	fiche.size_flags_vertical = Control.SIZE_EXPAND_FILL
	fiche.custom_minimum_size = Vector2(700, 0)
	fiche.add_theme_color_override("default_color", Color(0.22, 0.14, 0.07))
	fiche.add_theme_font_size_override("normal_font_size", 15)
	fiche.add_theme_font_size_override("bold_font_size", 16)
	fiche.meta_underlined = true
	fiche.meta_clicked.connect(func(meta: Variant) -> void: open_entry(str(meta)))
	split.add_child(fiche)
	build_entries()
	select_tab(0)


func _input(event: InputEvent) -> void:
	if visible and event.is_action_pressed("ui_cancel"):
		close_window()
		get_viewport().set_input_as_handled()


func toggle() -> void:
	if visible:
		close_window()
	else:
		open_window()


func open_window(entry_id: String = "") -> void:
	show()
	if entry_id != "":
		open_entry(entry_id)
	search_field.grab_focus.call_deferred()


func close_window() -> void:
	if not visible:
		return
	search_field.release_focus()
	hide()
	closed.emit()


# --- Entrées ------------------------------------------------------------------------


static func _icons() -> Node:
	var loop := Engine.get_main_loop() as SceneTree
	return loop.root.get_node_or_null("/root/IconLibrary") if loop != null else null


static func tab_index_of(tab_id: String) -> int:
	for index in TABS.size():
		if TABS[index]["id"] == tab_id:
			return index
	return -1


## Onglet d'un identifiant (`unit_…` → Unités…), -1 si inconnu.
static func tab_of(entry_id: String) -> int:
	for index in TABS.size():
		if entry_id.begins_with(str(TABS[index]["prefix"])):
			return index
	return -1


static func definition_of(entry_id: String) -> Dictionary:
	var index := tab_of(entry_id)
	if index < 0:
		return {}
	var directory := str(TABS[index]["dir"])
	if directory == "":
		for mechanic in MECHANICS:
			if mechanic["id"] == entry_id:
				return mechanic
		return {}
	return GameCatalog.definitions(directory).get(entry_id, {})


static func name_of(entry_id: String) -> String:
	var definition := definition_of(entry_id)
	var name: Variant = definition.get("name", null)
	if name is Dictionary and str(name.get("display", "")) != "":
		return str(name["display"])
	if name is String and name != "":
		return name
	if entry_id.begins_with("chr_"):
		var character: Dictionary = GameCatalog.definitions("characters").get(entry_id, {})
		if character.get("name", null) is Dictionary:
			return str(character["name"].get("display", entry_id))
	return GameCatalog.display_name(entry_id)


## Minuscules sans accents (recherche « epee » → « Épée »).
static func fold(text: String) -> String:
	var lowered := text.to_lower()
	var out := ""
	for character in lowered:
		out += str(FOLD_MAP.get(character, character))
	return out


const FOLD_MAP := {
	"à": "a", "â": "a", "ä": "a", "á": "a", "ç": "c", "é": "e", "è": "e", "ê": "e", "ë": "e",
	"î": "i", "ï": "i", "í": "i", "ô": "o", "ö": "o", "ó": "o", "ù": "u", "û": "u", "ü": "u",
	"ú": "u", "ÿ": "y", "œ": "oe", "æ": "ae", "’": "'",
}


func _entry_icon(tab_id: String, entry_id: String, definition: Dictionary) -> Texture2D:
	if tab_id == "factions":
		var heraldry := PortraitLoader.heraldry_texture(entry_id)
		if heraldry != null:
			return heraldry
	var library := _icons()
	if library == null:
		return null
	match tab_id:
		"traits":
			return library.call("get_icon", "trait_category_" + str(definition.get("category", "")), "trait")
		"skills":
			return library.call("get_icon", "branch_" + str(definition.get("branch", "")), "branch")
		"factions":
			return library.call("get_icon", "hud_diplomacy")
		"religions":
			return library.call("get_icon", "bld_parish_church")
		"mechanics":
			return library.call("get_icon", str(definition.get("icon", "")))
	return library.call("get_icon", entry_id)


## Construit les listes de tous les onglets depuis `data/`.
func build_entries() -> void:
	_entries = {}
	for tab in TABS:
		var tab_id := str(tab["id"])
		var rows: Array = []
		var definitions: Dictionary = {}
		if str(tab["dir"]) == "":
			for mechanic in MECHANICS:
				definitions[mechanic["id"]] = mechanic
		else:
			definitions = GameCatalog.definitions(str(tab["dir"]))
		for entry_id in definitions:
			if tab_id == "factions" and str(entry_id) == "fac_rebels":
				continue
			var definition: Dictionary = definitions[entry_id]
			var display := name_of(str(entry_id))
			if tab_id == "factions" and bool(definition.get("playable", false)):
				display += " ★"
			rows.append({
				"id": str(entry_id), "name": display,
				"icon": _entry_icon(tab_id, str(entry_id), definition),
				"search": fold("%s %s %s" % [display, entry_id, str(definition.get("description", definition.get("text", "")))]),
			})
		if tab_id != "mechanics":
			rows.sort_custom(func(a: Dictionary, b: Dictionary) -> bool: return fold(a["name"]) < fold(b["name"]))
		_entries[tab_id] = rows


func tab_ids() -> PackedStringArray:
	var ids := PackedStringArray()
	for tab in TABS:
		ids.append(str(tab["id"]))
	return ids


## Nombre d'entrées de l'onglet (toutes, sans filtre).
func total_count(tab_id: String) -> int:
	return (_entries.get(tab_id, []) as Array).size()


## Nombre d'entrées affichées (après filtre) dans l'onglet courant.
func entry_count() -> int:
	return _visible_ids.size()


func visible_ids() -> PackedStringArray:
	return _visible_ids


func select_tab(index: int) -> void:
	if index < 0 or index >= TABS.size():
		return
	current_tab = index
	if tab_bar.current_tab != index:
		tab_bar.set_block_signals(true)
		tab_bar.current_tab = index
		tab_bar.set_block_signals(false)
	_fill_list()
	if not _visible_ids.is_empty() and (current_entry == "" or tab_of(current_entry) != index):
		open_entry(_visible_ids[0], false)


func set_query(text: String) -> void:
	query = text
	if search_field.text != text:
		search_field.text = text
	_fill_list()


func _fill_list() -> void:
	entry_list.clear()
	_visible_ids = PackedStringArray()
	var tab_id := str(TABS[current_tab]["id"])
	var needle := fold(query.strip_edges())
	for row in _entries.get(tab_id, []):
		if needle != "" and not str(row["search"]).contains(needle):
			continue
		var index := entry_list.add_item(str(row["name"]), row["icon"])
		entry_list.set_item_tooltip_enabled(index, false)
		_visible_ids.append(str(row["id"]))
		if str(row["id"]) == current_entry:
			entry_list.select(index)
	var total := total_count(tab_id)
	count_label.text = "%d entrée%s" % [total, "s" if total > 1 else ""] if needle == "" else "%d / %d entrées" % [_visible_ids.size(), total]


## Ouvre la fiche de `entry_id` (bascule d'onglet si besoin). Faux si l'id est inconnu.
func open_entry(entry_id: String, remember: bool = true) -> bool:
	var tab := tab_of(entry_id)
	if tab < 0 or definition_of(entry_id).is_empty():
		return false
	if remember and current_entry != "" and current_entry != entry_id:
		_history.append(current_entry)
	current_entry = entry_id
	if tab != current_tab:
		current_tab = tab
		tab_bar.set_block_signals(true)
		tab_bar.current_tab = tab
		tab_bar.set_block_signals(false)
		_fill_list()
	else:
		var position := _visible_ids.find(entry_id)
		if position >= 0:
			entry_list.select(position)
			entry_list.ensure_current_is_visible()
	_render_fiche(entry_id)
	return true


func go_back() -> void:
	if _history.is_empty():
		return
	var previous := _history[_history.size() - 1]
	_history.remove_at(_history.size() - 1)
	open_entry(previous, false)


func _render_fiche(entry_id: String) -> void:
	fiche.clear()
	var illustration := PortraitLoader.load_texture(ILLUSTRATIONS_DIR + entry_id + ".jpg")
	if illustration != null:
		fiche.add_image(illustration, ILLUSTRATION_WIDTH, 0)
		fiche.append_text("\n")
	if entry_id.begins_with("fac_"):
		var heraldry := PortraitLoader.heraldry_texture(entry_id)
		if heraldry != null:
			fiche.add_image(heraldry, 72, 0)
			fiche.append_text("  ")
	fiche.append_text(fiche_bbcode(entry_id))
	fiche.scroll_to_line(0)
	codex_button.visible = codex_entry_of(entry_id) != ""


## H11 : fiche Codex liée à une entité (`CodexStore.entry_for_entity`), vide sinon.
static func codex_entry_of(entity_id: String) -> String:
	var loop := Engine.get_main_loop() as SceneTree
	var store: Node = loop.root.get_node_or_null("/root/CodexStore") if loop != null else null
	return str(store.call("entry_for_entity", entity_id)) if store != null and entity_id != "" else ""


func open_codex_entry() -> void:
	var codex_id := codex_entry_of(current_entry)
	var bubbles := get_node_or_null("/root/CodexBubbles")
	if codex_id != "" and bubbles != null:
		bubbles.call("open_entry", codex_id)


# --- Fiches ---------------------------------------------------------------------------


static func icon_bbcode(id: String, size: int = 16, category: String = "") -> String:
	return RichTooltip.icon_bbcode(id, size, category)


## Lien interne vers une autre fiche : « [icône] Nom » cliquable.
static func link(entry_id: String, with_icon: bool = true) -> String:
	if entry_id == "":
		return ""
	var name := name_of(entry_id)
	var known := not definition_of(entry_id).is_empty()
	var icon := icon_bbcode(entry_id, 16) + " " if with_icon and (entry_id.begins_with("unit_") or entry_id.begins_with("bld_") or entry_id.begins_with("tech_") or entry_id.begins_with("res_")) else ""
	if not known:
		return icon + name
	return "%s[url=%s][color=%s]%s[/color][/url]" % [icon, entry_id, LINK, name]


static func _links(ids: Array) -> String:
	var parts := PackedStringArray()
	for id in ids:
		parts.append(link(str(id)))
	return ", ".join(parts)


static func _heading(entry_id: String, name: String, subtitle: String, category: String = "") -> String:
	var icon := icon_bbcode(entry_id, 40, category) if entry_id != "" else ""
	var head := "[font_size=24][b]%s[/b][/font_size]" % name
	if icon != "":
		head = icon + " " + head
	if subtitle != "":
		head += "\n[color=%s][i]%s[/i][/color]" % [MUTED, subtitle]
	return head


static func _section(title: String, body: String) -> String:
	if body.strip_edges() == "":
		return ""
	return "[b]%s[/b]\n%s" % [title, body]


static func _description(definition: Dictionary) -> String:
	var text := str(definition.get("description", ""))
	return "[i]%s[/i]" % text if text != "" else ""


static func _sources(definition: Dictionary) -> String:
	var sources: Array = definition.get("sources", [])
	if sources.is_empty():
		return ""
	var parts := PackedStringArray()
	for source in sources:
		parts.append(str(source))
	return "[font_size=12][color=%s]Sources : %s[/color][/font_size]" % [MUTED, " ; ".join(parts)]


static func _join(parts: Array) -> String:
	var kept := PackedStringArray()
	for part in parts:
		if str(part).strip_edges() != "":
			kept.append(str(part))
	return "\n\n".join(kept)


static func _effects(effects: Array) -> String:
	var lines := PackedStringArray()
	for effect in effects:
		if effect is Dictionary:
			lines.append("• " + RichTooltip.effect_text(effect))
	return "\n".join(lines)


static func _date_value(value: Variant) -> String:
	if value is Dictionary:
		var text := str(value.get("value", ""))
		if bool(value.get("uncertain", false)):
			text = "vers " + text
		return text
	return str(value) if value != null else ""


## BBCode de la fiche de `entry_id` (vide si inconnu).
static func fiche_bbcode(entry_id: String) -> String:
	var definition := definition_of(entry_id)
	if definition.is_empty():
		return ""
	match str(TABS[tab_of(entry_id)]["id"]):
		"units":
			return _unit_fiche(entry_id, definition)
		"buildings":
			return _building_fiche(entry_id, definition)
		"technologies":
			return _technology_fiche(entry_id, definition)
		"resources":
			return _resource_fiche(entry_id, definition)
		"traits":
			return _trait_fiche(entry_id, definition)
		"skills":
			return _skill_fiche(entry_id, definition)
		"factions":
			return _faction_fiche(entry_id, definition)
		"religions":
			return _religion_fiche(entry_id, definition)
		"mechanics":
			return _mechanic_fiche(definition)
	return ""


## Identifiants d'un dossier dont le champ `key` (texte ou liste) contient `entry_id`.
static func _referencing(directory: String, key: String, entry_id: String) -> Array:
	var result: Array = []
	for id in GameCatalog.definitions(directory):
		var value: Variant = GameCatalog.definitions(directory)[id].get(key, null)
		if (value is String and value == entry_id) or (value is Array and entry_id in value):
			result.append(str(id))
	result.sort()
	return result


static func _tech_unlocking(kind: String, entry_id: String) -> Array:
	var result: Array = []
	var technologies := GameCatalog.definitions("technologies")
	for id in technologies:
		var unlocks: Dictionary = technologies[id].get("unlocks", {})
		if entry_id in unlocks.get(kind, []):
			result.append(str(id))
	result.sort()
	return result


static func _unit_fiche(entry_id: String, definition: Dictionary) -> String:
	var category := str(definition.get("category", ""))
	var subtitle := str(RichTooltip.UNIT_CATEGORY_LABELS.get(category, category))
	if definition.has("soldiers"):
		subtitle += ", %d hommes" % int(definition["soldiers"])
	var name_block: Variant = definition.get("name", {})
	if name_block is Dictionary and str(name_block.get("local", "")) != "":
		subtitle += " — « %s » (%s)" % [name_block["local"], name_block.get("local_language", "")]
	var costs := PackedStringArray()
	if definition.has("cost"):
		costs.append("Coût : " + RichTooltip.cost_text(definition["cost"]))
	if definition.has("upkeep"):
		costs.append("Entretien : %d %s / saison" % [int(definition["upkeep"]), RichTooltip.POUND])
	if definition.has("recruit_time_turns"):
		costs.append("Levée : %d tour(s)" % int(definition["recruit_time_turns"]))
	var stats: Dictionary = definition.get("stats", {})
	var stat_lines := PackedStringArray()
	for stat in ["melee", "ranged", "range", "armor", "morale", "speed", "charge", "siege_attack", "ammo"]:
		if stats.has(stat):
			stat_lines.append("%s %s" % [RichTooltip.STAT_LABELS[stat], RichTooltip._number(float(stats[stat]))])
	var sw := RichTooltip.strengths_weaknesses(definition)
	var traits := PackedStringArray()
	if not (sw[0] as PackedStringArray).is_empty():
		traits.append("[color=%s]Forces : %s[/color]" % [RichTooltip.GREEN, ", ".join(sw[0])])
	if not (sw[1] as PackedStringArray).is_empty():
		traits.append("[color=%s]Faiblesses : %s[/color]" % [RichTooltip.RED, ", ".join(sw[1])])
	var abilities := PackedStringArray()
	for ability in definition.get("abilities", []):
		abilities.append(str(RichTooltip.ABILITY_LABELS.get(ability, ability)))
	if not abilities.is_empty():
		traits.append("Capacités : " + ", ".join(abilities))
	var requires := PackedStringArray()
	if str(definition.get("required_technology", "")) != "":
		requires.append("Technologie : " + link(str(definition["required_technology"])))
	if str(definition.get("required_building", "")) != "":
		requires.append("Bâtiment : " + link(str(definition["required_building"])))
	var enabling := _referencing("buildings", "enables_units", entry_id)
	if not enabling.is_empty():
		requires.append("Levée dans : " + _links(enabling))
	var unlocking := _tech_unlocking("units", entry_id)
	if not unlocking.is_empty() and str(definition.get("required_technology", "")) == "":
		requires.append("Débloquée par : " + _links(unlocking))
	if str(definition.get("source_class", "")) != "":
		requires.append("Recrutés parmi : %s" % str(RichTooltip.CLASS_LABELS.get(definition["source_class"], definition["source_class"])).to_lower())
	var cultures: Array = definition.get("required_culture", [])
	if not cultures.is_empty():
		var names := PackedStringArray()
		for culture in cultures:
			names.append(str(culture).trim_prefix("cul_").replace("_", " "))
		requires.append("Cultures : " + ", ".join(names))
	return _join([
		_heading(entry_id, name_of(entry_id), subtitle, "unit"), _description(definition),
		" · ".join(costs), _section("Caractéristiques", " · ".join(stat_lines)), "\n".join(traits),
		_section("Conditions", "\n".join(requires)),
		_section("Équipement", str(definition.get("equipment", ""))), _sources(definition),
	])


static func _building_fiche(entry_id: String, definition: Dictionary) -> String:
	var category := str(definition.get("category", ""))
	var subtitle := str(RichTooltip.BUILDING_CATEGORY_LABELS.get(category, category))
	if definition.has("tier"):
		subtitle += ", rang %d" % int(definition["tier"])
	var costs := PackedStringArray()
	if definition.has("cost"):
		costs.append("Coût : " + RichTooltip.cost_text(definition["cost"]))
	if definition.has("build_time_turns"):
		costs.append("Durée : %d tour(s)" % int(definition["build_time_turns"]))
	costs.append("Entretien : %d %s / saison" % [int(definition.get("upkeep", 0)), RichTooltip.POUND])
	var requires := PackedStringArray()
	if str(definition.get("upgrades_from", "")) != "":
		requires.append("Amélioration de : " + link(str(definition["upgrades_from"])))
	for key in ["required_building", "required_technology", "required_resource"]:
		if str(definition.get(key, "")) != "":
			requires.append(link(str(definition[key])))
	if bool(definition.get("requires_coastal", false)):
		requires.append("Province côtière")
	if bool(definition.get("requires_river", false)):
		requires.append("Rivière")
	var unlocking := _tech_unlocking("buildings", entry_id)
	if not unlocking.is_empty():
		requires.append("Débloqué par : " + _links(unlocking))
	var leads := PackedStringArray()
	var upgrades := _referencing("buildings", "upgrades_from", entry_id)
	if not upgrades.is_empty():
		leads.append("Améliorable en : " + _links(upgrades))
	var enables: Array = definition.get("enables_units", [])
	if not enables.is_empty():
		leads.append("Permet de lever : " + _links(enables))
	return _join([
		_heading(entry_id, name_of(entry_id), subtitle, "building"), _description(definition),
		" · ".join(costs), _section("Effets", _effects(definition.get("effects", []))),
		_section("Prérequis", "\n".join(requires)), "\n".join(leads), _sources(definition),
	])


static func _technology_fiche(entry_id: String, definition: Dictionary) -> String:
	var branch := str(definition.get("branch", ""))
	var subtitle := "%s, rang %d" % ["Militaire" if branch == "military" else "Civile", int(definition.get("tier", 1))]
	var lines := PackedStringArray(["Coût : %d points de recherche" % int(definition.get("cost", 0))])
	var year: Variant = definition.get("historical_year", null)
	if year != null:
		var line := "Date historique : " + _date_value(year)
		if year is Dictionary and str(year.get("note", "")) != "":
			line += " — [i]%s[/i]" % year["note"]
		lines.append(line)
	var prerequisites: Array = definition.get("prerequisites", [])
	var unlocks: Dictionary = definition.get("unlocks", {})
	var unlocked: Array = []
	unlocked.append_array(unlocks.get("units", []))
	unlocked.append_array(unlocks.get("buildings", []))
	var required_by := _referencing("technologies", "prerequisites", entry_id)
	var starting := _referencing("factions", "starting_technologies", entry_id)
	var starting_names := PackedStringArray()
	for faction_id in starting:
		starting_names.append(link(faction_id))
	return _join([
		_heading(entry_id, name_of(entry_id), subtitle, "technology"), _description(definition),
		"\n".join(lines), _section("Effets", _effects(definition.get("effects", []))),
		_section("Prérequis", _links(prerequisites) if not prerequisites.is_empty() else "Aucun"),
		_section("Débloque", _links(unlocked)), _section("Ouvre la voie à", _links(required_by)),
		_section("Connue en 1337 par", ", ".join(starting_names)), _sources(definition),
	])


static func _resource_fiche(entry_id: String, definition: Dictionary) -> String:
	var category := str(definition.get("category", ""))
	var lines := PackedStringArray()
	if definition.has("base_price"):
		lines.append("Prix de base : %s %s" % [RichTooltip._number(float(definition["base_price"])), RichTooltip.POUND])
	var classes := PackedStringArray()
	for class_id in definition.get("satisfies_classes", []):
		classes.append("%s %s" % [icon_bbcode("class_" + str(class_id), 16), str(RichTooltip.CLASS_LABELS.get(class_id, class_id))])
	if not classes.is_empty():
		lines.append("Satisfait : " + ", ".join(classes))
	var used_by: Array = _referencing("buildings", "required_resource", entry_id)
	for id in GameCatalog.definitions("buildings"):
		var cost: Variant = GameCatalog.definitions("buildings")[id].get("cost", {})
		if cost is Dictionary and (cost.get("resources", {}) as Dictionary).has(entry_id) and not used_by.has(id):
			used_by.append(str(id))
	used_by.sort()
	return _join([
		_heading(entry_id, name_of(entry_id), str(RichTooltip.RESOURCE_CATEGORY_LABELS.get(category, category)), "resource"),
		_description(definition), "\n".join(lines),
		_section("Utilisée par les bâtiments", _links(used_by)), _sources(definition),
	])


static func _trait_fiche(entry_id: String, definition: Dictionary) -> String:
	var category := str(definition.get("category", ""))
	var opposites: Array = definition.get("opposites", [])
	return _join([
		_heading("trait_category_" + category, name_of(entry_id), "Trait " + str(RichTooltip.TRAIT_CATEGORY_LABELS.get(category, category)), "trait"),
		_description(definition), _section("Effets", _effects(definition.get("effects", []))),
		_section("Incompatible avec", _links(opposites)),
	])


static func _skill_fiche(entry_id: String, definition: Dictionary) -> String:
	var branch := str(definition.get("branch", ""))
	var subtitle := "%s, rang %d — %d point(s) de compétence" % [RichTooltip.BRANCH_LABELS.get(branch, branch), int(definition.get("tier", 1)), int(definition.get("cost", 1))]
	var prerequisites: Array = definition.get("prerequisites", [])
	return _join([
		_heading("branch_" + branch, name_of(entry_id), subtitle, "branch"), _description(definition),
		_section("Effets", _effects(definition.get("effects", []))),
		_section("Prérequis", _links(prerequisites) if not prerequisites.is_empty() else "Aucun"),
		_section("Ouvre", _links(_referencing("skills", "prerequisites", entry_id))),
		str(RichTooltip.BRANCH_TEXTS.get(branch, "")),
	])


static func _character_name(character_id: String) -> String:
	var character: Dictionary = GameCatalog.definitions("characters").get(character_id, {})
	var name: Variant = character.get("name", null)
	if name is Dictionary:
		return str(name.get("display", character_id))
	return character_id.trim_prefix("chr_").replace("_", " ").capitalize()


static func _province_name(province_id: String) -> String:
	var loop := Engine.get_main_loop() as SceneTree
	var facade: Node = loop.root.get_node_or_null("/root/SimFacade") if loop != null else null
	if facade != null and bool(facade.call("store_loaded")):
		var info: Dictionary = facade.get("store").call("get_province", province_id)
		if not info.is_empty():
			return str(info.get("display_name", province_id))
	return province_id.trim_prefix("prov_").replace("_", " ").capitalize()


static func _faction_fiche(entry_id: String, definition: Dictionary) -> String:
	var government := str(definition.get("government", ""))
	var subtitle := str(GOVERNMENT_LABELS.get(government, government)).capitalize()
	if bool(definition.get("playable", false)):
		subtitle += " — faction jouable"
	var lines := PackedStringArray()
	if str(definition.get("ruler", "")) != "":
		lines.append("Dirigeant en 1337 : [b]%s[/b]" % _character_name(str(definition["ruler"])))
	if str(definition.get("heir", "")) != "":
		lines.append("Héritier : %s" % _character_name(str(definition["heir"])))
	var capital := str(definition.get("capital_city", ""))
	if str(definition.get("capital", "")) != "":
		lines.append("Capitale : %s (%s)" % [capital if capital != "" else "—", _province_name(str(definition["capital"]))])
	if str(definition.get("religion", "")) != "":
		lines.append("Religion : " + link(str(definition["religion"])))
	var law := str(definition.get("succession_law", ""))
	if law != "":
		lines.append("Succession : " + str(SUCCESSION_LABELS.get(law, law.replace("_", " "))))
	if definition.has("treasury"):
		lines.append("Trésor en 1337 : %s %s · prestige %d" % [RichTooltip.thousands(int(definition["treasury"])), RichTooltip.POUND, int(definition.get("prestige", 0))])
	var titles: Array = definition.get("titles", [])
	if not titles.is_empty():
		lines.append("Titres : " + ", ".join(PackedStringArray(titles)))
	var heraldry: Dictionary = definition.get("heraldry", {})
	var arms := ""
	if str(heraldry.get("blazon", "")) != "":
		arms = "[i]%s[/i]" % heraldry["blazon"]
		if str(heraldry.get("description", "")) != "":
			arms += "\n" + str(heraldry["description"])
	var objectives := ""
	var victory: Dictionary = definition.get("victory", {})
	if not victory.is_empty():
		var parts := PackedStringArray()
		if str(victory.get("summary", "")) != "":
			parts.append("[i]%s[/i]" % victory["summary"])
		for objective in victory.get("objectives", []):
			parts.append("• [b]%s[/b] — %s" % [objective.get("title", ""), objective.get("description", "")])
		if victory.has("end_year"):
			parts.append("Échéance : %d" % int(victory["end_year"]))
		objectives = "\n".join(parts)
	var relations := PackedStringArray()
	for relation in definition.get("relations", []):
		if relation is Dictionary:
			relations.append("• %s : %s" % [link(str(relation.get("faction", ""))), RELATION_LABELS.get(str(relation.get("status", "")), str(relation.get("status", "")))])
	var starting: Array = definition.get("starting_technologies", [])
	return _join([
		_heading("", name_of(entry_id), subtitle), _description(definition), "\n".join(lines),
		_section("Armoiries", arms), _section("Objectifs historiques", objectives),
		_section("Relations en 1337", "\n".join(relations)),
		_section("Technologies connues", _links(starting)), _sources(definition),
	])


const RELATION_LABELS := {
	"war": "guerre", "alliance": "alliance", "truce": "trêve", "overlord": "suzerain de",
	"vassal": "vassal de", "marriage_tie": "liens matrimoniaux", "peace": "paix",
	"embargo": "embargo",
}


static func _religion_fiche(entry_id: String, definition: Dictionary) -> String:
	var kind := str(definition.get("kind", ""))
	var lines := PackedStringArray()
	if str(definition.get("head_faction", "")) != "":
		lines.append("Chef : " + link(str(definition["head_faction"])))
	if str(definition.get("seat", "")) != "":
		lines.append("Siège : " + str(definition["seat"]))
	var adherents: Array = definition.get("historical_adherents", [])
	var followers := _referencing("factions", "religion", entry_id)
	for faction_id in followers:
		if not adherents.has(faction_id):
			adherents.append(faction_id)
	return _join([
		_heading("bld_parish_church", name_of(entry_id), str(RELIGION_KIND_LABELS.get(kind, kind)).capitalize(), "building"),
		_description(definition), "\n".join(lines), _section("Effets", _effects(definition.get("effects", []))),
		_section("Fidèles", _links(adherents)), _sources(definition),
	])


static func _mechanic_fiche(definition: Dictionary) -> String:
	return _join([
		_heading(str(definition.get("icon", "")), str(definition.get("name", "")), "Mécanique de jeu", "hud"),
		str(definition.get("text", "")), _mechanic_extra(str(definition.get("extra", ""))),
	])


## Compléments tirés des données pour certaines mécaniques.
static func _mechanic_extra(kind: String) -> String:
	var factions := GameCatalog.definitions("factions")
	var playable: Array = []
	for id in factions:
		if bool(factions[id].get("playable", false)):
			playable.append(str(id))
	playable.sort()
	match kind:
		"victory":
			var parts := PackedStringArray()
			for id in playable:
				var victory: Dictionary = factions[id].get("victory", {})
				var titles := PackedStringArray()
				for objective in victory.get("objectives", []):
					titles.append(str(objective.get("title", "")))
				parts.append("• %s (échéance %d) : %s" % [link(id), int(victory.get("end_year", 0)), ", ".join(titles)])
			return _section("Objectifs des factions jouables", "\n".join(parts))
		"succession":
			var parts := PackedStringArray()
			for id in playable:
				var law := str(factions[id].get("succession_law", ""))
				parts.append("• %s : %s" % [link(id), SUCCESSION_LABELS.get(law, law)])
			return _section("Lois de succession", "\n".join(parts))
		"classes":
			var parts := PackedStringArray()
			for class_id in ["peasants", "burghers", "clergy", "nobility"]:
				var goods := PackedStringArray()
				var resources := GameCatalog.definitions("resources")
				for res_id in resources:
					if class_id in resources[res_id].get("satisfies_classes", []):
						goods.append(link(str(res_id)))
				parts.append("• %s %s : %s" % [icon_bbcode("class_" + class_id, 16), RichTooltip.CLASS_LABELS[class_id], ", ".join(goods)])
			return _section("Biens recherchés par classe", "\n".join(parts))
		"relations":
			var parts := PackedStringArray()
			for id in playable:
				var wars := PackedStringArray()
				for relation in factions[id].get("relations", []):
					if relation is Dictionary and str(relation.get("status", "")) == "war":
						wars.append(link(str(relation.get("faction", ""))))
				if not wars.is_empty():
					parts.append("• %s en guerre contre %s" % [link(id), ", ".join(wars)])
			return _section("Guerres en 1337", "\n".join(parts))
	return ""
