class_name CodexHub
extends PanelContainer

## Lot U11 (audit A3, D4 et D5) : une seule fenêtre « Codex » sur la carte, à deux onglets —
## « Histoire » (le Codex, `CodexWindow`, touche K) et « Règles » (l'encyclopédie de jeu,
## `Encyclopedia`, touche L) — avec une recherche commune et des onglets au style parchemin.
## Panneau central de la pile de `MapUI`. Les deux vues gardent leur API : ouvrir l'une (bulle,
## lien, raccourci) affiche la fenêtre sur son onglet ; les fermer toutes deux ferme la fenêtre.
## Aucune règle de jeu.

const TAB_HISTORY := 0
const TAB_RULES := 1
## P2c : taille minimale sous `UiZones.Zone.MODAL` (centrée par `UiZones`, marge conservée à
## 1280×720 — l'ancienne taille 1240×720 touchait les bords sans marge).
const SIZE := Vector2(1180, 620)

var tabs: TabBar
var search: LineEdit
var body: Control
var codex_window: CodexWindow
var encyclopedia: Encyclopedia
var _syncing := false


func _init() -> void:
	name = "CodexHub"
	add_to_group("codex_hub")


func _ready() -> void:
	theme = load("res://scenes/ui/parchment_theme.tres")
	mouse_filter = Control.MOUSE_FILTER_STOP
	custom_minimum_size = SIZE
	add_theme_stylebox_override("panel", HudStyle.panel_box(12))
	var box := VBoxContainer.new()
	box.add_theme_constant_override("separation", 6)
	add_child(box)
	var header := HBoxContainer.new()
	header.add_theme_constant_override("separation", 10)
	box.add_child(header)
	var title := Label.new()
	title.text = "✠ Codex"
	UiType.apply(title, UiType.TITLE)
	title.add_theme_color_override("font_color", HudStyle.RUBRIC)
	header.add_child(title)
	tabs = TabBar.new()
	tabs.name = "HubTabs"
	tabs.add_tab("Histoire")
	tabs.add_tab("Règles")
	tabs.set_tab_tooltip(TAB_HISTORY, "Personnages, lieux, batailles, vie du temps (touche K)")
	tabs.set_tab_tooltip(TAB_RULES, "Unités, bâtiments, technologies, mécaniques du jeu (touche L)")
	tabs.size_flags_vertical = Control.SIZE_SHRINK_CENTER
	style_tabs(tabs)
	tabs.tab_changed.connect(_on_tab_changed)
	header.add_child(tabs)
	var spacer := Control.new()
	spacer.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	header.add_child(spacer)
	search = LineEdit.new()
	search.name = "HubSearch"
	search.placeholder_text = "Rechercher dans l'histoire et les règles…"
	search.clear_button_enabled = true
	search.custom_minimum_size = Vector2(340, 0)
	# D5 : texte d'aide lisible (encre passée, pas clair sur clair).
	search.add_theme_color_override("font_placeholder_color", HudStyle.INK_SOFT)
	search.add_theme_color_override("font_color", HudStyle.INK)
	search.text_changed.connect(_on_search)
	header.add_child(search)
	var close := Button.new()
	close.text = "×"
	RichTooltip.attach_plain(close, "close_escape")
	close.pressed.connect(hide)
	header.add_child(close)
	body = VBoxContainer.new()
	body.size_flags_vertical = Control.SIZE_EXPAND_FILL
	box.add_child(body)
	codex_window = CodexWindow.new()
	codex_window.name = "CodexWindow"
	codex_window.embedded = true
	codex_window.hide()
	body.add_child(codex_window)
	codex_window.visibility_changed.connect(_on_view_visibility.bind(codex_window))
	visibility_changed.connect(_on_hub_visibility)
	hide()
	# P2c : fenêtre commune dans `UiZones.Zone.MODAL` (fond assombri, centrée sur sa taille
	# propre). En différé : `_ready` tourne encore dans la pile de `map_ui.add_child(codex_hub)`,
	# et un reparentage immédiat lèverait « parent busy setting up children ».
	call_deferred("_join_modal_zone")


func _join_modal_zone() -> void:
	if is_inside_tree():
		UiZones.put(UiZones.Zone.MODAL, self)


## Style parchemin d'une barre d'onglets (D5 : plus d'onglets gris foncé hors du thème).
static func style_tabs(bar: TabBar, variation: String = UiType.BODY, padding: int = 14) -> void:
	bar.clip_tabs = false
	var selected := HudStyle.card_box(HudStyle.PARCHMENT_LIGHT, HudStyle.RUBRIC, 2)
	selected.set_content_margin_all(5)
	selected.content_margin_left = padding
	selected.content_margin_right = padding
	var unselected := HudStyle.card_box(HudStyle.PARCHMENT_DARK, HudStyle.INK_SOFT, 1)
	unselected.set_content_margin_all(5)
	unselected.content_margin_left = padding
	unselected.content_margin_right = padding
	var hovered := unselected.duplicate() as StyleBoxFlat
	hovered.bg_color = HudStyle.PARCHMENT
	bar.add_theme_stylebox_override("tab_selected", selected)
	bar.add_theme_stylebox_override("tab_unselected", unselected)
	bar.add_theme_stylebox_override("tab_hovered", hovered)
	bar.add_theme_stylebox_override("tab_focus", StyleBoxEmpty.new())
	bar.add_theme_color_override("font_selected_color", HudStyle.RUBRIC)
	bar.add_theme_color_override("font_unselected_color", HudStyle.INK_SOFT)
	bar.add_theme_color_override("font_hovered_color", HudStyle.INK)
	UiType.apply(bar, variation)


## Accueille l'encyclopédie de la carte (créée par le tutoriel) dans l'onglet « Règles ».
func adopt_encyclopedia(view: Encyclopedia) -> void:
	if encyclopedia == view or view == null:
		return
	encyclopedia = view
	var was_visible := view.visible
	if view.get_parent() != null:
		view.get_parent().remove_child(view)
	view.set_embedded(true)
	view.hide()
	body.add_child(view)
	view.visibility_changed.connect(_on_view_visibility.bind(view))
	if was_visible:
		view.show()


## Onglet courant (`TAB_HISTORY` ou `TAB_RULES`).
func current_tab() -> int:
	return tabs.current_tab


## Ouvre la fenêtre sur l'onglet `tab` (fiche en cours de la vue).
func open_tab(tab: int) -> void:
	if tab == TAB_RULES and encyclopedia != null:
		encyclopedia.open_window()
	else:
		codex_window.open()


func toggle_tab(tab: int) -> void:
	if visible and tabs.current_tab == tab:
		hide()
	else:
		open_tab(tab)


func _on_tab_changed(tab: int) -> void:
	if _syncing:
		return
	open_tab(tab)


## Une vue s'affiche : l'autre se masque, l'onglet suit, la fenêtre s'ouvre ; toutes deux
## masquées : la fenêtre se ferme.
func _on_view_visibility(view: Control) -> void:
	if _syncing:
		return
	_syncing = true
	if view.visible:
		var other: Control = encyclopedia if view == codex_window else codex_window
		if other != null and other.visible:
			other.hide()
		tabs.current_tab = TAB_HISTORY if view == codex_window else TAB_RULES
		if not visible:
			show()
		_apply_search_to(view)
	elif not codex_window.visible and (encyclopedia == null or not encyclopedia.visible):
		hide()
	_syncing = false


func _on_hub_visibility() -> void:
	if visible or _syncing:
		return
	_syncing = true
	codex_window.hide()
	if encyclopedia != null and encyclopedia.visible:
		encyclopedia.close_window()
	_syncing = false


## Recherche commune : filtre les deux listes ; le nombre de résultats s'affiche dans les onglets.
func _on_search(text: String) -> void:
	codex_window.set_query(text)
	if encyclopedia != null:
		encyclopedia.set_query(text)
	var query := text.strip_edges()
	tabs.set_tab_title(TAB_HISTORY, "Histoire" if query == "" else "Histoire (%d)" % codex_window.match_count())
	if encyclopedia != null:
		tabs.set_tab_title(TAB_RULES, "Règles" if query == "" else "Règles (%d)" % encyclopedia.match_count())


func _apply_search_to(view: Control) -> void:
	if search == null or search.text == "":
		return
	if view == codex_window:
		codex_window.set_query(search.text)
	elif view == encyclopedia:
		encyclopedia.set_query(search.text)
